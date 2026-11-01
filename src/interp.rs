use crate::{
    Constant, FloatPredicate, Function, FunctionRef, Instruction, IntPredicate, LabelRef, LocalRef,
    Module, Type,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TypeMismatch,
    StackUnderflow,
    LocalNotFound(LocalRef),
    ArgOutOfBounds(usize),
    FunctionNotFound(FunctionRef),
    DivisionByZero,
    AssertionFailed,
    InvalidCast,
    InvalidOperation,
    InvalidControlFlow,
    ArgumentCountMismatch,
    LabelNotFound(LabelRef),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(u32),
    Float64(u64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.get_type();
        match self {
            Self::Null => write!(f, "{} null", ty),
            Self::Bool(v) => write!(f, "{} {}", ty, v),
            Self::Int8(v) => write!(f, "{} {}", ty, v),
            Self::Int16(v) => write!(f, "{} {}", ty, v),
            Self::Int32(v) => write!(f, "{} {}", ty, v),
            Self::Int64(v) => write!(f, "{} {}", ty, v),
            Self::Float32(v) => write!(f, "{} {}", ty, f32::from_bits(*v)),
            Self::Float64(v) => write!(f, "{} {}", ty, f64::from_bits(*v)),
        }
    }
}

impl Value {
    pub fn get_type(self) -> Type {
        match self {
            Self::Null => Type::Void,
            Self::Bool(_) => Type::Bool,
            Self::Int8(_) => Type::Int8,
            Self::Int16(_) => Type::Int16,
            Self::Int32(_) => Type::Int32,
            Self::Int64(_) => Type::Int64,
            Self::Float32(_) => Type::Float32,
            Self::Float64(_) => Type::Float64,
        }
    }

    pub fn to_i64(self) -> Result<i64> {
        match self {
            Self::Int8(v) => Ok(v as i64),
            Self::Int16(v) => Ok(v as i64),
            Self::Int32(v) => Ok(v as i64),
            Self::Int64(v) => Ok(v),
            _ => Err(Error::TypeMismatch),
        }
    }

    pub fn to_f64(self) -> Result<f64> {
        match self {
            Self::Float32(v) => Ok(f32::from_bits(v) as f64),
            Self::Float64(v) => Ok(f64::from_bits(v)),
            _ => Err(Error::TypeMismatch),
        }
    }

    pub fn to_bool(self) -> Result<bool> {
        match self {
            Self::Bool(v) => Ok(v),
            _ => Err(Error::TypeMismatch),
        }
    }
}

pub struct Frame {
    pub function_ref: FunctionRef,
    pub locals: HashMap<LocalRef, Value>,
    pub stack: Vec<Value>,
    pub current_block: usize,
    pub ip: usize,
    pub nargs: usize,
}

impl Frame {
    pub fn new(function_ref: FunctionRef, function: &Function, args: &[Value]) -> Result<Self> {
        if args.len() != function.params.len() {
            return Err(Error::ArgumentCountMismatch);
        }

        let mut locals = HashMap::new();
        for (index, (arg, expected)) in args.iter().zip(&function.params).enumerate() {
            if arg.get_type() != *expected {
                return Err(Error::TypeMismatch);
            }
            locals.insert(index as LocalRef, *arg);
        }

        for (index, ty) in function.locals.iter().enumerate() {
            let local_ref = function.params.len() + index;
            let local_ref =
                LocalRef::try_from(local_ref).map_err(|_| Error::LocalNotFound(u32::MAX))?;
            locals.insert(local_ref, default_value(*ty));
        }

        Ok(Self {
            function_ref,
            locals,
            stack: Vec::new(),
            current_block: 0,
            ip: 0,
            nargs: args.len(),
        })
    }
}

pub struct VM {
    pub module: Module,
    pub call_stack: Vec<Frame>,
}

impl VM {
    pub fn new(module: Module) -> Self {
        Self {
            module,
            call_stack: Vec::new(),
        }
    }

    pub fn invoke(&mut self, func_ref: FunctionRef, args: &[Value]) -> Result<Option<Value>> {
        let function = self
            .module
            .functions
            .get(&func_ref)
            .ok_or(Error::FunctionNotFound(func_ref))?;
        let frame = Frame::new(func_ref, function, args)?;
        self.call_stack.push(frame);
        self.run()
    }

    pub fn run(&mut self) -> Result<Option<Value>> {
        let result = self.run_inner();
        if result.is_err() {
            self.call_stack.clear();
        }
        result
    }

    fn run_inner(&mut self) -> Result<Option<Value>> {
        loop {
            let Some(frame) = self.call_stack.last() else {
                return Ok(None);
            };
            let function_ref = frame.function_ref;
            let block_index = frame.current_block;
            let instruction_index = frame.ip;
            let (return_type, local_types, labels, block_count, instruction) = {
                let function = self
                    .module
                    .functions
                    .get(&function_ref)
                    .ok_or(Error::FunctionNotFound(function_ref))?;
                let Some(block) = function.blocks.get(block_index) else {
                    return Err(Error::InvalidControlFlow);
                };
                (
                    function.return_type,
                    function
                        .params
                        .iter()
                        .chain(&function.locals)
                        .copied()
                        .collect::<Vec<_>>(),
                    function
                        .blocks
                        .iter()
                        .map(|block| block.label)
                        .collect::<Vec<_>>(),
                    function.blocks.len(),
                    block.instructions.get(instruction_index).copied(),
                )
            };

            let Some(instruction) = instruction else {
                if block_index + 1 >= block_count {
                    return Err(Error::InvalidControlFlow);
                }
                let frame = self.call_stack.last_mut().expect("frame exists");
                frame.current_block += 1;
                frame.ip = 0;
                continue;
            };

            self.call_stack.last_mut().expect("frame exists").ip += 1;

            match instruction {
                Instruction::LocalDecl(local_ref, ty) => {
                    let expected = get_local_type(&local_types, local_ref)?;
                    if expected != ty {
                        return Err(Error::TypeMismatch);
                    }
                }
                Instruction::LoadConstant(constant, ty) => {
                    let value = load_constant(constant, ty)?;
                    self.push(value);
                }
                Instruction::LoadArg(argno) => {
                    let frame = self.call_stack.last().expect("frame exists");

                    if argno >= frame.nargs {
                        return Err(Error::ArgOutOfBounds(argno));
                    }

                    let local_ref = argno as LocalRef;
                    let value = frame
                        .locals
                        .get(&local_ref)
                        .copied()
                        .ok_or(Error::LocalNotFound(local_ref))?;

                    self.push(value);
                }
                Instruction::LoadLocal(local_ref) => {
                    let value = self
                        .call_stack
                        .last()
                        .and_then(|frame| frame.locals.get(&local_ref))
                        .copied()
                        .ok_or(Error::LocalNotFound(local_ref))?;
                    self.push(value);
                }
                Instruction::StoreLocal(local_ref) => {
                    let expected = get_local_type(&local_types, local_ref)?;
                    let value = self.pop()?;
                    if value.get_type() != expected {
                        return Err(Error::TypeMismatch);
                    }
                    self.call_stack
                        .last_mut()
                        .expect("frame exists")
                        .locals
                        .insert(local_ref, value);
                }
                Instruction::Pop => {
                    self.pop()?;
                }
                Instruction::Dup => {
                    let value = self.pop()?;
                    self.push(value);
                    self.push(value);
                }
                Instruction::Swp => {
                    let r = self.pop()?;
                    let l = self.pop()?;
                    self.push(r);
                    self.push(l);
                }
                Instruction::Goto(label) => {
                    self.branch(&labels, label)?;
                }
                Instruction::GotoIfTrue(label) => {
                    if self.pop()?.to_bool()? {
                        self.branch(&labels, label)?;
                    }
                }
                Instruction::GotoIfFalse(label) => {
                    if !self.pop()?.to_bool()? {
                        self.branch(&labels, label)?;
                    }
                }
                Instruction::Assert => {
                    if !self.pop()?.to_bool()? {
                        return Err(Error::AssertionFailed);
                    }
                }
                Instruction::Ret => {
                    let frame = self.call_stack.last().expect("frame exists");

                    let value = if return_type.is_void() {
                        if !frame.stack.is_empty() {
                            return Err(Error::TypeMismatch);
                        }
                        None
                    } else {
                        let value = frame.stack.last().copied().ok_or(Error::StackUnderflow)?;

                        if value.get_type() != return_type || frame.stack.len() != 1 {
                            return Err(Error::TypeMismatch);
                        }

                        Some(value)
                    };

                    self.call_stack.pop();

                    if let Some(caller) = self.call_stack.last_mut() {
                        if let Some(value) = value {
                            caller.stack.push(value);
                        }
                    } else {
                        return Ok(value);
                    }
                }
                Instruction::Call(target) => {
                    let param_count = self
                        .module
                        .functions
                        .get(&target)
                        .ok_or(Error::FunctionNotFound(target))?
                        .params
                        .len();
                    let mut args = Vec::with_capacity(param_count);
                    for _ in 0..param_count {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let target_function = self
                        .module
                        .functions
                        .get(&target)
                        .ok_or(Error::FunctionNotFound(target))?;
                    let frame = Frame::new(target, target_function, &args)?;
                    self.call_stack.push(frame);
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
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(integer_binary(instruction, left, right)?);
                }
                Instruction::IAbs | Instruction::ISqrt | Instruction::INeg | Instruction::INot => {
                    let value = self.pop()?;
                    self.push(integer_unary(instruction, value)?);
                }
                Instruction::ICmp(predicate) => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(Value::Bool(integer_compare(predicate, left, right)?));
                }
                Instruction::FAdd
                | Instruction::FSub
                | Instruction::FMul
                | Instruction::FDiv
                | Instruction::FRem => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(float_binary(instruction, left, right)?);
                }
                Instruction::FAbs | Instruction::FSqrt | Instruction::FNeg => {
                    let value = self.pop()?;
                    self.push(float_unary(instruction, value)?);
                }
                Instruction::FCmp(predicate) => {
                    let right = self.pop()?.to_f64()?;
                    let left = self.pop()?.to_f64()?;
                    self.push(Value::Bool(float_compare(predicate, left, right)));
                }
                Instruction::Cast(target) => {
                    let value = self.pop()?;
                    self.push(cast(value, target)?);
                }
            }
        }
    }

    fn pop(&mut self) -> Result<Value> {
        self.call_stack
            .last_mut()
            .ok_or(Error::StackUnderflow)?
            .stack
            .pop()
            .ok_or(Error::StackUnderflow)
    }

    fn push(&mut self, value: Value) {
        self.call_stack
            .last_mut()
            .expect("instructions execute inside a frame")
            .stack
            .push(value);
    }

    fn branch(&mut self, labels: &[LabelRef], label: LabelRef) -> Result<()> {
        let block_index = labels
            .iter()
            .position(|candidate| *candidate == label)
            .ok_or(Error::LabelNotFound(label))?;
        let frame = self.call_stack.last_mut().expect("frame exists");
        frame.current_block = block_index;
        frame.ip = 0;
        Ok(())
    }
}

fn get_local_type(local_types: &[Type], local_ref: LocalRef) -> Result<Type> {
    local_types
        .get(local_ref as usize)
        .copied()
        .ok_or(Error::LocalNotFound(local_ref))
}

fn default_value(ty: Type) -> Value {
    match ty {
        Type::Void => Value::Null,
        Type::Bool => Value::Bool(false),
        Type::Int8 => Value::Int8(0),
        Type::Int16 => Value::Int16(0),
        Type::Int32 => Value::Int32(0),
        Type::Int64 => Value::Int64(0),
        Type::Float32 => Value::Float32(0),
        Type::Float64 => Value::Float64(0),
    }
}

fn load_constant(constant: Constant, ty: Type) -> Result<Value> {
    match (constant, ty) {
        (Constant::Int(value), Type::Bool) => Ok(Value::Bool(value != 0)),
        (Constant::Int(value), Type::Int8) => Ok(Value::Int8(value as i8)),
        (Constant::Int(value), Type::Int16) => Ok(Value::Int16(value as i16)),
        (Constant::Int(value), Type::Int32) => Ok(Value::Int32(value as i32)),
        (Constant::Int(value), Type::Int64) => Ok(Value::Int64(value)),
        (Constant::Float(value), Type::Float32) => Ok(Value::Float32((value as f32).to_bits())),
        (Constant::Float(value), Type::Float64) => Ok(Value::Float64(value.to_bits())),
        _ => Err(Error::TypeMismatch),
    }
}

fn integer_width(ty: Type) -> Result<u32> {
    match ty {
        Type::Int8 => Ok(8),
        Type::Int16 => Ok(16),
        Type::Int32 => Ok(32),
        Type::Int64 => Ok(64),
        _ => Err(Error::TypeMismatch),
    }
}

fn integer_value(value: Value) -> Result<i64> {
    value.to_i64()
}

fn integer_bits(value: Value) -> Result<u64> {
    let width = integer_width(value.get_type())?;
    let mask = width_mask(width);
    Ok((integer_value(value)? as u64) & mask)
}

fn width_mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn from_integer_bits(ty: Type, bits: u64) -> Result<Value> {
    let bits = bits & width_mask(integer_width(ty)?);
    match ty {
        Type::Int8 => Ok(Value::Int8(bits as u8 as i8)),
        Type::Int16 => Ok(Value::Int16(bits as u16 as i16)),
        Type::Int32 => Ok(Value::Int32(bits as u32 as i32)),
        Type::Int64 => Ok(Value::Int64(bits as i64)),
        _ => Err(Error::TypeMismatch),
    }
}

fn integer_binary(instruction: Instruction, left: Value, right: Value) -> Result<Value> {
    let ty = left.get_type();
    if ty != right.get_type() {
        return Err(Error::TypeMismatch);
    }
    let width = integer_width(ty)?;
    let mask = width_mask(width);
    let lhs_bits = integer_bits(left)?;
    let rhs_bits = integer_bits(right)?;
    if matches!(instruction, Instruction::UDiv | Instruction::URem) && rhs_bits == 0 {
        return Err(Error::DivisionByZero);
    }

    let result = match instruction {
        Instruction::IAdd => lhs_bits.wrapping_add(rhs_bits),
        Instruction::ISub => lhs_bits.wrapping_sub(rhs_bits),
        Instruction::IMul => lhs_bits.wrapping_mul(rhs_bits),
        Instruction::IDiv => {
            let lhs = integer_value(left)? as i128;
            let rhs = integer_value(right)? as i128;
            if rhs == 0 {
                return Err(Error::DivisionByZero);
            }
            (lhs / rhs) as u64
        }
        Instruction::UDiv => lhs_bits / rhs_bits,
        Instruction::IRem => {
            let lhs = integer_value(left)? as i128;
            let rhs = integer_value(right)? as i128;
            if rhs == 0 {
                return Err(Error::DivisionByZero);
            }
            (lhs % rhs) as u64
        }
        Instruction::URem => lhs_bits % rhs_bits,
        Instruction::IAnd => lhs_bits & rhs_bits,
        Instruction::IOr => lhs_bits | rhs_bits,
        Instruction::IXor => lhs_bits ^ rhs_bits,
        Instruction::IShl => lhs_bits.wrapping_shl(rhs_bits as u32),
        Instruction::IShr => lhs_bits.wrapping_shr(rhs_bits as u32),
        Instruction::ISar => {
            let lhs = integer_value(left)? as i128;
            let rhs = integer_value(right)? as i128;
            (lhs.wrapping_shr(rhs as u32) as u64) & mask
        }
        _ => return Err(Error::InvalidOperation),
    };
    from_integer_bits(ty, result & mask)
}

fn integer_unary(instruction: Instruction, value: Value) -> Result<Value> {
    let ty = value.get_type();
    let width = integer_width(ty)?;
    let bits = integer_bits(value)?;
    match instruction {
        Instruction::IAbs => {
            let value = integer_value(value)?;
            from_integer_bits(ty, value.wrapping_abs() as u64)
        }
        Instruction::INeg => {
            let value = integer_value(value)?;
            from_integer_bits(ty, value.wrapping_neg() as u64)
        }
        Instruction::ISqrt => {
            let value = integer_value(value)?;
            if value < 0 {
                return Err(Error::InvalidOperation);
            }
            let value = value as u64;
            let mut low = 0u64;
            let mut high = (1u64 << width.div_ceil(2)).min(value.saturating_add(1));
            while low < high {
                let mid = low + (high - low).div_ceil(2);
                if mid <= value / mid.max(1) {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            from_integer_bits(ty, low)
        }
        Instruction::INot => {
            let bits = !bits & width_mask(width);
            from_integer_bits(ty, bits)
        }
        _ => {
            let _ = bits;
            Err(Error::InvalidOperation)
        }
    }
}

fn integer_compare(predicate: IntPredicate, left: Value, right: Value) -> Result<bool> {
    if left.get_type() != right.get_type() {
        return Err(Error::TypeMismatch);
    }
    let signed_left = integer_value(left)?;
    let signed_right = integer_value(right)?;
    let unsigned_left = integer_bits(left)?;
    let unsigned_right = integer_bits(right)?;
    Ok(match predicate {
        IntPredicate::Eq => signed_left == signed_right,
        IntPredicate::Ne => signed_left != signed_right,
        IntPredicate::Slt => signed_left < signed_right,
        IntPredicate::Sle => signed_left <= signed_right,
        IntPredicate::Sgt => signed_left > signed_right,
        IntPredicate::Sge => signed_left >= signed_right,
        IntPredicate::Ult => unsigned_left < unsigned_right,
        IntPredicate::Ule => unsigned_left <= unsigned_right,
        IntPredicate::Ugt => unsigned_left > unsigned_right,
        IntPredicate::Uge => unsigned_left >= unsigned_right,
    })
}

fn float_binary(instruction: Instruction, left: Value, right: Value) -> Result<Value> {
    if left.get_type() != right.get_type() {
        return Err(Error::TypeMismatch);
    }
    match (instruction, left, right) {
        (op, Value::Float32(a), Value::Float32(b)) => Ok(Value::Float32(
            apply_f32(op, f32::from_bits(a), f32::from_bits(b))?.to_bits(),
        )),
        (op, Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(
            apply_f64(op, f64::from_bits(a), f64::from_bits(b))?.to_bits(),
        )),
        _ => Err(Error::TypeMismatch),
    }
}

fn apply_f32(instruction: Instruction, left: f32, right: f32) -> Result<f32> {
    match instruction {
        Instruction::FAdd => Ok(left + right),
        Instruction::FSub => Ok(left - right),
        Instruction::FMul => Ok(left * right),
        Instruction::FDiv => Ok(left / right),
        Instruction::FRem => Ok(left % right),
        _ => Err(Error::InvalidOperation),
    }
}

fn apply_f64(instruction: Instruction, left: f64, right: f64) -> Result<f64> {
    match instruction {
        Instruction::FAdd => Ok(left + right),
        Instruction::FSub => Ok(left - right),
        Instruction::FMul => Ok(left * right),
        Instruction::FDiv => Ok(left / right),
        Instruction::FRem => Ok(left % right),
        _ => Err(Error::InvalidOperation),
    }
}

fn float_unary(instruction: Instruction, value: Value) -> Result<Value> {
    match (instruction, value) {
        (Instruction::FAbs, Value::Float32(v)) => Ok(Value::Float32(v & 0x7fff_ffff)),
        (Instruction::FSqrt, Value::Float32(v)) => {
            Ok(Value::Float32(f32::from_bits(v).sqrt().to_bits()))
        }
        (Instruction::FNeg, Value::Float32(v)) => Ok(Value::Float32(v ^ 0x8000_0000)),
        (Instruction::FAbs, Value::Float64(v)) => Ok(Value::Float64(v & 0x7fff_ffff_ffff_ffff)),
        (Instruction::FSqrt, Value::Float64(v)) => {
            Ok(Value::Float64(f64::from_bits(v).sqrt().to_bits()))
        }
        (Instruction::FNeg, Value::Float64(v)) => Ok(Value::Float64(v ^ 0x8000_0000_0000_0000)),
        _ => Err(Error::TypeMismatch),
    }
}

fn float_compare(predicate: FloatPredicate, left: f64, right: f64) -> bool {
    let unordered = left.is_nan() || right.is_nan();
    match predicate {
        FloatPredicate::Eq => left == right,
        FloatPredicate::Ne => left != right,
        FloatPredicate::Olt => !unordered && left < right,
        FloatPredicate::Ole => !unordered && left <= right,
        FloatPredicate::Ogt => !unordered && left > right,
        FloatPredicate::Oge => !unordered && left >= right,
        FloatPredicate::Ult => unordered || left < right,
        FloatPredicate::Ule => unordered || left <= right,
        FloatPredicate::Ugt => unordered || left > right,
        FloatPredicate::Uge => unordered || left >= right,
    }
}

fn cast(value: Value, target: Type) -> Result<Value> {
    if value.get_type() == target {
        return Ok(value);
    }
    match target {
        Type::Void => todo!("cast into void is not allowed"),
        Type::Bool => match value {
            Value::Int8(_) | Value::Int16(_) | Value::Int32(_) | Value::Int64(_) => {
                Ok(Value::Bool(integer_value(value)? != 0))
            }
            _ => Err(Error::InvalidCast),
        },
        Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64 => match value {
            Value::Bool(value) => from_integer_bits(target, u64::from(value)),
            Value::Int8(_) | Value::Int16(_) | Value::Int32(_) | Value::Int64(_) => {
                from_integer_bits(target, integer_value(value)? as u64)
            }
            _ => cast_float_to_integer(value.to_f64()?, target),
        },
        Type::Float32 | Type::Float64 => {
            let float = match value {
                Value::Int8(_) | Value::Int16(_) | Value::Int32(_) | Value::Int64(_) => {
                    let integer = integer_value(value)?;
                    return Ok(match target {
                        Type::Float32 => Value::Float32((integer as f32).to_bits()),
                        Type::Float64 => Value::Float64((integer as f64).to_bits()),
                        _ => unreachable!(),
                    });
                }
                Value::Bool(_) => return Err(Error::InvalidCast),
                _ => value.to_f64()?,
            };
            match target {
                Type::Float32 => Ok(Value::Float32((float as f32).to_bits())),
                Type::Float64 => Ok(Value::Float64(float.to_bits())),
                _ => unreachable!(),
            }
        }
    }
}

fn cast_float_to_integer(value: f64, target: Type) -> Result<Value> {
    match target {
        Type::Int8 => Ok(Value::Int8(value as i8)),
        Type::Int16 => Ok(Value::Int16(value as i16)),
        Type::Int32 => Ok(Value::Int32(value as i32)),
        Type::Int64 => Ok(Value::Int64(value as i64)),
        _ => Err(Error::InvalidCast),
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, VM};
    use crate::{BasicBlock, Function, Instruction, Module, Type};

    #[test]
    fn interpreter_is_available_through_sema_module() {
        let mut module = Module::new("test");
        let function = Function::new("identity", &[Type::Int32], Type::Int32).add_block(
            BasicBlock::new(0).with_instructions(&[Instruction::LoadLocal(0), Instruction::Ret]),
        );
        let function_ref = module.add_function(function);

        let mut vm = VM::new(module);
        assert_eq!(
            vm.invoke(function_ref, &[Value::Int32(42)]).unwrap(),
            Some(Value::Int32(42))
        );
    }
}
