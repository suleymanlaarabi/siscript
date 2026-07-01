use si_ast::ty::{PrimitiveType, Type, TypeKind};
use si_bytecode::{BytecodeBuilder, Constant, Instruction, Value};
use si_vm::{Vm, VmError, VmLimits};

fn i32_ty() -> Type {
    Type {
        id: Default::default(),
        kind: TypeKind::Primitive(PrimitiveType::I32),
        span: Default::default(),
    }
}

fn one_function(
    instructions: Vec<Instruction>,
    constants: Vec<Constant>,
) -> si_bytecode::BytecodeModule {
    let mut builder = BytecodeBuilder::new();
    let name = builder.intern("main");
    let id = builder.add_function(name, Vec::new(), i32_ty());
    let function = builder.function_mut(id);
    function.locals_count = 0;
    function.instructions = instructions;
    function.constants = constants;
    builder.finish()
}

#[test]
fn division_by_zero() {
    let module = one_function(
        vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::DivI32,
            Instruction::Return,
        ],
        vec![Constant::I32(1), Constant::I32(0)],
    );
    assert_eq!(Vm::new(module).call_function("main", Vec::new()), Err(VmError::DivisionByZero));
}

#[test]
fn index_out_of_bounds() {
    let module = one_function(
        vec![
            Instruction::Const(0),
            Instruction::MakeArray(1),
            Instruction::Const(1),
            Instruction::LoadIndex,
            Instruction::Return,
        ],
        vec![Constant::I32(10), Constant::I32(2)],
    );
    assert_eq!(Vm::new(module).call_function("main", Vec::new()), Err(VmError::IndexOutOfBounds));
}

#[test]
fn stack_limit() {
    let module = one_function(
        vec![Instruction::Const(0), Instruction::Const(0), Instruction::Return],
        vec![Constant::I32(1)],
    );
    let mut vm = Vm::with_limits(module, VmLimits { max_stack: 1, ..VmLimits::default() });
    assert_eq!(vm.call_function("main", Vec::new()), Err(VmError::StackLimit));
}

#[test]
fn call_wrong_arg_count() {
    let mut builder = BytecodeBuilder::new();
    let name = builder.intern("id");
    let id = builder.add_function(name, vec![i32_ty()], i32_ty());
    builder.function_mut(id).locals_count = 1;
    builder.function_mut(id).instructions = vec![Instruction::LoadLocal(0), Instruction::Return];
    let mut vm = Vm::new(builder.finish());
    assert_eq!(
        vm.call_function("id", Vec::new()),
        Err(VmError::WrongArgCount { expected: 1, actual: 0 })
    );
}

#[test]
fn arithmetic_f32() {
    let module = one_function(
        vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::AddF32,
            Instruction::Return,
        ],
        vec![Constant::F32(1.5), Constant::F32(2.0)],
    );
    assert_eq!(Vm::new(module).call_function("main", Vec::new()), Ok(Value::F32(3.5)));
}

#[test]
fn tuple_access() {
    let module = one_function(
        vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::MakeTuple(2),
            Instruction::LoadField(1),
            Instruction::Return,
        ],
        vec![Constant::I32(3), Constant::I32(6)],
    );
    assert_eq!(Vm::new(module).call_function("main", Vec::new()), Ok(Value::I32(6)));
}
