use si_bytecode::value::Value;
use si_core::id::FileId;
use si_core::source::SourceFile;
use si_lexer::lexer::lex;
use si_lowering::compile_to_bytecode;
use si_parser::parser::parse_tokens;
use si_resolver::resolve;
use si_typecheck::{TypeContext, TypedAst, check_memory};
use si_vm::{Vm, VmError, VmLimits};

fn compile(source: &str) -> si_bytecode::BytecodeModule {
    let source = SourceFile::new(FileId::new(1), "backend.si", source);
    let tokens = lex(&source).expect("lexer should accept backend fixture");
    let parsed = parse_tokens(tokens);
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let resolved = resolve(&parsed.ast).expect("resolver should accept backend fixture");
    let typed = TypedAst { ast: &parsed.ast, resolved: &resolved };
    let ctx = TypeContext::new();
    let checked = check_memory(&ctx, &typed).expect("memory checker should accept backend fixture");
    compile_to_bytecode(&checked, &ctx).expect("lowering should accept backend fixture")
}

fn run_main(source: &str) -> Value {
    let module = compile(source);
    Vm::new(module).call_function("main", Vec::new()).expect("main should run")
}

#[test]
fn run_return_i32() {
    assert_eq!(run_main("fn main() -> i32 { return 7; }"), Value::I32(7));
}

#[test]
fn local_variables() {
    assert_eq!(run_main("fn main() -> i32 { let x: i32 = 4; x }"), Value::I32(4));
}

#[test]
fn arithmetic_i32() {
    assert_eq!(run_main("fn main() -> i32 { 1 + 2 * 3 }"), Value::I32(7));
}

#[test]
fn if_else() {
    assert_eq!(run_main("fn main() -> i32 { if true { 1 } else { 2 } }"), Value::I32(1));
}

#[test]
fn while_loop() {
    assert_eq!(
        run_main("fn main() -> i32 { let mut x: i32 = 0; while x < 3 { x = x + 1; } x }"),
        Value::I32(3)
    );
}

#[test]
fn function_call() {
    assert_eq!(
        run_main("fn add(a: i32, b: i32) -> i32 { a + b } fn main() -> i32 { add(2, 5) }"),
        Value::I32(7)
    );
}

#[test]
fn struct_init_and_field() {
    assert_eq!(
        run_main(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let p: Point = Point { x: 4, y: 9 }; p.x }"
        ),
        Value::I32(4)
    );
}

#[test]
fn struct_init_uses_default_fields() {
    assert_eq!(
        run_main(
            "struct Point { x: i32 = 4, y: i32 = 9 } fn main() -> i32 { let p: Point = Point {}; p.x }"
        ),
        Value::I32(4)
    );
}

#[test]
fn associated_method_constructor() {
    assert_eq!(
        run_main(
            "struct Point { x: i32 = 0, fn default() -> Point { Point {} } } fn main() -> i32 { let p: Point = Point::default(); p.x }"
        ),
        Value::I32(0)
    );
}

#[test]
fn method_reads_self() {
    assert_eq!(
        run_main(
            "struct Point { x: i32, fn get(&self) -> i32 { self.x } } fn main() -> i32 { let p: Point = Point { x: 5 }; p.get() }"
        ),
        Value::I32(5)
    );
}

#[test]
fn method_mutates_self() {
    assert_eq!(
        run_main(
            "struct Point { x: i32, fn add(&mut self, dx: i32) { self.x = self.x + dx; } } fn main() -> i32 { let mut p: Point = Point { x: 5 }; p.add(3); p.x }"
        ),
        Value::I32(8)
    );
}

#[test]
fn struct_field_assignment() {
    assert_eq!(
        run_main(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let mut p: Point = Point { x: 1, y: 2 }; p.x = 8; p.x }"
        ),
        Value::I32(8)
    );
}

#[test]
fn enum_variant_and_match() {
    assert_eq!(
        run_main(
            "enum Flag { Off, On } fn main() -> i32 { match Flag::On { Flag::Off => 0, Flag::On => 1 } }"
        ),
        Value::I32(1)
    );
}

#[test]
fn tuple_create() {
    assert_eq!(
        run_main("fn main() -> (i32, i32) { (3, 6) }"),
        Value::Tuple(vec![Value::I32(3), Value::I32(6)])
    );
}

#[test]
fn reference_read() {
    assert_eq!(
        run_main(
            "extern fn use_ref(x: &i32) -> i32; fn main() -> i32 { let x: i32 = 3; let r: &i32 = &x; 1 }"
        ),
        Value::I32(1)
    );
}

#[test]
fn export_registered_in_module() {
    let module = compile("export fn answer() -> i32 { 42 }");
    let mut vm = Vm::new(module);
    assert_eq!(vm.call_export("answer", Vec::new()), Ok(Value::I32(42)));
}

#[test]
fn extern_required_in_module_and_missing_at_runtime() {
    let module = compile("extern fn host() -> i32; fn main() -> i32 { host() }");
    assert_eq!(module.extern_signatures.len(), 1);
    let err = Vm::new(module).call_function("main", Vec::new()).unwrap_err();
    assert!(matches!(err, VmError::MissingExtern(_)));
}

#[test]
fn recursion_depth_limit() {
    let module =
        compile("fn loop_forever() -> i32 { loop_forever() } fn main() -> i32 { loop_forever() }");
    let mut vm = Vm::with_limits(
        module,
        VmLimits { max_frames: 8, max_instructions: Some(1_000), ..VmLimits::default() },
    );
    assert_eq!(vm.call_function("main", Vec::new()), Err(VmError::FrameLimit));
}
