use si_core::id::FileId;
use si_core::source::SourceFile;
use si_diagnostics::report::DiagnosticReport;
use si_lexer::lexer::lex;
use si_parser::parser::parse_tokens;
use si_resolver::def::DefKind;
use si_resolver::resolved::ResolvedAst;

fn parse_and_resolve(input: &str) -> Result<ResolvedAst<'static>, DiagnosticReport> {
    let source = SourceFile::new(FileId::new(1), "test.si", input);
    let tokens = lex(&source).expect("lexer should accept resolver fixture");
    let parsed = parse_tokens(tokens);
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let ast = Box::leak(Box::new(parsed.ast));
    si_resolver::resolve(ast)
}

fn messages(report: &DiagnosticReport) -> Vec<String> {
    report.iter().map(|diagnostic| diagnostic.message.clone()).collect()
}

#[test]
fn resolve_local_variable() {
    let resolved = parse_and_resolve("fn main() { let x: i32 = 1; x }").unwrap();

    assert!(resolved.resolved_names.len() >= 3);
}

#[test]
fn resolve_shadowing() {
    let resolved =
        parse_and_resolve("fn main() { let x: i32 = 1; if true { let x: i32 = 2; x } }").unwrap();

    let locals = resolved.symbols.defs.iter().filter(|def| def.kind == DefKind::Local).count();
    assert_eq!(locals, 2);
}

#[test]
fn resolve_const() {
    let resolved = parse_and_resolve("const A: i32 = 1; fn main() { A }").unwrap();

    assert!(resolved.symbols.defs.iter().any(|def| def.kind == DefKind::Const));
}

#[test]
fn resolve_fn_call() {
    let resolved = parse_and_resolve("fn f() {} fn main() { f() }").unwrap();

    assert_eq!(resolved.resolved_calls.len(), 1);
}

#[test]
fn resolve_export_fn_call() {
    let resolved = parse_and_resolve("export fn f() {} fn main() { f() }").unwrap();

    let call = resolved.resolved_calls.values().next().copied().unwrap();
    assert_eq!(resolved.symbols.def(call).unwrap().kind, DefKind::ExportFunction);
}

#[test]
fn resolve_extern_fn_call() {
    let resolved = parse_and_resolve("extern fn host(); fn main() { host() }").unwrap();

    let call = resolved.resolved_calls.values().next().copied().unwrap();
    assert_eq!(resolved.symbols.def(call).unwrap().kind, DefKind::ExternFunction);
}

#[test]
fn resolve_struct_name() {
    let resolved =
        parse_and_resolve("struct Point { x: i32 } fn main() { Point { x: 1 } }").unwrap();

    assert!(resolved.symbols.defs.iter().any(|def| def.kind == DefKind::Struct));
}

#[test]
fn resolve_struct_fields() {
    let resolved =
        parse_and_resolve("struct Point { x: i32, y: i32 } fn main() { Point { x: 1, y: 2 } }")
            .unwrap();

    assert_eq!(resolved.resolved_fields.len(), 2);
}

#[test]
fn resolve_enum_variant() {
    let resolved = parse_and_resolve("enum Color { Red, Blue } fn main() { Color::Red }").unwrap();

    assert_eq!(resolved.resolved_variants.len(), 1);
}

#[test]
fn resolve_struct_associated_method() {
    let resolved = parse_and_resolve(
        "struct Point { x: i32, fn make() -> Point { Point { x: 1 } } } fn main() { Point::make() }",
    )
    .unwrap();

    let call = resolved.resolved_calls.values().next().copied().unwrap();
    assert_eq!(resolved.symbols.def(call).unwrap().kind, DefKind::Method);
}

#[test]
fn resolve_self_inside_method() {
    let resolved =
        parse_and_resolve("struct Point { x: i32, fn get(&self) -> i32 { self.x } }").unwrap();

    assert!(resolved.symbols.defs.iter().any(|def| def.kind == DefKind::Local));
    assert_eq!(resolved.resolved_fields.len(), 1);
}

#[test]
fn unknown_variable() {
    let report = parse_and_resolve("fn main() { missing }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0101")));
}

#[test]
fn unknown_function() {
    let report = parse_and_resolve("fn main() { missing() }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0102")));
}

#[test]
fn duplicate_global() {
    let report = parse_and_resolve("const A: i32 = 1; const A: i32 = 2;").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0103")));
}

#[test]
fn duplicate_function_name() {
    let report = parse_and_resolve("fn f() {} extern fn f();").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0103")));
}

#[test]
fn duplicate_struct_field() {
    let report = parse_and_resolve("struct Point { x: i32, x: i32 }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0107")));
}

#[test]
fn duplicate_struct_method() {
    let report = parse_and_resolve(
        "struct Point { fn make() -> Point { Point {} } fn make() -> Point { Point {} } }",
    )
    .unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0103")));
}

#[test]
fn duplicate_enum_variant() {
    let report = parse_and_resolve("enum Color { Red, Red }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0108")));
}

#[test]
fn call_non_callable() {
    let report = parse_and_resolve("const A: i32 = 1; fn main() { A() }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0106")));
}

#[test]
fn unknown_field() {
    let report =
        parse_and_resolve("struct Point { x: i32 } fn main() { Point { y: 1 } }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0104")));
}

#[test]
fn unknown_variant() {
    let report = parse_and_resolve("enum Color { Red } fn main() { Color::Blue }").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0105")));
}

#[test]
fn function_type_name_collision() {
    let report = parse_and_resolve("struct Draw {} fn Draw() {}").unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0109")));
}

#[test]
fn valid_fixture_resolves() {
    let source = include_str!("fixtures/valid/basic.si");

    assert!(parse_and_resolve(source).is_ok());
}

#[test]
fn invalid_fixture_reports_diagnostic() {
    let source = include_str!("fixtures/invalid/unknown_variable.si");
    let report = parse_and_resolve(source).unwrap_err();

    assert!(messages(&report).iter().any(|message| message.starts_with("E0101")));
}
