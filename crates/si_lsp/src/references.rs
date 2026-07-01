#![forbid(unsafe_code)]

use lsp_types::{Location, Position};

use crate::analysis::AnalysisResult;
use crate::document::Document;

pub fn references(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let symbol = crate::analysis::position_to_symbol(document, analysis, position)?;
    let def_id = symbol.def_id?;
    let refs = crate::analysis::def_to_references(document, analysis, def_id, include_declaration);
    Some(refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn references_local() {
        let uri = Url::parse("file:///refs.si").unwrap();
        let text = "fn main() { let x = 1; x; x; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        let refs = references(&document, Position::new(0, 26), &analysis, true).unwrap();
        assert!(refs.len() >= 3);
    }

    #[test]
    fn test_references_all_cases() {
        let uri = Url::parse("file:///refs_cases.si").unwrap();
        let text = r#"
struct Point { x: i32 }
enum EntityType { Player }
fn add(a: i32) {
    let mut my_a = a;
    my_a = my_a + 1;
}
fn main() {
    let p = Point { x: 1 };
    let field_x = p.x;
    let ent = EntityType::Player;
    add(1);
    add(2);
}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        // 1. Local variable: my_a
        let refs_local_with = references(&document, Position::new(5, 4), &analysis, true).unwrap();
        let refs_local_without =
            references(&document, Position::new(5, 4), &analysis, false).unwrap();
        assert_eq!(refs_local_with.len(), 3);
        assert_eq!(refs_local_without.len(), 2);

        // 2. Parameter: a
        let refs_param = references(&document, Position::new(4, 19), &analysis, true).unwrap();
        assert_eq!(refs_param.len(), 2);

        // 3. Function called multiple times: add
        let refs_func = references(&document, Position::new(11, 4), &analysis, true).unwrap();
        assert_eq!(refs_func.len(), 3);

        // 4. Field: x
        let refs_field = references(&document, Position::new(9, 20), &analysis, true).unwrap();
        assert_eq!(refs_field.len(), 3);

        // 5. Variant: Player
        let refs_variant = references(&document, Position::new(10, 26), &analysis, true).unwrap();
        assert_eq!(refs_variant.len(), 2);
    }
}
