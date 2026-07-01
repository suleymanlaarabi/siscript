use si_core::id::FileId;
use si_core::source::SourceFile;
use si_diagnostics::report::DiagnosticReport;
use si_lexer::lexer::lex;
use si_parser::parser::parse_tokens;
use std::fs;
use std::path::Path;

#[test]
fn run_ui_tests() {
    let ui_dir = Path::new("tests/ui");
    if !ui_dir.exists() {
        return;
    }

    let mut failed = 0;
    for entry in fs::read_dir(ui_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "si") && !run_ui_test(&path) {
            failed += 1;
        }
    }

    assert_eq!(failed, 0, "{} UI tests failed", failed);
}

fn run_ui_test(path: &Path) -> bool {
    let source_text = fs::read_to_string(path).unwrap();
    let source = SourceFile::new(FileId::new(1), path.to_str().unwrap(), &source_text);

    let mut report = DiagnosticReport::new();

    let tokens = match lex(&source) {
        Ok(t) => t,
        Err(e) => {
            report.push(si_diagnostics::diagnostic::Diagnostic::new(
                format!("lexer error: {:?}", e.kind),
                e.span,
            ));
            vec![] // Return empty tokens on lex error for now
        }
    };

    if report.is_empty() {
        let result = parse_tokens(tokens);
        for err in result.errors {
            report.push(si_diagnostics::diagnostic::Diagnostic::new(
                format!("parser error: {:?}", err.kind),
                err.span,
            ));
        }
    }

    // Very simple snapshot for now: Just write a .stderr file
    let stderr_path = path.with_extension("stderr");
    let actual_stderr = if report.is_empty() {
        String::new()
    } else {
        // Here we could use ariadne, but for now just basic text
        let mut out = String::new();
        for diag in report.iter() {
            let line =
                source.text[..diag.span.start as usize].chars().filter(|&c| c == '\n').count();
            out.push_str(&format!("{}:{}: {}\n", path.display(), line + 1, diag.message));
        }
        out
    };

    if stderr_path.exists() {
        let expected_stderr = fs::read_to_string(&stderr_path).unwrap();
        if expected_stderr != actual_stderr {
            println!(
                "Mismatch in {}:\nExpected:\n{}\nActual:\n{}",
                path.display(),
                expected_stderr,
                actual_stderr
            );
            return false;
        }
    } else if !actual_stderr.is_empty() {
        // If there are errors but no .stderr, create it
        fs::write(&stderr_path, actual_stderr).unwrap();
    }

    true
}
