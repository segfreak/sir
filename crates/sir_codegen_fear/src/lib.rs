use std::collections::{HashMap, HashSet, VecDeque};

use fear::ssa::{Function as FearFunction, FunctionDef, Module as FearModule};
use fear::types::{
    CallingConvention, CastKind, FloatCmp, FunctionSignature, IntCmp, Linkage, Type as FearType,
};

use sir::*;

pub fn lower_module(module: &Module) -> FearModule {
    let mut fear_module = FearModule::new(module.name.clone());
    let mut function_map = HashMap::new();

    for (sir_id, function) in &module.functions {
        let signature = FunctionSignature::new(
            function.params.iter().copied().map(map_type).collect(),
            map_type(function.return_type),
        );

        let fear_id = fear_module.add_function(FearFunction::declaration(
            &function.name,
            signature,
            Linkage::External,
            CallingConvention::C,
        ));
        function_map.insert(*sir_id, fear_id);
    }

    for (sir_id, function) in &module.functions {
        let fear_id = *function_map
            .get(sir_id)
            .expect("function declaration should have been created");
        let def = lower_function_def(module, function, &function_map);
        fear_module
            .set_definition(fear_id, def)
            .expect("lowered function definition should be valid");
    }

    fear_module
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BlockPoint {
    label: LabelRef,
    instruction: usize,
}

impl BlockPoint {
    fn new(label: LabelRef, instruction: usize) -> Self {
        Self { label, instruction }
    }
}

fn lower_function_def(
    module: &Module,
    function: &Function,
    function_map: &HashMap<FunctionRef, fear::ssa::FuncId>,
) -> FunctionDef {
    let mut def = FunctionDef::new();
    let entry_id = def.get_entry();
    let entry_point = BlockPoint::new(
        function
            .blocks
            .first()
            .map(|block| block.label)
            .unwrap_or(0),
        0,
    );

    for ty in &function.params {
        def.add_param(map_type(*ty));
    }

    let mut slot_ptrs: HashMap<LocalRef, fear::ssa::ValueId> = HashMap::new();
    for local in 0..(function.params.len() + function.locals.len()) as LocalRef {
        let ty = local_type(function, local);
        let ptr = def.make_alloca(entry_id, map_type(ty));
        slot_ptrs.insert(local, ptr);

        if local < function.params.len() as LocalRef {
            let param = def.get_params()[local as usize];
            def.make_store(entry_id, false, ptr, param);
        }
    }

    // The entry block has no operand-stack inputs. Every other block gets
    // block parameters corresponding to the stack values passed by its
    // predecessors.
    let mut block_map = HashMap::new();
    block_map.insert(entry_point, entry_id);

    let mut entry_stack = HashMap::new();
    entry_stack.insert(entry_point, Vec::new());

    // Use an explicit worklist instead of recursive DFS. A block can be
    // reached by a backedge before or after it has been lowered; it must
    // still retain the same block parameters in either case.
    let mut pending = VecDeque::new();
    pending.push_back(entry_point);
    let mut processed = HashSet::new();

    while let Some(point) = pending.pop_front() {
        if !processed.insert(point) {
            continue;
        }

        let incoming = entry_stack
            .get(&point)
            .cloned()
            .expect("target block must have an initialized operand stack");

        lower_block(
            module,
            function,
            &mut def,
            &mut block_map,
            &slot_ptrs,
            function_map,
            point,
            incoming,
            &mut entry_stack,
            &mut pending,
        );
    }

    def
}

fn lower_block(
    module: &Module,
    function: &Function,
    def: &mut FunctionDef,
    block_map: &mut HashMap<BlockPoint, fear::ssa::BlockId>,
    slot_ptrs: &HashMap<LocalRef, fear::ssa::ValueId>,
    function_map: &HashMap<FunctionRef, fear::ssa::FuncId>,
    point: BlockPoint,
    mut stack: Vec<fear::ssa::ValueId>,
    entry_stack: &mut HashMap<BlockPoint, Vec<fear::ssa::ValueId>>,
    pending: &mut VecDeque<BlockPoint>,
) {
    let block_id = *block_map
        .get(&point)
        .expect("block point must be prepared before lowering");
    let source_block = function
        .blocks
        .iter()
        .find(|candidate| candidate.label == point.label)
        .unwrap_or_else(|| {
            panic!(
                "missing label {} while lowering function {}",
                point.label, function.name
            )
        });

    assert!(
        point.instruction <= source_block.instructions.len(),
        "instruction offset {} is out of range for block {} in function {}",
        point.instruction,
        point.label,
        function.name
    );

    let mut terminated = false;

    for (instruction_index, instruction) in source_block
        .instructions
        .iter()
        .enumerate()
        .skip(point.instruction)
    {
        match *instruction {
            Instruction::LoadConstant(value, ty) => {
                stack.push(lower_constant(def, block_id, ty, value));
            }
            Instruction::LoadArg(argno) => {
                stack.push(entry_arg_value(def, argno, function));
            }
            Instruction::LoadLocal(local) => {
                let ptr = slot_ptrs[&local];
                let ty = local_type(function, local);
                stack.push(def.make_load(block_id, false, map_type(ty), ptr));
            }
            Instruction::StoreLocal(local) => {
                let value = stack.pop().expect("stack underflow while storing local");
                let ptr = slot_ptrs[&local];
                def.make_store(block_id, false, ptr, value);
            }
            Instruction::Assert => {
                let _ = stack.pop().expect("stack underflow while asserting");
                log::warn!("ignoring assert instruction");
            }
            Instruction::Goto(target) => {
                let target_point = BlockPoint::new(target, 0);
                let target_id = prepare_point(
                    def,
                    block_map,
                    target_point,
                    &stack,
                    entry_stack,
                );
                def.make_jump(block_id, target_id, stack.clone());
                pending.push_back(target_point);
                terminated = true;
                break;
            }
            Instruction::GotoIfTrue(target) => {
                let cond = stack.pop().expect("stack underflow while branching");
                let false_point = next_instruction_point(function, point.label, instruction_index)
                    .unwrap_or_else(|| {
                        panic!(
                            "conditional branch in block {} has no false successor",
                            point.label
                        )
                    });
                let true_point = BlockPoint::new(target, 0);

                let true_id = prepare_point(
                    def,
                    block_map,
                    true_point,
                    &stack,
                    entry_stack,
                );
                let false_id = prepare_point(
                    def,
                    block_map,
                    false_point,
                    &stack,
                    entry_stack,
                );
                def.make_jumpif(
                    block_id,
                    cond,
                    true_id,
                    stack.clone(),
                    false_id,
                    stack.clone(),
                );

                pending.push_back(true_point);
                pending.push_back(false_point);
                terminated = true;
                break;
            }
            Instruction::GotoIfFalse(target) => {
                let cond = stack.pop().expect("stack underflow while branching");
                let true_point = next_instruction_point(function, point.label, instruction_index)
                    .unwrap_or_else(|| {
                        panic!(
                            "conditional branch in block {} has no true successor",
                            point.label
                        )
                    });
                let false_point = BlockPoint::new(target, 0);

                let true_id = prepare_point(
                    def,
                    block_map,
                    true_point,
                    &stack,
                    entry_stack,
                );
                let false_id = prepare_point(
                    def,
                    block_map,
                    false_point,
                    &stack,
                    entry_stack,
                );
                def.make_jumpif(
                    block_id,
                    cond,
                    true_id,
                    stack.clone(),
                    false_id,
                    stack.clone(),
                );

                pending.push_back(true_point);
                pending.push_back(false_point);
                terminated = true;
                break;
            }
            Instruction::Call(function_ref) => {
                let callee = function_map
                    .get(&function_ref)
                    .copied()
                    .expect("callee function should be declared before lowering callers");

                let callee_function = module
                    .get_function(function_ref)
                    .expect("callee function should exist in source module");
                let arity = callee_function.params.len();
                let mut args = Vec::with_capacity(arity);
                for _ in 0..arity {
                    args.push(stack.pop().expect("stack underflow while calling function"));
                }
                args.reverse();
                let return_type = callee_function.return_type;
                let result = def.make_call(block_id, map_type(return_type), callee, args);
                if return_type != Type::Void {
                    stack.push(result);
                }
            }
            Instruction::Pop => {
                let _ = stack.pop().expect("stack underflow while popping");
            }
            Instruction::Dup => {
                let value = *stack.last().expect("stack underflow while duplicating");
                stack.push(value);
            }
            Instruction::Swp => {
                let rhs = stack.pop().expect("stack underflow while swapping");
                let lhs = stack.pop().expect("stack underflow while swapping");
                stack.push(rhs);
                stack.push(lhs);
            }
            Instruction::Over => {
                let rhs = stack.pop().expect("stack underflow while over");
                let lhs = stack.pop().expect("stack underflow while over");
                stack.push(lhs);
                stack.push(rhs);
                stack.push(lhs);
            }
            Instruction::Nip => {
                let rhs = stack.pop().expect("stack underflow while nip");
                let _ = stack.pop().expect("stack underflow while nip");
                stack.push(rhs);
            }
            Instruction::Tuck => {
                let rhs = stack.pop().expect("stack underflow while tuck");
                let lhs = stack.pop().expect("stack underflow while tuck");
                stack.push(rhs);
                stack.push(lhs);
                stack.push(rhs);
            }
            Instruction::Rot => {
                let c = stack.pop().expect("stack underflow while rot");
                let b = stack.pop().expect("stack underflow while rot");
                let a = stack.pop().expect("stack underflow while rot");
                stack.push(b);
                stack.push(c);
                stack.push(a);
            }
            Instruction::Pick(n) => {
                let depth = n.checked_add(1).expect("pick depth overflow");
                let index = stack
                    .len()
                    .checked_sub(depth)
                    .expect("stack underflow while pick");
                let value = stack[index];
                stack.push(value);
            }
            Instruction::Roll(n) => {
                let depth = n.checked_add(1).expect("roll depth overflow");
                let index = stack
                    .len()
                    .checked_sub(depth)
                    .expect("stack underflow while roll");
                let value = stack.remove(index);
                stack.push(value);
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
                lower_binary_int(def, block_id, &mut stack, *instruction);
            }
            Instruction::IAbs  | Instruction::INeg | Instruction::INot /* | Instruction::ISqrt  */ => {
                lower_unary_int(def, block_id, &mut stack, *instruction);
            }
            Instruction::ICmp(predicate) => {
                let rhs = stack.pop().expect("stack underflow while comparing ints");
                let lhs = stack.pop().expect("stack underflow while comparing ints");
                let result = def.make_int_cmp(block_id, map_int_cmp(predicate), lhs, rhs);
                stack.push(result);
            }
            Instruction::FAdd
            | Instruction::FSub
            | Instruction::FMul
            | Instruction::FDiv
            | Instruction::FRem => {
                lower_binary_float(def, block_id, &mut stack, *instruction);
            }
            Instruction::FAbs | Instruction::FNeg /* | Instruction::FSqrt */  => {
                lower_unary_float(def, block_id, &mut stack, *instruction);
            }
            Instruction::FCmp(predicate) => {
                let rhs = stack.pop().expect("stack underflow while comparing floats");
                let lhs = stack.pop().expect("stack underflow while comparing floats");
                let result = def.make_float_cmp(block_id, map_float_cmp(predicate), lhs, rhs);
                stack.push(result);
            }
            Instruction::Cast(target) => {
                let value = stack.pop().expect("stack underflow while casting");
                let source = def.get_type_of(value);
                let cast_ty = map_type(target);
                // An identity cast is a no-op, not a bitcast instruction.
                let result = if source == cast_ty {
                    value
                } else {
                    def.make_cast(block_id, map_cast_kind(source, cast_ty), cast_ty, value)
                };
                stack.push(result);
            }
            Instruction::Ret => {
                let value = if function.return_type == Type::Void {
                    None
                } else {
                    Some(stack.pop().expect("stack underflow while returning"))
                };
                def.make_ret(block_id, value);
                terminated = true;
                break;
            }
        }
    }

    // A basic block that ends without an explicit terminator falls through
    // to the next source block. Emit an actual CFG edge; merely lowering the
    // next block would leave it disconnected in FEAR.
    if !terminated && let Some(next_label) = next_block_label(function, point.label) {
        let next_point = BlockPoint::new(next_label, 0);
        let next_id = prepare_point(def, block_map, next_point, &stack, entry_stack);
        def.make_jump(block_id, next_id, stack);
        pending.push_back(next_point);
    }
}

fn next_instruction_point(
    function: &Function,
    label: LabelRef,
    instruction_index: usize,
) -> Option<BlockPoint> {
    let block = function
        .blocks
        .iter()
        .find(|candidate| candidate.label == label)?;

    if instruction_index + 1 < block.instructions.len() {
        Some(BlockPoint::new(label, instruction_index + 1))
    } else {
        next_block_label(function, label).map(|next_label| BlockPoint::new(next_label, 0))
    }
}

fn prepare_point(
    def: &mut FunctionDef,
    block_map: &mut HashMap<BlockPoint, fear::ssa::BlockId>,
    point: BlockPoint,
    incoming: &[fear::ssa::ValueId],
    entry_stack: &mut HashMap<BlockPoint, Vec<fear::ssa::ValueId>>,
) -> fear::ssa::BlockId {
    if let Some(block_id) = block_map.get(&point).copied() {
        let params = entry_stack
            .get(&point)
            .expect("prepared block must have an operand-stack signature");
        assert_eq!(
            params.len(),
            incoming.len(),
            "inconsistent operand-stack height at control-flow merge: label {}, instruction {}",
            point.label,
            point.instruction
        );
        for (param, argument) in params.iter().zip(incoming) {
            assert_eq!(
                def.get_type_of(*param),
                def.get_type_of(*argument),
                "inconsistent operand-stack type at control-flow merge: label {}, instruction {}",
                point.label,
                point.instruction
            );
        }
        return block_id;
    }

    let block_id = def.create_block();
    block_map.insert(point, block_id);

    let mut params = Vec::with_capacity(incoming.len());
    for value in incoming {
        let ty = def.get_type_of(*value);
        let param = def.add_block_param(block_id, ty);
        params.push(param);
    }
    entry_stack.insert(point, params);

    block_id
}

fn lower_constant(
    def: &mut FunctionDef,
    block_id: fear::ssa::BlockId,
    ty: Type,
    value: Constant,
) -> fear::ssa::ValueId {
    match value {
        Constant::Int(value) => def.make_int_const(block_id, map_type(ty), value),
        Constant::Float(value) => def.make_float_const(block_id, map_type(ty), value),
    }
}

fn lower_binary_int(
    def: &mut FunctionDef,
    block_id: fear::ssa::BlockId,
    stack: &mut Vec<fear::ssa::ValueId>,
    instruction: Instruction,
) {
    let rhs = stack
        .pop()
        .expect("stack underflow while lowering int binary op");
    let lhs = stack
        .pop()
        .expect("stack underflow while lowering int binary op");
    let ty = def.get_type_of(lhs);
    let value = match instruction {
        Instruction::IAdd => def.make_int_add(block_id, ty, lhs, rhs),
        Instruction::ISub => def.make_int_sub(block_id, ty, lhs, rhs),
        Instruction::IMul => def.make_int_mul(block_id, ty, lhs, rhs),
        Instruction::IDiv => def.make_int_div(block_id, true, ty, lhs, rhs),
        Instruction::UDiv => def.make_int_div(block_id, false, ty, lhs, rhs),
        Instruction::IRem => def.make_int_rem(block_id, true, ty, lhs, rhs),
        Instruction::URem => def.make_int_rem(block_id, false, ty, lhs, rhs),
        Instruction::IAnd => def.make_bitand(block_id, ty, lhs, rhs),
        Instruction::IOr => def.make_bitor(block_id, ty, lhs, rhs),
        Instruction::IXor => def.make_bitxor(block_id, ty, lhs, rhs),
        Instruction::IShl => def.make_lshl(block_id, ty, lhs, rhs),
        Instruction::IShr => def.make_lshr(block_id, ty, lhs, rhs),
        Instruction::ISar => def.make_ashr(block_id, ty, lhs, rhs),
        _ => unreachable!(),
    };
    stack.push(value);
}

fn lower_unary_int(
    def: &mut FunctionDef,
    block_id: fear::ssa::BlockId,
    stack: &mut Vec<fear::ssa::ValueId>,
    instruction: Instruction,
) {
    let value = stack
        .pop()
        .expect("stack underflow while lowering int unary op");
    let ty = def.get_type_of(value);
    let lowered = match instruction {
        Instruction::IAbs => {
            let zero = def.make_int_const(block_id, ty, 0);
            let negated = def.make_int_neg(block_id, ty, value);
            let cond = def.make_int_cmp(block_id, IntCmp::Lt, value, zero);
            def.make_select(block_id, ty, cond, negated, value)
        }
        // Instruction::ISqrt => unimplemented!("SIR integer sqrt is not representable in FEAR SSA"),
        Instruction::INeg => def.make_int_neg(block_id, ty, value),
        Instruction::INot => def.make_bitnot(block_id, ty, value),
        _ => unreachable!(),
    };
    stack.push(lowered);
}

fn lower_binary_float(
    def: &mut FunctionDef,
    block_id: fear::ssa::BlockId,
    stack: &mut Vec<fear::ssa::ValueId>,
    instruction: Instruction,
) {
    let rhs = stack
        .pop()
        .expect("stack underflow while lowering float binary op");
    let lhs = stack
        .pop()
        .expect("stack underflow while lowering float binary op");
    let ty = def.get_type_of(lhs);
    let value = match instruction {
        Instruction::FAdd => def.make_float_add(block_id, ty, lhs, rhs),
        Instruction::FSub => def.make_float_sub(block_id, ty, lhs, rhs),
        Instruction::FMul => def.make_float_mul(block_id, ty, lhs, rhs),
        Instruction::FDiv => def.make_float_div(block_id, ty, lhs, rhs),
        Instruction::FRem => def.make_float_rem(block_id, ty, lhs, rhs),
        _ => unreachable!(),
    };
    stack.push(value);
}

fn lower_unary_float(
    def: &mut FunctionDef,
    block_id: fear::ssa::BlockId,
    stack: &mut Vec<fear::ssa::ValueId>,
    instruction: Instruction,
) {
    let value = stack
        .pop()
        .expect("stack underflow while lowering float unary op");
    let ty = def.get_type_of(value);
    let lowered = match instruction {
        Instruction::FAbs => {
            let zero = def.make_float_const(block_id, ty, 0.0);
            let negated = def.make_float_neg(block_id, ty, value);
            let cond = def.make_float_cmp(block_id, FloatCmp::OLt, value, zero);
            def.make_select(block_id, ty, cond, negated, value)
        }
        // Instruction::FSqrt => unimplemented!("SIR float sqrt is not representable in FEAR SSA"),
        Instruction::FNeg => def.make_float_neg(block_id, ty, value),
        _ => unreachable!(),
    };
    stack.push(lowered);
}

fn map_type(ty: Type) -> FearType {
    match ty {
        Type::Void => FearType::Void,
        Type::Bool => FearType::Int1,
        Type::Int8 => FearType::Int8,
        Type::Int16 => FearType::Int16,
        Type::Int32 => FearType::Int32,
        Type::Int64 => FearType::Int64,
        Type::Float32 => FearType::Float32,
        Type::Float64 => FearType::Float64,
    }
}

fn map_int_cmp(predicate: IntPredicate) -> IntCmp {
    match predicate {
        IntPredicate::Eq => IntCmp::Eq,
        IntPredicate::Ne => IntCmp::Ne,
        IntPredicate::Slt => IntCmp::Lt,
        IntPredicate::Sle => IntCmp::Le,
        IntPredicate::Sgt => IntCmp::Gt,
        IntPredicate::Sge => IntCmp::Ge,
        IntPredicate::Ult => IntCmp::ULt,
        IntPredicate::Ule => IntCmp::ULe,
        IntPredicate::Ugt => IntCmp::UGt,
        IntPredicate::Uge => IntCmp::UGe,
    }
}

fn map_float_cmp(predicate: FloatPredicate) -> FloatCmp {
    match predicate {
        FloatPredicate::Eq => FloatCmp::OEq,
        FloatPredicate::Ne => FloatCmp::ONe,
        FloatPredicate::Olt => FloatCmp::OLt,
        FloatPredicate::Ole => FloatCmp::OLe,
        FloatPredicate::Ogt => FloatCmp::OGt,
        FloatPredicate::Oge => FloatCmp::OGe,
        FloatPredicate::Ult => FloatCmp::ULt,
        FloatPredicate::Ule => FloatCmp::ULe,
        FloatPredicate::Ugt => FloatCmp::UGt,
        FloatPredicate::Uge => FloatCmp::UGe,
    }
}

fn map_cast_kind(source: FearType, target: FearType) -> CastKind {
    match (source, target) {
        (FearType::Int1, FearType::Int8 | FearType::Int16 | FearType::Int32 | FearType::Int64) => {
            CastKind::Zext
        }
        (FearType::Int8, FearType::Int16 | FearType::Int32 | FearType::Int64) => CastKind::Sext,
        (FearType::Int16, FearType::Int32 | FearType::Int64) => CastKind::Sext,
        (FearType::Int32, FearType::Int64) => CastKind::Sext,
        (FearType::Int16, FearType::Int8) => CastKind::Trunc,
        (FearType::Int32, FearType::Int8 | FearType::Int16) => CastKind::Trunc,
        (FearType::Int64, FearType::Int8 | FearType::Int16 | FearType::Int32) => CastKind::Trunc,
        (
            FearType::Int8 | FearType::Int16 | FearType::Int32 | FearType::Int64,
            FearType::Float32 | FearType::Float64,
        ) => CastKind::SIToFP,
        (
            FearType::Float32 | FearType::Float64,
            FearType::Int8 | FearType::Int16 | FearType::Int32 | FearType::Int64,
        ) => CastKind::FPToSI,
        (FearType::Float32, FearType::Float64) => CastKind::FPromote,
        (FearType::Float64, FearType::Float32) => CastKind::FTrunc,
        (_, _) => CastKind::Bitcast,
    }
}

fn entry_arg_value(def: &FunctionDef, argno: usize, function: &Function) -> fear::ssa::ValueId {
    assert!(
        argno < function.params.len(),
        "argument index {argno} out of range in function {}",
        function.name
    );
    def.get_params()[argno]
}

fn local_type(function: &Function, local: LocalRef) -> Type {
    let index = local as usize;
    if index < function.params.len() {
        function.params[index]
    } else {
        let local_index = index - function.params.len();
        function.locals[local_index]
    }
}

fn next_block_label(function: &Function, label: LabelRef) -> Option<LabelRef> {
    let index = function
        .blocks
        .iter()
        .position(|block| block.label == label)?;
    function.blocks.get(index + 1).map(|block| block.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_add_function() {
        let mut module = Module::new("algorithm");
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

        let fear_module = lower_module(&module);
        let fear_func = fear_module
            .lookup_function("add")
            .expect("missing add function");
        let fear_def = fear_module
            .get_function(fear_func)
            .and_then(|f| f.get_definition())
            .expect("missing function definition");

        assert!(!fear_def.get_blocks().is_empty());
    }

    #[test]
    fn splits_conditional_fallthrough_inside_source_block() {
        let mut module = Module::new("cfg");
        module.add_function(
            Function::new("branch", &[Type::Int32], Type::Int32)
                .add_block(BasicBlock::new(0).with_instructions(&[
                    Instruction::LoadArg(0),
                    Instruction::LoadConstant(Constant::Int(0), Type::Int32),
                    Instruction::ICmp(IntPredicate::Sgt),
                    Instruction::GotoIfTrue(1),
                    Instruction::LoadConstant(Constant::Int(0), Type::Int32),
                    Instruction::Ret,
                ]))
                .add_block(BasicBlock::new(1).with_instructions(&[
                    Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                    Instruction::Ret,
                ])),
        );

        let fear_module = lower_module(&module);
        let fear_func = fear_module
            .lookup_function("branch")
            .expect("missing branch");
        let fear_def = fear_module
            .get_function(fear_func)
            .and_then(|f| f.get_definition())
            .expect("missing function definition");

        // Entry + branch target + the instruction-level fallthrough block.
        assert!(fear_def.get_blocks().len() >= 3);
    }

    #[test]
    fn lowers_cfg_backedge_without_recursive_dfs() {
        let mut module = Module::new("cfg");
        module.add_function(
            Function::new("loop", &[Type::Int32], Type::Int32)
                .add_block(BasicBlock::new(0).with_instructions(&[
                    Instruction::LoadArg(0),
                    Instruction::LoadConstant(Constant::Int(0), Type::Int32),
                    Instruction::ICmp(IntPredicate::Eq),
                    Instruction::GotoIfTrue(2),
                    Instruction::Goto(1),
                ]))
                .add_block(BasicBlock::new(1).with_instructions(&[Instruction::Goto(0)]))
                .add_block(BasicBlock::new(2).with_instructions(&[
                    Instruction::LoadConstant(Constant::Int(1), Type::Int32),
                    Instruction::Ret,
                ])),
        );

        let fear_module = lower_module(&module);
        let fear_func = fear_module.lookup_function("loop").expect("missing loop");
        let fear_def = fear_module
            .get_function(fear_func)
            .and_then(|f| f.get_definition())
            .expect("missing function definition");

        // Entry, branch target, fallthrough continuation, and loop body.
        assert!(fear_def.get_blocks().len() >= 4);
    }
}
