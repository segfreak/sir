use sir::interp::{Value, VM};
use sir::sema::verify::verify_module;
use sir::*;

fn main() {
    pretty_env_logger::try_init().expect("failed to init logger");

    let mut module = Module::new("algorithm");

    module.add_function(
        Function::new("add", &[Type::Int32, Type::Int32], Type::Int32).add_block(
            BasicBlock::new(0).with_instructions(&[
                Instruction::LoadLocal(0),
                Instruction::LoadLocal(1),
                Instruction::IAdd,
                Instruction::Ret,
            ]),
        ),
    );

    module.set_function(
        4,
        Function::new("fib", &[Type::Int32], Type::Int32)
            .add_block(BasicBlock::new(0).with_instructions(&[
                Instruction::LoadLocal(0),
                Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                Instruction::ICmp(IntPredicate::Sle),
                Instruction::GotoIfTrue(1),
                Instruction::Goto(2),
            ]))
            .add_block(
                BasicBlock::new(1)
                    .with_instructions(&[Instruction::LoadLocal(0), Instruction::Ret]),
            )
            .add_block(BasicBlock::new(2).with_instructions(&[
                Instruction::LoadLocal(0),
                Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                Instruction::ISub,
                Instruction::Call(4),
                Instruction::LoadLocal(0),
                Instruction::LoadConstant(Constant::Int(2), Type::Int32),
                Instruction::ISub,
                Instruction::Call(4),
                Instruction::IAdd,
                Instruction::Ret,
            ])),
    );

    module.add_function(
        Function::new("fibi", &[Type::Int32], Type::Int32)
            .with_locals(&[Type::Int32, Type::Int32, Type::Int32, Type::Int32])
            .add_block(BasicBlock::new(0).with_instructions(&[
                Instruction::LoadLocal(0),
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
                BasicBlock::new(1)
                    .with_instructions(&[Instruction::LoadLocal(0), Instruction::Ret]),
            )
            .add_block(BasicBlock::new(2).with_instructions(&[
                Instruction::LoadLocal(3),
                Instruction::LoadLocal(0),
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

    println!("{}", module);

    let add = module
        .lookup_function("add")
        .expect("cannot find add function");
    let fib = module
        .lookup_function("fib")
        .expect("cannot find fib function");
    let fibi = module
        .lookup_function("fibi")
        .expect("cannot find fibi function");

    match verify_module(&module) {
        Ok(()) => {
            println!("verification succeeded");
            let mut vm = VM::new(module);
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
        }
        Err(errors) => {
            println!("verification failed:");

            for error in errors {
                println!(" {error}");
            }
        }
    }
}
