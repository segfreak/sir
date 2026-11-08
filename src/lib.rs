use std::collections::HashMap;

use crate::traits::ToStaticStr;

pub mod interp;
pub mod sema;
pub mod traits;

pub type LocalRef = u32;

pub type LabelRef = u32;

pub type FunctionRef = u32;

pub type ArgRef = usize;

pub type InstructionRef = usize;

#[derive(Debug, Clone, Copy)]

pub enum Constant {
    Int(i64),
    Float(f64),
}

impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Int(value) => write!(f, "{value}"),
            Constant::Float(value) => {
                if value.is_nan() {
                    write!(f, "nan")
                } else if *value == f64::INFINITY {
                    write!(f, "inf")
                } else if *value == f64::NEG_INFINITY {
                    write!(f, "-inf")
                } else {
                    write!(f, "{value}")
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]

pub enum Type {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
}

impl ToStaticStr for Type {
    fn to_static_str(self) -> &'static str {
        match self {
            Type::Void => "void",
            Type::Bool => "i1",
            Type::Int8 => "i8",
            Type::Int16 => "i16",
            Type::Int32 => "i32",
            Type::Int64 => "i64",
            Type::Float32 => "f32",
            Type::Float64 => "f64",
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

impl Type {
    pub const fn is_integer(self) -> bool {
        matches!(self, Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64)
    }

    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    pub const fn is_bool(self) -> bool {
        matches!(self, Self::Bool)
    }

    pub const fn is_void(self) -> bool {
        matches!(self, Self::Void)
    }

    pub const fn bits(self) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool => 1,
            Self::Int8 => 8,
            Self::Int16 => 16,
            Self::Int32 => 32,
            Self::Int64 => 64,
            Self::Float32 => 32,
            Self::Float64 => 64,
        }
    }

    pub const fn bytes(self) -> usize {
        if self.is_void() {
            0
        } else {
            self.bits().div_ceil(8)
        }
    }

    pub fn can_cast_to(self, to: Self) -> bool {
        if self == to {
            return true;
        }

        // int -> int, float -> float
        if self.is_integer() && to.is_integer() || self.is_float() && to.is_float() {
            return true;
        }

        // int -> float, float -> int
        if self.is_integer() && to.is_float() || self.is_float() && to.is_integer() {
            return true;
        }

        // bool -> int, int -> bool
        if self.is_bool() && to.is_integer() || self.is_integer() && to.is_bool() {
            return true;
        }

        false
    }
}

#[derive(Debug, Clone, Copy)]

pub enum IntPredicate {
    Eq,
    Ne,
    Slt,
    Sle,
    Sgt,
    Sge,
    Ult,
    Ule,
    Ugt,
    Uge,
}

#[derive(Debug, Clone, Copy)]

pub enum FloatPredicate {
    Eq,
    Ne,
    Olt,
    Ole,
    Ogt,
    Oge,
    Ult,
    Ule,
    Ugt,
    Uge,
}

impl ToStaticStr for IntPredicate {
    fn to_static_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Slt => "slt",
            Self::Sle => "sle",
            Self::Sgt => "sgt",
            Self::Sge => "sge",
            Self::Ult => "ult",
            Self::Ule => "ule",
            Self::Ugt => "ugt",
            Self::Uge => "uge",
        }
    }
}

impl ToStaticStr for FloatPredicate {
    fn to_static_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Olt => "olt",
            Self::Ole => "ole",
            Self::Ogt => "ogt",
            Self::Oge => "oge",
            Self::Ult => "ult",
            Self::Ule => "ule",
            Self::Ugt => "ugt",
            Self::Uge => "uge",
        }
    }
}

impl std::fmt::Display for IntPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

impl std::fmt::Display for FloatPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    // Копирует второй элемент стека на вершину
    // [a, b] => [a, b, a]
    Over,

    // Удаляет второй элемент стека, оставляя вершину на месте.
    // [a, b] => [b]
    Nip,

    // Дублирует вершину стека под второй элемент
    // [a, b] => [b, a, b]
    Tuck,

    // Удаляет верхний элемент стека
    // [a, b] => [a]
    Pop,

    // Дублирует элемент на вершине стека
    // [a] => [a, b]
    Dup,

    // Меняет местами два элемента на вершине стека
    // [a, b] => [b, a]
    Swp,

    // Циклический сдвиг трех верхних элементов стека
    // [a, b, c] => [c, a, b]
    Rot,

    // Копирует элемент с глубины n на вершину стека (0 — это вершина)
    // [..., a, b, c] pick(1) => [..., a, b, c, b]
    Pick(usize),

    // Переносит элемент с глубины n на вершину стека, сдвигая промежуточные
    // [..., a, b, c] roll(1) => [..., a, c, b] (если 1 — это b)
    Roll(usize),

    // Загружает константу на вершину
    LoadConstant(Constant, Type),

    // Загружает значение аргумента функции на вершину
    LoadArg(ArgRef),

    // Загружает значение слота на вершину
    LoadLocal(LocalRef),

    // Читает значение вершины в слот
    StoreLocal(LocalRef),

    Assert,
    Goto(LabelRef),
    GotoIfTrue(LabelRef),
    GotoIfFalse(LabelRef),
    Ret,
    Call(FunctionRef),

    IAdd,
    ISub,
    IMul,
    IDiv,
    UDiv,
    IRem,
    URem,
    IAbs,
    ISqrt,
    INeg,
    ICmp(IntPredicate),
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FAbs,
    FSqrt,
    FNeg,
    FCmp(FloatPredicate),
    Cast(Type),

    IAnd,
    IOr,
    IXor,
    INot,
    IShl,
    IShr,
    ISar,
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::LoadConstant(constant, ty) => {
                write!(f, "ldc.{} {}", ty, constant)
            }
            Instruction::LoadArg(local) => write!(f, "ldarg %{local}"),
            Instruction::LoadLocal(local) => write!(f, "ldloc %{local}"),
            Instruction::StoreLocal(local) => write!(f, "stloc %{local}"),
            Instruction::Assert => write!(f, "assert"),
            Instruction::Goto(label) => write!(f, "br      .L{label}"),
            Instruction::GotoIfTrue(label) => write!(f, "brtrue  .L{label}"),
            Instruction::GotoIfFalse(label) => write!(f, "brfalse .L{label}"),
            Instruction::Ret => write!(f, "ret"),
            Instruction::Call(function) => write!(f, "call @{function}"),
            Instruction::Pop => write!(f, "pop"),
            Instruction::Dup => write!(f, "dup"),
            Instruction::Swp => write!(f, "swp"),
            Instruction::Over => write!(f, "over"),
            Instruction::Nip => write!(f, "nip"),
            Instruction::Tuck => write!(f, "tuck"),
            Instruction::Rot => write!(f, "rot"),
            Instruction::Pick(n) => write!(f, "pick {n}"),
            Instruction::Roll(n) => write!(f, "roll {n}"),
            Instruction::IAdd => write!(f, "iadd"),
            Instruction::ISub => write!(f, "isub"),
            Instruction::IMul => write!(f, "imul"),
            Instruction::IDiv => write!(f, "idiv"),
            Instruction::UDiv => write!(f, "udiv"),
            Instruction::IRem => write!(f, "irem"),
            Instruction::URem => write!(f, "urem"),
            Instruction::IAbs => write!(f, "iabs"),
            Instruction::ISqrt => write!(f, "isqrt"),
            Instruction::INeg => write!(f, "ineg"),
            Instruction::IAnd => write!(f, "iand"),
            Instruction::IOr => write!(f, "ior"),
            Instruction::IXor => write!(f, "ixor"),
            Instruction::INot => write!(f, "inot"),
            Instruction::IShl => write!(f, "ishl"),
            Instruction::IShr => write!(f, "ishr"),
            Instruction::ISar => write!(f, "isar"),
            Instruction::ICmp(predicate) => {
                write!(f, "i{}", predicate)
            }
            Instruction::FAdd => write!(f, "fadd"),
            Instruction::FSub => write!(f, "fsub"),
            Instruction::FMul => write!(f, "fmul"),
            Instruction::FDiv => write!(f, "fdiv"),
            Instruction::FRem => write!(f, "frem"),
            Instruction::FAbs => write!(f, "fabs"),
            Instruction::FSqrt => write!(f, "fsqrt"),
            Instruction::FNeg => write!(f, "fneg"),
            Instruction::FCmp(predicate) => {
                write!(f, "f{}", predicate)
            }
            Instruction::Cast(ty) => write!(f, "cast.{}", ty),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: LabelRef,
    pub instructions: Vec<Instruction>,
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, " .L{}:", self.label)?;

        for (index, instruction) in self.instructions.iter().enumerate() {
            // writeln!(f, "    {index:04}  {instruction}")?;
            writeln!(f, "   + {index:04}    {instruction}")?;
        }

        Ok(())
    }
}

impl BasicBlock {
    pub fn new(label: LabelRef) -> Self {
        Self {
            label,
            instructions: Vec::new(),
        }
    }

    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    pub fn with_instructions(mut self, instructions: &[Instruction]) -> Self {
        self.instructions = instructions.to_vec();
        self
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Type>,
    pub return_type: Type,

    pub locals: Vec<Type>,
    pub blocks: Vec<BasicBlock>,
}

impl Function {
    fn fmt_impl(
        &self,
        func_ref: Option<FunctionRef>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "function @{} {:?}(",
            func_ref.map(|r| r.to_string()).unwrap_or('?'.to_string()),
            self.name
        )?;

        for (index, ty) in self.params.iter().enumerate() {
            if index != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", ty)?;
        }

        writeln!(f, ") -> {} {{", self.return_type)?;

        write!(f, "  .locals [")?;
        for (index, ty) in self.locals.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            write!(f, "%{index}: {ty}")?;
        }
        writeln!(f, "]")?;

        for block in self.blocks.iter() {
            writeln!(f)?;
            write!(f, "{}", block)?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_impl(None, f)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionViewWithRef<'a> {
    funcref: FunctionRef,
    func: &'a Function,
}

impl<'a> FunctionViewWithRef<'a> {
    pub fn new(r: FunctionRef, f: &'a Function) -> Self {
        Self {
            funcref: r,
            func: f,
        }
    }
}

impl<'a> Function {
    pub fn into_view_with_ref(&'a self, funcref: FunctionRef) -> FunctionViewWithRef<'a> {
        FunctionViewWithRef::new(funcref, self)
    }
}

impl<'a> std::fmt::Display for FunctionViewWithRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.func.fmt_impl(Some(self.funcref), f)
    }
}

impl Default for Function {
    fn default() -> Self {
        Self {
            name: String::new(),
            params: Vec::new(),
            return_type: Type::Int32,
            locals: Vec::new(),
            blocks: Vec::new(),
        }
    }
}

impl Function {
    pub fn new(name: &str, params: &[Type], return_type: Type) -> Self {
        Self {
            name: name.into(),
            params: params.to_vec(),
            return_type,
            locals: Vec::new(),
            blocks: Vec::new(),
        }
    }

    pub fn add_block(mut self, block: BasicBlock) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn with_locals(mut self, locals: &[Type]) -> Self {
        self.locals = locals.to_vec();
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub name: String,
    pub functions: HashMap<FunctionRef, Function>,

    names: HashMap<String, FunctionRef>,
    function_counter: FunctionRef,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Self::default()
        }
    }

    pub fn next_function_ref(&mut self) -> FunctionRef {
        let function_ref = self.function_counter;
        self.function_counter += 1;
        function_ref
    }

    pub fn add_function(&mut self, function: Function) -> FunctionRef {
        let mut function_ref = self.next_function_ref();
        while self.has_function_ref(function_ref) {
            function_ref = self.next_function_ref();
        }
        self.set_function(function_ref, function);
        function_ref
    }

    pub fn set_function(&mut self, function_ref: FunctionRef, function: Function) {
        self.names.insert(function.name.clone(), function_ref);
        self.functions.insert(function_ref, function);
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.values().any(|f| f.name == name)
    }

    pub fn has_function_ref(&self, function_ref: FunctionRef) -> bool {
        self.functions.contains_key(&function_ref)
    }

    pub fn lookup_function(&self, name: &str) -> Option<FunctionRef> {
        self.names.get(name).copied()
    }

    pub fn get_function(&self, function_ref: FunctionRef) -> Option<&Function> {
        self.functions.get(&function_ref)
    }

    pub fn get_function_mut(&mut self, function_ref: FunctionRef) -> Option<&mut Function> {
        self.functions.get_mut(&function_ref)
    }
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "module {};", self.name)?;
        writeln!(f)?;

        let mut functions: Vec<_> = self.functions.iter().collect();
        functions.sort_by_key(|(function_ref, _)| **function_ref);

        for (index, (function_ref, function)) in functions.iter().enumerate() {
            if index != 0 {
                writeln!(f)?;
            }

            let view = function.into_view_with_ref(**function_ref);
            write!(f, "{}", view)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{interp::VM, sema::verify::verify_module, *};

    #[test]
    fn test_stack_operations() {
        pretty_env_logger::try_init().expect("failed to init logger");

        let mut instructions = Vec::new();

        // Over: [10, 20] => [10, 20, 10]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::Over,
            Instruction::StoreLocal(2),
            Instruction::StoreLocal(1),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(1),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(2),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Nip: [10, 20] => [20]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::Nip,
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Tuck: [10, 20] => [20, 10, 20]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::Tuck,
            Instruction::StoreLocal(2),
            Instruction::StoreLocal(1),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(1),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(2),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Pop: [10, 20] => [10]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::Pop,
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Dup: [10] => [10, 10]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::Dup,
            Instruction::StoreLocal(1),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadLocal(1),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Swp: [10, 20] => [20, 10]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::Swp,
            Instruction::StoreLocal(1),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(1),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Rot: [10, 20, 30] => [30, 10, 20]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::LoadConstant(Constant::Int(30), Type::Int32),
            Instruction::Rot,
            Instruction::StoreLocal(2), // 10
            Instruction::StoreLocal(1), // 30
            Instruction::StoreLocal(0), // 20
            Instruction::LoadLocal(0),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(1),
            Instruction::LoadConstant(Constant::Int(30), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(2),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        // Pick(1): [10, 20, 30] => [10, 20, 30, 20]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::LoadConstant(Constant::Int(30), Type::Int32),
            Instruction::Pick(1),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::Pop,
            Instruction::Pop,
            Instruction::Pop,
        ]);

        // Roll(1): [10, 20, 30] => [10, 30, 20]
        instructions.extend([
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::LoadConstant(Constant::Int(30), Type::Int32),
            Instruction::Roll(1),
            Instruction::StoreLocal(2),
            Instruction::StoreLocal(1),
            Instruction::StoreLocal(0),
            Instruction::LoadLocal(0),
            Instruction::LoadConstant(Constant::Int(10), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(1),
            Instruction::LoadConstant(Constant::Int(30), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
            Instruction::LoadLocal(2),
            Instruction::LoadConstant(Constant::Int(20), Type::Int32),
            Instruction::ICmp(IntPredicate::Eq),
            Instruction::Assert,
        ]);

        instructions.push(Instruction::Ret);

        let mut module = Module::new("m1");

        let function = Function::new("fn", &[], Type::Void)
            .with_locals(&[Type::Int32; 4])
            .add_block(BasicBlock::new(0).with_instructions(&instructions));

        let func_ref = module.add_function(function);

        log::debug!("{}", module);

        if let Err(errors) = verify_module(&module) {
            for error in errors {
                log::error!("{error}");
            }
        }

        let mut vm = VM::new(module);
        if let Err(error) = vm.invoke(func_ref, &[]) {
            log::error!("execution failed: {error:?}");
        }
    }
}
