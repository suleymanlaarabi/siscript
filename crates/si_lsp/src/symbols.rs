#![forbid(unsafe_code)]

use lsp_types::DocumentSymbolResponse;

use crate::analysis::AnalysisResult;
use crate::document::Document;

pub fn document_symbols(
    document: &Document,
    analysis: &AnalysisResult,
) -> Option<DocumentSymbolResponse> {
    let symbols = crate::analysis::document_symbols(document, analysis);
    Some(DocumentSymbolResponse::Nested(symbols))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;
    use lsp_types::Url;

    #[test]
    fn document_symbols_from_ast_even_with_type_errors() {
        let uri = Url::parse("file:///symbols.si").unwrap();
        let text = "struct Point { x: i32 }\nfn main() { unknown; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        let symbols = document_symbols(&document, &analysis).unwrap();
        assert!(format!("{symbols:?}").contains("Point"));
        assert!(format!("{symbols:?}").contains("main"));
    }

    #[test]
    fn test_document_symbols_all_kinds() {
        let uri = Url::parse("file:///symbols_cases.si").unwrap();
        let text = r#"
struct Position { x: f32, y: f32 }
enum EntityType { Player, Enemy }
const MY_CONST: i32 = 42;
type Kilometers = i32;
extern fn draw();
export fn update();
fn add() {}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));
        let symbols = document_symbols(&document, &analysis).unwrap();
        let sym_str = format!("{symbols:?}");

        assert!(sym_str.contains("Position"));
        assert!(sym_str.contains("EntityType"));
        assert!(sym_str.contains("MY_CONST"));
        assert!(sym_str.contains("Kilometers"));
        assert!(sym_str.contains("draw"));
        assert!(sym_str.contains("update"));
        assert!(sym_str.contains("add"));
    }

    #[test]
    fn test_workspace_symbols_all() {
        let workspace = Workspace::new();
        let uri1 = Url::parse("file:///a.si").unwrap();
        let uri2 = Url::parse("file:///b.si").unwrap();

        let text1 = "struct Position { x: f32 }";
        let text2 = "fn add_vals() {}";

        workspace.open(uri1.clone(), text1.to_string(), Some(1));
        workspace.open(uri2.clone(), text2.to_string(), Some(1));

        let analysis1 =
            crate::analysis::analyze_source(&uri1, Some(1), std::sync::Arc::new(text1.to_string()));
        let analysis2 =
            crate::analysis::analyze_source(&uri2, Some(1), std::sync::Arc::new(text2.to_string()));

        workspace.set_analysis(uri1.clone(), std::sync::Arc::new(analysis1));
        workspace.set_analysis(uri2.clone(), std::sync::Arc::new(analysis2));

        // 1. Documents ouverts
        let syms_all = crate::analysis::workspace_symbols(&workspace, "");
        assert_eq!(syms_all.len(), 2);

        // 2. Query exacte
        let syms_exact = crate::analysis::workspace_symbols(&workspace, "Position");
        assert_eq!(syms_exact.len(), 1);
        assert_eq!(syms_exact[0].name, "Position");

        // 3. Query partielle
        let syms_part = crate::analysis::workspace_symbols(&workspace, "add");
        assert_eq!(syms_part.len(), 1);
        assert_eq!(syms_part[0].name, "add_vals");

        // 4. Résultat vide
        let syms_empty = crate::analysis::workspace_symbols(&workspace, "NonExistent");
        assert_eq!(syms_empty.len(), 0);
    }
}
