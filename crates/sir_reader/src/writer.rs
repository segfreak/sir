use std::fmt::Write;

use sir::*;

pub struct Writer;

impl Writer {
    pub fn write_module(module: &Module) -> String {
        let mut output = String::new();

        let mut functions: Vec<_> = module.functions.iter().collect();
        functions.sort_by_key(|(id, _)| *id);

        let function_names: Vec<String> = functions
            .iter()
            .map(|(_, function)| function.name.to_string())
            .collect();

        for (index, (_, function)) in functions.iter().enumerate() {
            if index != 0 {
                output.push('\n');
            }

            Self::write_function(&mut output, function, &function_names);
        }

        output
    }

    fn write_function(output: &mut String, function: &Function, function_names: &[String]) {
        write!(output, "function {}(", function.name).unwrap();

        for (index, ty) in function.params.iter().enumerate() {
            if index != 0 {
                output.push_str(", ");
            }

            write!(output, "arg{index}: {}", Self::write_type(ty)).unwrap();
        }

        writeln!(output, "): {} {{", Self::write_type(&function.return_type)).unwrap();

        if !function.locals.is_empty() {
            output.push_str("    locals [");

            for (index, ty) in function.locals.iter().enumerate() {
                if index != 0 {
                    output.push_str(", ");
                }

                write!(output, "local{index}: {}", Self::write_type(ty)).unwrap();
            }

            output.push_str("]\n");
        }

        if !function.blocks.is_empty() {
            output.push('\n');
        }

        for (block_index, block) in function.blocks.iter().enumerate() {
            writeln!(output, "bb{}:", block.label).unwrap();

            for instruction in &block.instructions {
                output.push_str("    ");

                Self::write_instruction(output, instruction, function_names);

                output.push('\n');
            }

            if block_index + 1 != function.blocks.len() {
                output.push('\n');
            }
        }

        output.push_str("}\n");
    }

    fn write_type(ty: &Type) -> &'static str {
        match ty {
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

    fn write_constant(constant: &Constant) -> String {
        match constant {
            Constant::Int(value) => value.to_string(),
            Constant::Float(value) => {
                if value.is_finite() {
                    value.to_string()
                } else {
                    format!("{value:?}")
                }
            }
        }
    }

    fn write_int_predicate(predicate: &IntPredicate) -> &'static str {
        match predicate {
            IntPredicate::Eq => "eq",
            IntPredicate::Ne => "ne",
            IntPredicate::Slt => "slt",
            IntPredicate::Sle => "sle",
            IntPredicate::Sgt => "sgt",
            IntPredicate::Sge => "sge",
            IntPredicate::Ult => "ult",
            IntPredicate::Ule => "ule",
            IntPredicate::Ugt => "ugt",
            IntPredicate::Uge => "uge",
        }
    }

    fn write_float_predicate(predicate: &FloatPredicate) -> &'static str {
        match predicate {
            FloatPredicate::Eq => "feq",
            FloatPredicate::Ne => "fne",
            FloatPredicate::Olt => "folt",
            FloatPredicate::Ole => "fole",
            FloatPredicate::Ogt => "fogt",
            FloatPredicate::Oge => "foge",
            FloatPredicate::Ult => "fult",
            FloatPredicate::Ule => "fule",
            FloatPredicate::Ugt => "fugt",
            FloatPredicate::Uge => "fuge",
        }
    }

    fn write_instruction(
        output: &mut String,
        instruction: &Instruction,
        function_names: &[String],
    ) {
        match instruction {
            Instruction::LoadArg(index) => {
                write!(output, "ldarg #{index}").unwrap();
            }

            Instruction::LoadLocal(index) => {
                write!(output, "ldloc %{index}").unwrap();
            }

            Instruction::StoreLocal(index) => {
                write!(output, "stloc %{index}").unwrap();
            }

            Instruction::LoadConstant(constant, ty) => {
                write!(
                    output,
                    "ldc {} {}",
                    Self::write_type(ty),
                    Self::write_constant(constant)
                )
                .unwrap();
            }

            Instruction::Call(function_ref) => {
                let name = function_names
                    .get(*function_ref as usize)
                    .map(String::as_str)
                    .unwrap_or("<invalid-function>");

                write!(output, "call {name}").unwrap();
            }

            Instruction::Goto(label) => {
                write!(output, "br bb{label}").unwrap();
            }

            Instruction::GotoIfTrue(label) => {
                write!(output, "brtrue bb{label}").unwrap();
            }

            Instruction::GotoIfFalse(label) => {
                write!(output, "brfalse bb{label}").unwrap();
            }

            Instruction::Ret => output.push_str("ret"),
            Instruction::Assert => output.push_str("assert"),

            Instruction::Pop => output.push_str("pop"),
            Instruction::Dup => output.push_str("dup"),
            Instruction::Swp => output.push_str("swp"),
            Instruction::Over => output.push_str("over"),
            Instruction::Nip => output.push_str("nip"),
            Instruction::Tuck => output.push_str("tuck"),
            Instruction::Rot => output.push_str("rot"),

            Instruction::Pick(index) => {
                write!(output, "pick {index}").unwrap();
            }

            Instruction::Roll(index) => {
                write!(output, "roll {index}").unwrap();
            }

            Instruction::IAdd => output.push_str("iadd"),
            Instruction::ISub => output.push_str("isub"),
            Instruction::IMul => output.push_str("imul"),
            Instruction::IDiv => output.push_str("idiv"),
            Instruction::UDiv => output.push_str("udiv"),
            Instruction::IRem => output.push_str("irem"),
            Instruction::URem => output.push_str("urem"),
            Instruction::IAbs => output.push_str("iabs"),
            Instruction::INeg => output.push_str("ineg"),

            Instruction::IAnd => output.push_str("iand"),
            Instruction::IOr => output.push_str("ior"),
            Instruction::IXor => output.push_str("ixor"),
            Instruction::INot => output.push_str("inot"),
            Instruction::IShl => output.push_str("ishl"),
            Instruction::IShr => output.push_str("ishr"),
            Instruction::ISar => output.push_str("isar"),

            Instruction::ICmp(predicate) => {
                output.push_str(Self::write_int_predicate(predicate));
            }

            Instruction::FAdd => output.push_str("fadd"),
            Instruction::FSub => output.push_str("fsub"),
            Instruction::FMul => output.push_str("fmul"),
            Instruction::FDiv => output.push_str("fdiv"),
            Instruction::FRem => output.push_str("frem"),
            Instruction::FAbs => output.push_str("fabs"),
            Instruction::FNeg => output.push_str("fneg"),

            Instruction::FCmp(predicate) => {
                output.push_str(Self::write_float_predicate(predicate));
            }

            Instruction::Cast(ty) => {
                write!(output, "cast {}", Self::write_type(ty)).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(source: &str) -> Module {
        let tokens = Lexer::new(source).tokenize().expect("lexer failed");

        Parser::new(tokens)
            .parse_module("test")
            .expect("parser failed")
    }

    fn round_trip(source: &str) -> (Module, Module, String) {
        let first = parse(source);

        let output = Writer::write_module(&first);

        let second = parse(&output);

        (first, second, output)
    }

    #[test]
    fn writer_simple_function() {
        let source = r#"
            function foo(a: i32): i32 {
            entry:
                ldarg #a
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("function foo(arg0: i32): i32"));
        assert!(output.contains("ldarg #0"));
        assert!(output.contains("ret"));
    }

    #[test]
    fn writer_function_with_locals() {
        let source = r#"
            function foo(a: i32): i32 {
                locals [x: i32]

            entry:
                ldarg #a
                stloc %x
                ldloc %x
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("locals [local0: i32]"));
        assert!(output.contains("stloc %0"));
        assert!(output.contains("ldloc %0"));
    }

    #[test]
    fn writer_function_without_locals() {
        let source = r#"
            function foo(a: i32): i32 {
            entry:
                ldarg #a
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(!output.contains("locals"));
    }

    #[test]
    fn writer_integer_constants() {
        let source = r#"
            function foo(): i32 {
            entry:
                ldc i32 42
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("ldc i32 42"));
    }

    #[test]
    fn writer_float_constants() {
        let source = r#"
            function foo(): f64 {
            entry:
                ldc f64 3.14
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("ldc f64 3.14"));
    }

    #[test]
    fn writer_arithmetic() {
        let source = r#"
            function add(a: i32, b: i32): i32 {
            entry:
                ldarg #a
                ldarg #b
                iadd
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("iadd"));
    }

    #[test]
    fn writer_integer_predicates() {
        let source = r#"
            function cmp(a: i32, b: i32): i1 {
            entry:
                ldarg #a
                ldarg #b
                slt
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("slt"));
    }

    #[test]
    fn writer_float_predicates() {
        let source = r#"
            function cmp(a: f64, b: f64): i1 {
            entry:
                ldarg #a
                ldarg #b
                folt
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("folt"));
    }

    #[test]
    fn writer_cast() {
        let source = r#"
            function foo(a: i32): i64 {
            entry:
                ldarg #a
                cast i64
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("cast i64"));
    }

    #[test]
    fn writer_branches() {
        let source = r#"
            function foo(a: i1): void {
            entry:
                ldarg #a
                brtrue yes
                br no

            yes:
                ret

            no:
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("brtrue bb"));
        assert!(output.contains("br bb"));
    }

    #[test]
    fn writer_stack_instructions() {
        let source = r#"
            function foo(): void {
            entry:
                ldc i32 1
                dup
                pop
                assert
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("dup"));
        assert!(output.contains("pop"));
        assert!(output.contains("assert"));
    }

    #[test]
    fn writer_function_calls() {
        let source = r#"
            function callee(a: i32): i32 {
            entry:
                ldarg #a
                ret
            }

            function caller(a: i32): i32 {
            entry:
                ldarg #a
                call callee
                ret
            }
        "#;

        let (_, _, output) = round_trip(source);

        assert!(output.contains("call callee"));
    }

    #[test]
    fn reader_writer_reader() {
        let source = r#"
            function add(a: i32, b: i32): i32 {
                locals [result: i32]

            entry:
                ldarg #a
                ldarg #b
                iadd
                stloc %result
                ldloc %result
                ret
            }
        "#;

        let (first, second, _) = round_trip(source);

        assert_eq!(first, second);
    }

    #[test]
    fn reader_writer_reader_multiple_functions() {
        let source = r#"
            function foo(a: i32): i32 {
            entry:
                ldarg #a
                ret
            }

            function bar(a: i32): i32 {
            entry:
                ldarg #a
                call foo
                ret
            }
        "#;

        let (first, second, _) = round_trip(source);

        assert_eq!(first, second);
    }

    #[test]
    fn reader_writer_reader_control_flow() {
        let source = r#"
            function foo(a: i1): void {
            entry:
                ldarg #a
                brtrue yes
                br no

            yes:
                ret

            no:
                ret
            }
        "#;

        let (first, second, _) = round_trip(source);

        assert_eq!(first, second);
    }
}
