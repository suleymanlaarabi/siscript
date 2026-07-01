use si_core::id::FileId;
use si_core::source::SourceFile;
use si_diagnostics::report::DiagnosticReport;
use si_lexer::lexer::lex;
use si_parser::parser::parse_tokens;
use si_resolver::resolved::ResolvedAst;
use si_typecheck::{check_memory, TypeContext, TypedAst};

fn parse_resolve_check(input: &str) -> Result<(), DiagnosticReport> {
    let source = SourceFile::new(FileId::new(1), "memory.si", input);
    let tokens = lex(&source).expect("lexer should accept memory checker fixture");
    let parsed = parse_tokens(tokens);
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let ast = Box::leak(Box::new(parsed.ast));
    let resolved =
        Box::leak(Box::new(si_resolver::resolve(ast).expect("resolver fixture should resolve")));
    let typed = TypedAst { ast, resolved: resolved as &'static ResolvedAst<'static> };
    check_memory(&TypeContext::new(), &typed).map(|_| ())
}

fn messages(report: &DiagnosticReport) -> Vec<String> {
    report.iter().map(|diagnostic| diagnostic.message.clone()).collect()
}

fn assert_code(input: &str, code: &str) {
    let report = parse_resolve_check(input).unwrap_err();
    assert!(
        messages(&report).iter().any(|message| message.starts_with(code)),
        "{:?}",
        messages(&report)
    );
}

#[test]
fn copy_primitive_reuse() {
    parse_resolve_check("fn main() { let x: i32 = 1; let y: i32 = x; x }").unwrap();
}

#[test]
fn move_str_once() {
    parse_resolve_check("fn main() { let s = \"abc\"; let t = s; }").unwrap();
}

#[test]
fn clone_str_reuse() {
    parse_resolve_check(
        "extern fn clone(x: str) -> str; fn main() { let s = \"abc\"; let t = clone(s); s }",
    )
    .unwrap();
}

#[test]
fn borrow_many_immutable() {
    parse_resolve_check("fn main() { let x: i32 = 1; let a = &x; let b = &x; }").unwrap();
}

#[test]
fn borrow_mut_unique() {
    parse_resolve_check("fn main() { let mut x: i32 = 1; let a = &mut x; }").unwrap();
}

#[test]
fn mutate_let_mut() {
    parse_resolve_check("fn main() { let mut x: i32 = 1; x = 2; }").unwrap();
}

#[test]
fn shadowing_resets_state() {
    parse_resolve_check("fn main() { let s = \"a\"; let t = s; { let s = \"b\"; s } }").unwrap();
}

#[test]
fn pass_ref_to_extern() {
    parse_resolve_check("extern fn host(x: &str); fn main() { let s = \"a\"; host(&s); }").unwrap();
}

#[test]
fn pass_mut_ref_to_extern() {
    parse_resolve_check(
        "extern fn host(x: &mut i32); fn main() { let mut x: i32 = 1; host(&mut x); }",
    )
    .unwrap();
}

#[test]
fn return_owned_move() {
    parse_resolve_check("fn main() -> str { let s = \"a\"; return s; }").unwrap();
}

#[test]
fn use_after_move_str() {
    assert_code("fn main() { let s = \"abc\"; let t = s; s }", "E0300");
}

#[test]
fn use_after_move_array() {
    assert_code("extern fn make() -> [i32]; fn main() { let a = make(); let b = a; a }", "E0300");
}

#[test]
fn use_after_move_struct_with_str() {
    assert_code(
        "struct Box { value: str } fn main() { let b: Box = Box { value: \"a\" }; let c = b; b }",
        "E0300",
    );
}

#[test]
fn mutate_without_mut() {
    assert_code("fn main() { let x: i32 = 1; x = 2; }", "E0410");
}

#[test]
fn mutable_ref_to_immutable() {
    assert_code("fn main() { let x: i32 = 1; let r = &mut x; }", "E0411");
}

#[test]
fn mut_borrow_while_immut_borrowed() {
    assert_code("fn main() { let mut x: i32 = 1; let a = &x; let b = &mut x; }", "E0401");
}

#[test]
fn imm_borrow_while_mut_borrowed() {
    assert_code("fn main() { let mut x: i32 = 1; let a = &mut x; let b = &x; }", "E0402");
}

#[test]
fn two_mut_borrows() {
    assert_code("fn main() { let mut x: i32 = 1; let a = &mut x; let b = &mut x; }", "E0401");
}

#[test]
fn move_while_borrowed() {
    assert_code("fn main() { let s = \"abc\"; let r = &s; let t = s; }", "E0301");
}

#[test]
fn mutate_while_borrowed() {
    assert_code("fn main() { let mut x: i32 = 1; let r = &x; x = 2; }", "E0403");
}

#[test]
fn return_ref_to_local() {
    assert_code("fn main() -> &str { let s = \"abc\"; return &s; }", "E0302");
}

#[test]
fn reference_escapes_export_param() {
    assert_code("export fn f(x: &str) -> &str { return x; }", "E0303");
}

#[test]
fn store_host_ref_in_struct() {
    assert_code(
        "struct Box { value: &str } export fn f(x: &str) { let b: Box = Box { value: x }; }",
        "E0303",
    );
}

#[test]
fn mutate_array_while_element_borrowed() {
    assert_code(
        "extern fn make() -> [i32]; fn main() { let mut a = make(); let e = &a[0]; a = make(); }",
        "E0403",
    );
}
