use crate::sema::TypeStack;
use crate::*;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone)]

pub struct Error {
    pub function: String,
    pub block: Option<LabelRef>,
    pub instruction: Option<usize>,
    pub message: String,
}

impl Error {
    fn new(
        function: &Function,
        block: Option<LabelRef>,
        instruction: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            function: function.name.clone(),
            block,
            instruction,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in '{}'", self.function)?;

        if let Some(label) = self.block {
            write!(f, " at .L{}", label)?;
        }

        if let Some(index) = self.instruction {
            write!(f, " instruction {}", index)?;
        }

        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for Error {}

type Stack = Vec<Type>;

struct FunctionVerifier<'a> {
    module: &'a Module,
    function: &'a Function,
    blocks: HashMap<LabelRef, &'a BasicBlock>,
    entry_stacks: HashMap<LabelRef, Stack>,
    reachable: HashSet<LabelRef>,
    errors: Vec<Error>,
}

impl<'a> FunctionVerifier<'a> {
    fn error(&self, label: LabelRef, index: usize, message: impl Into<String>) -> Error {
        Error::new(self.function, Some(label), Some(index), message)
    }

    fn new(module: &'a Module, function: &'a Function) -> Self {
        let mut blocks = HashMap::new();
        let mut errors = Vec::new();

        for block in &function.blocks {
            if blocks.insert(block.label, block).is_some() {
                errors.push(Error::new(
                    function,
                    Some(block.label),
                    None,
                    "duplicate basic block label",
                ));
            }
        }

        if function.blocks.is_empty() {
            errors.push(Error::new(
                function,
                None,
                None,
                "function has no basic blocks",
            ));
        }

        Self {
            module,
            function,
            blocks,
            entry_stacks: HashMap::new(),
            reachable: HashSet::new(),
            errors,
        }
    }

    fn verify(mut self) -> Vec<Error> {
        log::debug!("verifying function '{}'", self.function.name);

        if self.function.blocks.is_empty() {
            return self.errors;
        }

        let entry = self.function.blocks[0].label;
        self.visit_block(entry, Vec::new());

        log::debug!(
            "verified function '{}', reachable blocks: {:?}",
            self.function.name,
            self.reachable,
        );

        if !self.errors.is_empty() {
            log::error!(
                "verification failed for function '{}', {} errors",
                self.function.name,
                self.errors.len()
            );
        }

        for e in &self.errors {
            match (e.block, e.instruction) {
                (Some(block), Some(instruction)) => {
                    log::error!("  in [L{}, I{}]: {}", block, instruction, e.message);
                }
                (Some(block), None) => {
                    log::error!("  in [L{}]: {}", block, e.message);
                }
                (None, Some(instruction)) => {
                    log::error!("  at instruction {}: {}", instruction, e.message);
                }
                (None, None) => {
                    log::error!("  {}", e.message);
                }
            }
        }

        self.errors
    }

    fn visit_block(&mut self, label: LabelRef, incoming: Stack) {
        let Some(block) = self.blocks.get(&label).copied() else {
            self.errors.push(Error::new(
                self.function,
                Some(label),
                None,
                "branch references unknown basic block",
            ));
            return;
        };

        /*
         * If this block was already reached, its stack state must
         * be identical to the state we have now.
         */

        if let Some(existing) = self.entry_stacks.get(&label) {
            if existing != &incoming {
                self.errors.push(Error::new(
                    self.function,
                    Some(label),
                    None,
                    format!(
                        "incompatible incoming stack states: {:?} vs {:?}",
                        existing, incoming
                    ),
                ));
            }

            return;
        }

        self.entry_stacks.insert(label, incoming.clone());

        self.reachable.insert(label);

        let mut stack = incoming;

        let mut terminated = false;

        for (index, inst) in block.instructions.iter().enumerate() {
            if terminated {
                self.errors.push(Error::new(
                    self.function,
                    Some(label),
                    Some(index),
                    "instruction appears after a terminator",
                ));
                continue;
            }

            match self.transfer(inst, &mut stack, label, index) {
                Ok(ControlFlow::Continue) => {}

                Ok(ControlFlow::Goto(target)) => {
                    self.visit_block(target, stack.clone());
                    terminated = true;
                }

                Ok(ControlFlow::Conditional(target)) => {
                    self.visit_block(target, stack.clone());
                }

                Ok(ControlFlow::Return) => {
                    terminated = true;
                }

                Err(error) => {
                    self.errors.push(error);
                    match *inst {
                        Instruction::Goto(target) => {
                            self.visit_block(target, stack.clone());
                            terminated = true;
                        }
                        Instruction::GotoIfTrue(target) | Instruction::GotoIfFalse(target) => {
                            self.visit_block(target, stack.clone());
                        }
                        Instruction::Ret => {
                            terminated = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        /*
         * A block without a terminator falls through to the next
         * block in Function::blocks.
         */

        if !terminated {
            let index = self
                .function
                .blocks
                .iter()
                .position(|b| b.label == label)
                .unwrap();

            if let Some(next) = self.function.blocks.get(index + 1) {
                self.visit_block(next.label, stack);
            } else {
                self.errors.push(Error::new(
                    self.function,
                    Some(label),
                    block.instructions.len().checked_sub(1),
                    "basic block falls off the end of the function",
                ));
            }
        }
    }

    fn transfer(
        &self,
        inst: &Instruction,
        stack: &mut TypeStack,
        label: LabelRef,
        index: usize,
    ) -> Result<ControlFlow, Error> {
        match *inst {
            Instruction::LoadConstant(value, ty) => {
                self.verify_constant(value, ty, label, index)?;

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::LoadLocal(local) => {
                let ty = self.local_type(local)?;

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::LoadArg(argno) => {
                let ty = self.arg_type(argno)?;

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::StoreLocal(local) => {
                let expected = self.local_type(local)?;

                let actual = self.pop(stack, label, index);
                self.expect_type(actual?, expected, label, index)?;

                Ok(ControlFlow::Continue)
            }

            Instruction::Assert => {
                let ty = self.pop(stack, label, index)?;

                self.expect_type(ty, Type::Bool, label, index)?;

                Ok(ControlFlow::Continue)
            }

            Instruction::Goto(target) => Ok(ControlFlow::Goto(target)),

            Instruction::GotoIfTrue(target) | Instruction::GotoIfFalse(target) => {
                let ty = self.pop(stack, label, index)?;

                self.expect_type(ty, Type::Bool, label, index)?;

                Ok(ControlFlow::Conditional(target))
            }

            Instruction::Call(function) => {
                let callee = self.function_ref(function, label, index)?;

                /*
                 * Arguments are represented on the operand stack in
                 * declaration order:
                 *
                 *   [arg0, arg1, ... argN]
                 *
                 * Therefore we pop them in reverse order.
                 */

                for expected in callee.params.iter().rev() {
                    let actual = self.pop(stack, label, index)?;

                    self.expect_type(actual, *expected, label, index)?;
                }

                stack.push(callee.return_type);

                Ok(ControlFlow::Continue)
            }

            Instruction::Pop => {
                self.pop(stack, label, index)?;

                Ok(ControlFlow::Continue)
            }

            Instruction::Dup => {
                let ty = self.pop(stack, label, index)?;

                stack.push(ty);
                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::Swp => {
                let r = self.pop(stack, label, index)?;
                let l = self.pop(stack, label, index)?;

                stack.push(r);
                stack.push(l);

                Ok(ControlFlow::Continue)
            }

            Instruction::Over => {
                let r = self.pop(stack, label, index)?;
                let l = self.pop(stack, label, index)?;

                stack.push(l);
                stack.push(r);
                stack.push(l);

                Ok(ControlFlow::Continue)
            }

            Instruction::Nip => {
                let r = self.pop(stack, label, index)?;
                let _ = self.pop(stack, label, index)?;

                stack.push(r);

                Ok(ControlFlow::Continue)
            }

            Instruction::Tuck => {
                let r = self.pop(stack, label, index)?;
                let l = self.pop(stack, label, index)?;

                stack.push(r);
                stack.push(l);
                stack.push(r);

                Ok(ControlFlow::Continue)
            }

            Instruction::Rot => {
                let c = self.pop(stack, label, index)?;
                let b = self.pop(stack, label, index)?;
                let a = self.pop(stack, label, index)?;

                stack.push(c);
                stack.push(a);
                stack.push(b);

                Ok(ControlFlow::Continue)
            }

            Instruction::Pick(n) => {
                if n >= stack.len() {
                    return Err(self.error(
                        label,
                        index,
                        format!("pick depth {} exceeds stack size {}", n, stack.len()),
                    ));
                }

                let ty = stack[stack.len() - 1 - n];

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::Roll(n) => {
                if n >= stack.len() {
                    return Err(self.error(
                        label,
                        index,
                        format!("roll depth {} exceeds stack size {}", n, stack.len()),
                    ));
                }

                let ty = stack.remove(stack.len() - 1 - n);

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::IAdd
            | Instruction::ISub
            | Instruction::IMul
            | Instruction::IDiv
            | Instruction::UDiv
            | Instruction::IRem
            | Instruction::URem
            | Instruction::IAnd
            | Instruction::IOr
            | Instruction::IXor
            | Instruction::IShl
            | Instruction::IShr
            | Instruction::ISar => {
                let (left, right) = self.pop_binary(stack, label, index)?;

                if !left.is_integer() || left != right {
                    return Err(self.error(
                        label,
                        index,
                        format!(
                            "integer binary operation requires two identical integer types, got {:?}, {:?}",
                            left, right
                        ),
                    ));
                }

                stack.push(left);

                Ok(ControlFlow::Continue)
            }

            Instruction::IAbs | Instruction::ISqrt | Instruction::INeg | Instruction::INot => {
                let ty = self.pop(stack, label, index)?;

                if !ty.is_integer() {
                    return Err(self.error(
                        label,
                        index,
                        format!(
                            "integer unary operation requires integer operand, got {:?}",
                            ty
                        ),
                    ));
                }

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::ICmp(_) => {
                let (left, right) = self.pop_binary(stack, label, index)?;

                if !left.is_integer() || left != right {
                    return Err(self.error(
                        label,
                        index,
                        format!(
                            "integer comparison requires two identical integer types, got {:?}, {:?}",
                            left, right
                        ),
                    ));
                }

                stack.push(Type::Bool);

                Ok(ControlFlow::Continue)
            }

            Instruction::FAdd
            | Instruction::FSub
            | Instruction::FMul
            | Instruction::FDiv
            | Instruction::FRem => {
                let (left, right) = self.pop_binary(stack, label, index)?;

                if !left.is_float() || left != right {
                    return Err(self.error(
                        label,
                        index,
                        format!(
                            "float binary operation requires two identical float types, got {:?}, {:?}",
                            left, right
                        ),
                    ));
                }

                stack.push(left);

                Ok(ControlFlow::Continue)
            }

            Instruction::FAbs | Instruction::FSqrt | Instruction::FNeg => {
                let ty = self.pop(stack, label, index)?;

                if !ty.is_float() {
                    return Err(self.error(
                        label,
                        index,
                        format!("float unary operation requires float operand, got {:?}", ty),
                    ));
                }

                stack.push(ty);

                Ok(ControlFlow::Continue)
            }

            Instruction::FCmp(_) => {
                let (left, right) = self.pop_binary(stack, label, index)?;

                if !left.is_float() || left != right {
                    return Err(self.error(
                        label,
                        index,
                        format!(
                            "float comparison requires two identical float types, got {:?}, {:?}",
                            left, right
                        ),
                    ));
                }

                stack.push(Type::Bool);

                Ok(ControlFlow::Continue)
            }

            Instruction::Cast(target) => {
                let source = self.pop(stack, label, index)?;

                if !source.can_cast_to(target) {
                    return Err(self.error(
                        label,
                        index,
                        format!("invalid cast from {:?} to {:?}", source, target),
                    ));
                }

                stack.push(target);

                Ok(ControlFlow::Continue)
            }

            Instruction::Ret => {
                if self.function.return_type.is_void() {
                    if !stack.is_empty() {
                        return Err(self.error(
                            label,
                            index,
                            format!("stack must be empty after return, remaining: {:?}", stack),
                        ));
                    }

                    return Ok(ControlFlow::Return);
                }

                let result = self.pop(stack, label, index)?;

                self.expect_type(result, self.function.return_type, label, index)?;

                if !stack.is_empty() {
                    return Err(self.error(
                        label,
                        index,
                        format!("stack must be empty after return, remaining: {:?}", stack),
                    ));
                }

                Ok(ControlFlow::Return)
            }
        }
    }

    fn arg_type(&self, argno: usize) -> Result<Type, Error> {
        /*
         * LocalRef numbering:
         *
         *   0 .. params.len()
         *       function parameters
         *
         *   params.len() ..
         *       function locals
         */

        if argno < self.function.params.len() {
            return Ok(self.function.params[argno]);
        }

        Err(Error::new(
            self.function,
            None,
            None,
            format!("invalid argno {}", argno),
        ))
    }

    fn local_type(&self, local: LocalRef) -> Result<Type, Error> {
        let index = local as usize;

        /*
         * LocalRef numbering:
         *
         *   0 .. params.len()
         *       function parameters
         *
         *   params.len() ..
         *       function locals
         */

        if index < self.function.params.len() {
            return Ok(self.function.params[index]);
        }

        let local_index = index - self.function.params.len();

        self.function
            .locals
            .get(local_index)
            .copied()
            .ok_or_else(|| {
                Error::new(
                    self.function,
                    None,
                    None,
                    format!("invalid localref {}", local),
                )
            })
    }

    fn function_ref(
        &self,
        reference: FunctionRef,
        label: LabelRef,
        index: usize,
    ) -> Result<&Function, Error> {
        self.module
            .functions
            .get(&reference)
            .ok_or_else(|| self.error(label, index, format!("invalid FunctionRef {}", reference)))
    }

    fn pop(&self, stack: &mut Stack, label: LabelRef, index: usize) -> Result<Type, Error> {
        stack
            .pop()
            .ok_or_else(|| self.error(label, index, "operand stack underflow"))
    }

    fn pop_binary(
        &self,
        stack: &mut Stack,
        label: LabelRef,
        index: usize,
    ) -> Result<(Type, Type), Error> {
        let right = self.pop(stack, label, index)?;
        let left = self.pop(stack, label, index)?;

        Ok((left, right))
    }

    fn expect_type(
        &self,
        actual: Type,
        expected: Type,
        label: LabelRef,
        index: usize,
    ) -> Result<(), Error> {
        if actual != expected {
            return Err(self.error(
                label,
                index,
                format!("type mismatch: expected {:?}, got {:?}", expected, actual),
            ));
        }

        Ok(())
    }

    fn verify_constant(
        &self,
        value: Constant,
        ty: Type,
        label: LabelRef,
        index: usize,
    ) -> Result<(), Error> {
        match value {
            Constant::Int(_) if ty == Type::Bool || ty.is_integer() => Ok(()),
            Constant::Float(_) if ty.is_float() => Ok(()),

            _ => Err(self.error(
                label,
                index,
                format!("constant does not match declared type {:?}", ty),
            )),
        }
    }
}

enum ControlFlow {
    Continue,
    Goto(LabelRef),
    Conditional(LabelRef),
    Return,
}

pub struct Verifier<'a> {
    module: &'a Module,
}

impl<'a> Verifier<'a> {
    pub fn new(module: &'a Module) -> Self {
        Self { module }
    }

    pub fn verify(&self) -> Result<(), Vec<Error>> {
        let mut errors = Vec::new();
        for function in self.module.functions.values() {
            errors.extend(FunctionVerifier::new(self.module, function).verify());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub fn verify_module(module: &Module) -> Result<(), Vec<Error>> {
    Verifier::new(module).verify()
}
