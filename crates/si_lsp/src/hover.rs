#![forbid(unsafe_code)]

use lsp_types::{Hover, HoverContents, MarkedString, Position};

use crate::analysis::{AnalysisResult, SymbolEntryKind};
use crate::document::Document;

pub fn hover(document: &Document, position: Position, analysis: &AnalysisResult) -> Option<Hover> {
    let symbol = crate::analysis::position_to_symbol(document, analysis, position)?;
    let text = match symbol.kind {
        SymbolEntryKind::Struct => {
            let mut lines = Vec::new();
            lines.push(format!("struct {}", symbol.name));
            if let Some(ast) = &analysis.ast {
                if let Some(s_item) = crate::analysis::find_struct_item(ast, &symbol.name) {
                    lines.push("fields:".to_string());
                    for field in &s_item.fields {
                        lines.push(format!(
                            "* {}:{}",
                            field.name,
                            crate::analysis::type_to_string(&field.ty)
                        ));
                    }
                }
            }
            let abi = symbol.abi_info.as_deref().unwrap_or("stable");
            lines.push(format!("layout:{}", abi));
            lines.join("\n")
        }
        SymbolEntryKind::Enum => {
            let mut lines = Vec::new();
            lines.push(format!("enum {}:i32", symbol.name));
            if let Some(ast) = &analysis.ast {
                if let Some(e_item) = crate::analysis::find_enum_item(ast, &symbol.name) {
                    lines.push("variants:".to_string());
                    for (i, variant) in e_item.variants.iter().enumerate() {
                        let disc = if let Some(expr) = &variant.discriminant {
                            crate::analysis::expr_to_string(expr)
                        } else {
                            i.to_string()
                        };
                        lines.push(format!("* {}={}", variant.name, disc));
                    }
                }
            }
            lines.join("\n")
        }
        _ => symbol.signature.clone().unwrap_or_default(),
    };

    Some(Hover { contents: HoverContents::Scalar(MarkedString::String(text)), range: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn hover_variable() {
        let uri = Url::parse("file:///hover.si").unwrap();
        let text = "fn main() { let x: i32 = 1; x; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        let hover = hover(&document, Position::new(0, 28), &analysis).unwrap();

        assert!(format!("{:?}", hover.contents).contains("x"));
    }

    #[test]
    fn hover_returns_none_without_info() {
        let uri = Url::parse("file:///hover_invalid.si").unwrap();
        let text = "fn main( {";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        assert!(hover(&document, Position::new(0, 10), &analysis).is_none());
    }

    #[test]
    fn test_hover_all_cases() {
        let uri = Url::parse("file:///hover_cases.si").unwrap();

        // 1. hover variable mut
        let text = "fn main() { let mut pos: i32 = 1; pos; }";
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 34), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("let mut pos: i32"));

        // 2. hover paramètre
        let text = "fn add(a: i32) { a; }";
        let document = Document::new(uri.clone(), Some(2), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(2), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 17), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("a: i32"));

        // 3. hover fn
        let text = "fn add(a: i32, b: i32) -> i32 { return a + b; } fn main() { add(1, 2); }";
        let document = Document::new(uri.clone(), Some(3), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(3), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 60), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("fn add(a: i32, b: i32) -> i32"));

        // 4. hover export fn
        let text = "export fn update(dt: f32) {} fn main() { update(1.0); }";
        let document = Document::new(uri.clone(), Some(4), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(4), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 41), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("export fn update(dt: f32)"));

        // 5. hover extern fn
        let text = "extern fn draw(x: i32); fn main() { draw(1); }";
        let document = Document::new(uri.clone(), Some(5), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(5), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 36), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("extern fn draw(x: i32)"));

        // 6. hover struct layout stable
        let text =
            "struct Position { x: f32, y: f32 } fn main() { let p = Position { x: 1.0, y: 2.0 }; }";
        let document = Document::new(uri.clone(), Some(6), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(6), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 7), &analysis).unwrap();
        let hover_str = format!("{:?}", hover_res.contents);
        assert!(hover_str.contains("struct Position"));
        assert!(hover_str.contains("fields:"));
        assert!(hover_str.contains("* x:f32"));
        assert!(hover_str.contains("* y:f32"));
        assert!(hover_str.contains("layout:stable"));

        // 7. hover enum discriminant
        let text =
            "enum EntityType { Player = 0, Enemy = 1 } fn main() { let e = EntityType::Player; }";
        let document = Document::new(uri.clone(), Some(7), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(7), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 5), &analysis).unwrap();
        let hover_str = format!("{:?}", hover_res.contents);
        assert!(hover_str.contains("enum EntityType:i32"));
        assert!(hover_str.contains("variants:"));
        assert!(hover_str.contains("* Player=0"));
        assert!(hover_str.contains("* Enemy=1"));

        // 8. hover field
        let text = "struct Position { x: f32 } fn main() { let p = Position { x: 1.0 }; p.x; }";
        let document = Document::new(uri.clone(), Some(8), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(8), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 70), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("field x:f32"));

        // 9. hover variant
        let text = "enum EntityType { Player } fn main() { let e = EntityType::Player; }";
        let document = Document::new(uri.clone(), Some(9), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(9), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 60), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("EntityType::Player"));

        // 10. hover type alias
        let text = "type Kilometers = i32; fn main() { let d: Kilometers = 10; }";
        let document = Document::new(uri.clone(), Some(10), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(10), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 5), &analysis).unwrap();
        assert!(format!("{:?}", hover_res.contents).contains("type Kilometers = i32"));

        // 11. hover sur erreur -> None sans panic
        let text = "fn main() { let x = ; }   ";
        let document = Document::new(uri.clone(), Some(11), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(11), std::sync::Arc::new(text.to_string()));
        let hover_res = hover(&document, Position::new(0, 25), &analysis);
        assert!(hover_res.is_none());
    }
}
