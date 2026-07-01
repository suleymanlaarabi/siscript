#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::fs;
    use std::panic;
    use std::path::{Path, PathBuf};

    use si_ast::item::ItemKind;
    use si_core::id::FileId;
    use si_core::source::SourceFile;
    use si_core::span::Span;
    use si_diagnostics::diagnostic::Diagnostic;
    use si_diagnostics::render::Renderer;
    use si_lexer::lexer::lex;
    use si_parser::parser::{parse, parse_tokens};

    #[test]
    fn workspace_compiles_and_links_core_crates() {
        let source = SourceFile::new(FileId::new(1), "main.si", "fn main() {}");
        let ast = parse(&source.text);
        let diagnostic = Diagnostic::new(String::from("ok"), Span::new(FileId::new(1), 0, 1));

        assert_eq!(ast.items.len(), 1);
        assert!(matches!(ast.items[0].kind, ItemKind::Function(_)));
        assert_eq!(diagnostic.span.start, 0);
        assert_eq!(source.path, "main.si");
        assert_eq!(source.id, FileId::new(1));
    }

    #[test]
    fn valid_scripts_lex_and_parse_without_diagnostics() {
        for path in fixture_files("valid") {
            let source = read_source(&path);
            let tokens = lex(&source).unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
            let result = parse_tokens(tokens);

            assert!(result.errors.is_empty(), "{}: {:?}", path.display(), result.errors);
            assert!(!result.ast.items.is_empty(), "{}: expected AST items", path.display());
        }
    }

    #[test]
    fn invalid_scripts_produce_renderable_diagnostics() {
        for path in fixture_files("invalid") {
            let source = read_source(&path);
            let renderer = Renderer::new();

            match lex(&source) {
                Ok(tokens) => {
                    let result = parse_tokens(tokens);
                    assert!(
                        !result.errors.is_empty(),
                        "{}: expected parser diagnostics",
                        path.display()
                    );
                    for error in result.errors {
                        let diagnostic =
                            Diagnostic::new(format!("parser error: {:?}", error.kind), error.span);
                        let rendered = renderer.render(&source, &diagnostic);
                        assert!(rendered.contains(&source.path));
                        assert!(rendered.contains("parser error"));
                        assert!(rendered.contains('^'));
                    }
                }
                Err(error) => {
                    let diagnostic =
                        Diagnostic::new(format!("lexer error: {:?}", error.kind), error.span);
                    let rendered = renderer.render(&source, &diagnostic);
                    assert!(rendered.contains(&source.path));
                    assert!(rendered.contains("lexer error"));
                    assert!(rendered.contains('^'));
                }
            }
        }
    }

    #[test]
    fn parser_never_panics_on_script_fixtures() {
        for path in all_fixture_files() {
            let source = read_source(&path);
            let Ok(tokens) = lex(&source) else {
                continue;
            };

            let result = panic::catch_unwind(|| parse_tokens(tokens));
            assert!(result.is_ok(), "{}: parser panicked", path.display());
        }
    }

    fn all_fixture_files() -> Vec<PathBuf> {
        let mut files = fixture_files("valid");
        files.extend(fixture_files("invalid"));
        files
    }

    fn fixture_files(kind: &str) -> Vec<PathBuf> {
        let root = fixture_root().join(kind);
        let mut files: Vec<_> = fs::read_dir(&root)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "lang"))
            .collect();
        files.sort();
        files
    }

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripts")
    }

    fn read_source(path: &Path) -> SourceFile {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        SourceFile::new(FileId::new(1), path.display().to_string(), text)
    }
}
