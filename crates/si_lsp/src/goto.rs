#![forbid(unsafe_code)]

use lsp_types::{GotoDefinitionResponse, Location, Position};

use crate::analysis::AnalysisResult;
use crate::document::Document;

pub fn goto_definition(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
) -> Option<GotoDefinitionResponse> {
    let symbol = crate::analysis::position_to_symbol(document, analysis, position)?;
    let range = symbol.definition_range?;
    Some(GotoDefinitionResponse::Scalar(Location::new(analysis.uri.clone(), range)))
}

pub fn goto_declaration(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
) -> Option<GotoDefinitionResponse> {
    goto_definition(document, position, analysis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn goto_local_and_function() {
        let uri = Url::parse("file:///goto.si").unwrap();
        let text = "fn add() { return 1; }\nfn main() { let x = add(); x; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        assert!(goto_definition(&document, Position::new(1, 27), &analysis).is_some());
        assert!(goto_definition(&document, Position::new(1, 21), &analysis).is_some());
    }

    #[test]
    fn test_goto_all_cases() {
        let uri = Url::parse("file:///goto_cases.si").unwrap();

        let text = r#"
type Kilometers = i32;
struct Position { x: f32, y: f32 }
enum EntityType { Player = 0, Enemy = 1 }
const MY_CONST: i32 = 42;

extern fn draw(p: &Position);
export fn update(dt: f32) {}
fn add(a: i32, b: i32) -> i32 { return a + b; }

fn main() {
    let mut pos = Position { x: 1.0, y: 2.0 };
    let e = EntityType::Player;
    let dist: Kilometers = 10;
    
    draw(&pos);
    update(0.1);
    add(1, 2);
    let my_x = pos.x;
}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        // 1. Variable locale: pos
        let pos_def = goto_definition(&document, Position::new(14, 9), &analysis).unwrap();
        // 2. Paramètre: a
        let param_def = goto_definition(&document, Position::new(8, 39), &analysis).unwrap();
        // 3. Const: MY_CONST
        let const_def = goto_definition(&document, Position::new(4, 6), &analysis).unwrap();
        // 4. Appel fonction: add
        let call_def = goto_definition(&document, Position::new(17, 4), &analysis).unwrap();
        // 5. Nom struct: Position
        let struct_def = goto_definition(&document, Position::new(11, 22), &analysis).unwrap();
        // 6. Nom enum: EntityType
        let enum_def = goto_definition(&document, Position::new(12, 14), &analysis).unwrap();
        // 7. Field access: x
        let field_def = goto_definition(&document, Position::new(18, 19), &analysis).unwrap();
        // 8. Enum variant: Player
        let variant_def = goto_definition(&document, Position::new(12, 25), &analysis).unwrap();
        // 9. Extern fn: draw
        let extern_def = goto_definition(&document, Position::new(14, 4), &analysis).unwrap();
        // 10. Export fn: update
        let export_def = goto_definition(&document, Position::new(15, 4), &analysis).unwrap();
        // 11. Type alias: Kilometers
        let alias_def = goto_definition(&document, Position::new(13, 16), &analysis).unwrap();

        // Also test goto_declaration
        let call_decl = goto_declaration(&document, Position::new(17, 4), &analysis).unwrap();

        // Assertions:
        assert!(format!("{:?}", pos_def).contains("goto_cases.si"));
        assert!(format!("{:?}", param_def).contains("goto_cases.si"));
        assert!(format!("{:?}", const_def).contains("goto_cases.si"));
        assert!(format!("{:?}", call_def).contains("goto_cases.si"));
        assert!(format!("{:?}", struct_def).contains("goto_cases.si"));
        assert!(format!("{:?}", enum_def).contains("goto_cases.si"));
        assert!(format!("{:?}", field_def).contains("goto_cases.si"));
        assert!(format!("{:?}", variant_def).contains("goto_cases.si"));
        assert!(format!("{:?}", extern_def).contains("goto_cases.si"));
        assert!(format!("{:?}", export_def).contains("goto_cases.si"));
        assert!(format!("{:?}", alias_def).contains("goto_cases.si"));
        assert_eq!(call_def, call_decl);
    }
}
