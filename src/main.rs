use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use sir::interp::{Value, VM};
use sir::sema::verify::verify_module;
use sir::*;
use target_lexicon::Triple;

fn main() {
    pretty_env_logger::try_init().expect("failed to init logger");

    let mut module = Module::new("g");

    module.add_function(
        Function::new("add", &[Type::Int32, Type::Int32], Type::Int32).add_block(
            BasicBlock::new(0).with_instructions(&[
                Instruction::LoadArg(0),
                Instruction::LoadArg(1),
                Instruction::IAdd,
                Instruction::Ret,
            ]),
        ),
    );

    {
        let f = module.function_counter();

        module.set_function(
            f,
            Function::new("fib", &[Type::Int32], Type::Int32)
                .add_block(BasicBlock::new(0).with_instructions(&[
                    Instruction::LoadArg(0),
                    Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                    Instruction::ICmp(IntPredicate::Sle),
                    Instruction::GotoIfTrue(1),
                    Instruction::Goto(2),
                ]))
                .add_block(
                    BasicBlock::new(1)
                        .with_instructions(&[Instruction::LoadArg(0), Instruction::Ret]),
                )
                .add_block(BasicBlock::new(2).with_instructions(&[
                    Instruction::LoadArg(0),
                    Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                    Instruction::ISub,
                    Instruction::Call(f),
                    Instruction::LoadArg(0),
                    Instruction::LoadConstant(Constant::Int(2), Type::Int32),
                    Instruction::ISub,
                    Instruction::Call(f),
                    Instruction::IAdd,
                    Instruction::Ret,
                ])),
        );
    }

    module.add_function(
        Function::new("fibi", &[Type::Int32], Type::Int32)
            .with_locals(&[Type::Int32, Type::Int32, Type::Int32, Type::Int32])
            .add_block(BasicBlock::new(0).with_instructions(&[
                Instruction::LoadArg(0),
                Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                Instruction::ICmp(IntPredicate::Sle),
                Instruction::GotoIfTrue(1),
                Instruction::LoadConstant(Constant::Int(0), Type::Int32),
                Instruction::StoreLocal(1),
                Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                Instruction::StoreLocal(2),
                Instruction::LoadConstant(Constant::Int(2), Type::Int32),
                Instruction::StoreLocal(3),
                Instruction::Goto(2),
            ]))
            .add_block(
                BasicBlock::new(1).with_instructions(&[Instruction::LoadArg(0), Instruction::Ret]),
            )
            .add_block(BasicBlock::new(2).with_instructions(&[
                Instruction::LoadLocal(3),
                Instruction::LoadArg(0),
                Instruction::ICmp(IntPredicate::Sle),
                Instruction::GotoIfFalse(4),
                Instruction::Goto(3),
            ]))
            .add_block(BasicBlock::new(3).with_instructions(&[
                Instruction::LoadLocal(1),
                Instruction::LoadLocal(2),
                Instruction::IAdd,
                Instruction::StoreLocal(4),
                Instruction::LoadLocal(2),
                Instruction::StoreLocal(1),
                Instruction::LoadLocal(4),
                Instruction::StoreLocal(2),
                Instruction::LoadLocal(3),
                Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                Instruction::IAdd,
                Instruction::StoreLocal(3),
                Instruction::Goto(2),
            ]))
            .add_block(
                BasicBlock::new(4)
                    .with_instructions(&[Instruction::LoadLocal(2), Instruction::Ret]),
            ),
    );

    {
        use Instruction::*;
        module.add_function(
            Function::new("min", &[Type::Int32, Type::Int32], Type::Int32)
                .add_block(BasicBlock::new(0).with_instructions(&[
                    LoadArg(0),
                    LoadArg(1),
                    ICmp(IntPredicate::Slt),
                    GotoIfTrue(1),
                    Goto(2),
                ]))
                .add_block(BasicBlock::new(1).with_instructions(&[LoadArg(0), Ret]))
                .add_block(BasicBlock::new(2).with_instructions(&[LoadArg(1), Ret])),
        );
    }

    {
        use Instruction::*;
        module.add_function(
            Function::new("max", &[Type::Int32, Type::Int32], Type::Int32)
                .add_block(BasicBlock::new(0).with_instructions(&[
                    LoadArg(0),
                    LoadArg(1),
                    ICmp(IntPredicate::Sgt),
                    GotoIfTrue(1),
                    Goto(2),
                ]))
                .add_block(BasicBlock::new(1).with_instructions(&[LoadArg(0), Ret]))
                .add_block(BasicBlock::new(2).with_instructions(&[LoadArg(1), Ret])),
        );
    }

    {
        use Instruction::*;
        module.add_function(
            Function::new("iseven", &[Type::Int32], Type::Bool).add_block(
                BasicBlock::new(0).with_instructions(&[
                    LoadArg(0),
                    LoadConstant(Constant::Int(2), Type::Int32),
                    IRem,
                    LoadConstant(Constant::Int(0), Type::Int32),
                    ICmp(IntPredicate::Eq),
                    Ret,
                ]),
            ),
        );
    }

    println!("{}", module);

    let add = module.lookup_function("add").unwrap();
    let fib = module.lookup_function("fib").unwrap();
    let fibi = module.lookup_function("fibi").unwrap();
    let min = module.lookup_function("min").unwrap();
    let max = module.lookup_function("max").unwrap();
    let iseven = module.lookup_function("iseven").unwrap();

    match verify_module(&module) {
        Ok(()) => {
            println!("verification succeeded");
            let mut vm = VM::new(module.clone());
            match vm.invoke(add, &[Value::Int32(20), Value::Int32(22)]) {
                Ok(Some(value)) => println!("add(20, 22) = {value}"),
                Ok(None) => println!("add returned no value"),
                Err(error) => println!("add failed: {error:?}"),
            }
            match vm.invoke(fib, &[Value::Int32(10)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(55));
                    println!("fib(10) = {value}");
                }
                Ok(None) => println!("fib returned no value"),
                Err(error) => println!("fib failed: {error:?}"),
            }
            match vm.invoke(fibi, &[Value::Int32(10)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(55));
                    println!("fibi(10) = {value}");
                }
                Ok(None) => println!("fib returned no value"),
                Err(error) => println!("fib failed: {error:?}"),
            }
            match vm.invoke(fib, &[Value::Int32(11)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(89));
                    println!("fib(11) = {value}");
                }
                Ok(None) => println!("fib returned no value"),
                Err(error) => println!("fib failed: {error:?}"),
            }
            match vm.invoke(fibi, &[Value::Int32(11)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(89));
                    println!("fibi(11) = {value}");
                }
                Ok(None) => println!("fib returned no value"),
                Err(error) => println!("fib failed: {error:?}"),
            }
            match vm.invoke(min, &[Value::Int32(4), Value::Int32(7)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(4));
                    println!("min(4, 7) = {value}");
                }
                Ok(None) => println!("min returned no value"),
                Err(error) => println!("min failed: {error:?}"),
            }
            match vm.invoke(max, &[Value::Int32(4), Value::Int32(7)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Int32(7));
                    println!("max(4, 7) = {value}");
                }
                Ok(None) => println!("max returned no value"),
                Err(error) => println!("max failed: {error:?}"),
            }
            match vm.invoke(iseven, &[Value::Int32(16)]) {
                Ok(Some(value)) => {
                    assert_eq!(value, Value::Bool(true));
                    println!("iseven(16) = {value}");
                }
                _ => todo!("unexpected result for iseven(16)"),
            }
        }
        Err(errors) => {
            println!("verification failed:");

            for error in errors {
                println!(" {error}");
            }
        }
    }

    println!();
    println!();

    let mut fm = module.to_fear();

    if fm.verify().is_err() {
        println!("fear verification failed");
        println!("  note: fix your sir/codegen.rs stupid hotdih");
    }

    println!("{}", fm.dump());
    fear::binary::write_to_file(&fm, Path::new("g.preopt.bin"))
        .expect("failed to write fear module to file");
    println!("wrote fear module to g.preopt.bin");

    println!();
    println!();

    fm.optimize(fear::types::OptLevel::Full, true);

    if fm.verify().is_err() {
        println!("fear postopt verification failed");
        println!("  note: launch debugger motherfucker");
    }

    println!("{}", fm.dump());

    fear::binary::write_to_file(&fm, Path::new("g.bin"))
        .expect("failed to write fear module to file");
    println!("wrote fear module to g.bin");

    {
        let file = File::create("g.o").expect("cannot open g.object file");
        let writer = BufWriter::new(file);
        fear::compiler::compile_module(
            &fm,
            &fear::compiler::CompilerConfig::setup(
                fear::compiler::OutputType::Object,
                Triple::host(),
                fear::types::OptLevel::None,
                true,
                None,
            ),
            writer,
        )
        .expect("cannot compile module");
    }
}
