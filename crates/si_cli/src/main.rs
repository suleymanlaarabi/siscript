use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use si_core::id::FileId;
use si_core::source::SourceFile;
use si_diagnostics::diagnostic::Diagnostic;
use si_diagnostics::render::Renderer;
use si_lexer::lexer::lex;
use si_lowering::compile_to_bytecode;
use si_parser::parser::parse_tokens;
use si_resolver::resolve;
use si_typecheck::{TypeContext, TypedAst, check_memory};
use si_vm::Value;
use si_vm::Vm;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    let command = parse_args(args)?;
    match command {
        Command::Check { path, dump_ast } => check_file(path, dump_ast),
        Command::Run { path } => run_file(path),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Check { path: PathBuf, dump_ast: bool },
    Run { path: PathBuf },
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(usage());
    };

    match command.as_str() {
        "check" => {
            let mut dump_ast = false;
            let mut path = None;
            for arg in args {
                if arg == "--dump-ast" {
                    dump_ast = true;
                } else if path.is_none() {
                    path = Some(PathBuf::from(arg));
                } else {
                    return Err(usage());
                }
            }

            let Some(path) = path else {
                return Err(usage());
            };

            Ok(Command::Check { path, dump_ast })
        }
        "run" => {
            let mut path = None;
            for arg in args {
                if path.is_none() {
                    path = Some(PathBuf::from(arg));
                } else {
                    return Err(usage());
                }
            }

            let Some(path) = path else {
                return Err(usage());
            };

            Ok(Command::Run { path })
        }
        _ => Err(usage()),
    }
}

fn check_file(path: PathBuf, dump_ast: bool) -> Result<(), String> {
    let (_, result) = parse_file(&path, "check")?;

    if dump_ast {
        println!("{:#?}", result.ast);
    }

    Ok(())
}

fn run_file(path: PathBuf) -> Result<(), String> {
    let (_, parsed) = parse_file(&path, "run")?;

    let resolved = resolve(&parsed.ast).map_err(|report| format!("run failed: {report:?}"))?;
    let typed = TypedAst { ast: &parsed.ast, resolved: &resolved };
    let ctx = TypeContext::new();
    let checked = check_memory(&ctx, &typed).map_err(|report| format!("run failed: {report:?}"))?;
    let module =
        compile_to_bytecode(&checked, &ctx).map_err(|report| format!("run failed: {report:?}"))?;
    let mut vm = Vm::new(module);

    let _ = vm.register_extern("print", |_: Vec<Value>| {
        println!("ok");
        Ok(Value::Void)
    });
    let value =
        vm.call_function("main", Vec::new()).map_err(|err| format!("runtime error: {err:?}"))?;

    println!("{value:?}");
    Ok(())
}

fn parse_file(
    path: &PathBuf,
    action: &str,
) -> Result<(SourceFile, si_parser::parser::ParseResult), String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let source = SourceFile::new(FileId::new(1), path.display().to_string(), text);
    let renderer = Renderer::new();

    let tokens = match lex(&source) {
        Ok(tokens) => tokens,
        Err(err) => {
            let diagnostic = Diagnostic::new(format!("lexer error: {:?}", err.kind), err.span);
            eprint!("{}", renderer.render(&source, &diagnostic));
            return Err(format!("{action} failed"));
        }
    };

    let result = parse_tokens(tokens);
    if !result.errors.is_empty() {
        for error in result.errors {
            let diagnostic = Diagnostic::new(format!("parser error: {:?}", error.kind), error.span);
            eprint!("{}", renderer.render(&source, &diagnostic));
        }
        return Err(format!("{action} failed"));
    }

    Ok((source, result))
}

fn usage() -> String {
    "usage: si check [--dump-ast] <file.si>\n       si run <file.si>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_check_command() {
        let command = parse_args(["check".to_string(), "main.si".to_string()]).unwrap();

        assert_eq!(command, Command::Check { path: PathBuf::from("main.si"), dump_ast: false });
    }

    #[test]
    fn parses_dump_ast_flag() {
        let command =
            parse_args(["check".to_string(), "--dump-ast".to_string(), "main.si".to_string()])
                .unwrap();

        assert_eq!(command, Command::Check { path: PathBuf::from("main.si"), dump_ast: true });
    }

    #[test]
    fn parses_run_command() {
        let command = parse_args(["run".to_string(), "main.si".to_string()]).unwrap();

        assert_eq!(command, Command::Run { path: PathBuf::from("main.si") });
    }

    #[test]
    fn check_file_accepts_valid_source() {
        let path = temp_file_path("valid.si");
        fs::write(&path, "fn main() { let x: i32 = 1; }\n").unwrap();

        let result = check_file(path.clone(), false);
        let _ = fs::remove_file(path);

        assert!(result.is_ok());
    }

    #[test]
    fn check_file_rejects_invalid_source() {
        let path = temp_file_path("invalid.si");
        fs::write(&path, "123 fn main() {}\n").unwrap();

        let result = check_file(path.clone(), false);
        let _ = fs::remove_file(path);

        assert!(result.is_err());
    }

    #[test]
    fn run_file_executes_main() {
        let path = temp_file_path("run.si");
        fs::write(&path, "fn main() -> i32 { 7 }\n").unwrap();

        let result = run_file(path.clone());
        let _ = fs::remove_file(path);

        assert!(result.is_ok());
    }

    #[test]
    fn run_file_rejects_missing_main() {
        let path = temp_file_path("missing-main.si");
        fs::write(&path, "fn answer() -> i32 { 7 }\n").unwrap();

        let result = run_file(path.clone());
        let _ = fs::remove_file(path);

        assert!(result.unwrap_err().contains("UnknownFunction"));
    }

    #[test]
    fn run_file_reports_missing_extern() {
        let path = temp_file_path("missing-extern.si");
        fs::write(&path, "extern fn host() -> i32; fn main() -> i32 { host() }\n").unwrap();

        let result = run_file(path.clone());
        let _ = fs::remove_file(path);

        assert!(result.unwrap_err().contains("MissingExtern"));
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_args(["build".to_string(), "main.si".to_string()]).is_err());
    }

    fn temp_file_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        env::temp_dir().join(format!("siscript-{nanos}-{name}"))
    }
}
