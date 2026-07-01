#![forbid(unsafe_code)]

use lsp_types::{
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureInformation,
};

use crate::analysis::{AnalysisResult, SymbolEntryKind};
use crate::document::Document;

struct BuiltinInfo {
    name: &'static str,
    signature: &'static str,
}

const BUILTINS: &[BuiltinInfo] = &[
    BuiltinInfo { name: "print", signature: "fn print(s: cstr)" },
    BuiltinInfo { name: "println", signature: "fn println(s: cstr)" },
    BuiltinInfo { name: "print_i32", signature: "fn print_i32(v: i32)" },
    BuiltinInfo { name: "println_i32", signature: "fn println_i32(v: i32)" },
    BuiltinInfo { name: "print_f32", signature: "fn print_f32(v: f32)" },
    BuiltinInfo { name: "println_f32", signature: "fn println_f32(v: f32)" },
    BuiltinInfo { name: "print_bool", signature: "fn print_bool(v: bool)" },
    BuiltinInfo { name: "println_bool", signature: "fn println_bool(v: bool)" },
    BuiltinInfo { name: "sqrt_f32", signature: "fn sqrt_f32(x: f32) -> f32" },
    BuiltinInfo { name: "sqrt_f64", signature: "fn sqrt_f64(x: f64) -> f64" },
    BuiltinInfo { name: "sin_f32", signature: "fn sin_f32(x: f32) -> f32" },
    BuiltinInfo { name: "cos_f32", signature: "fn cos_f32(x: f32) -> f32" },
    BuiltinInfo { name: "abs_i32", signature: "fn abs_i32(x: i32) -> i32" },
    BuiltinInfo { name: "abs_f32", signature: "fn abs_f32(x: f32) -> f32" },
    BuiltinInfo { name: "min_i32", signature: "fn min_i32(a: i32, b: i32) -> i32" },
    BuiltinInfo { name: "max_i32", signature: "fn max_i32(a: i32, b: i32) -> i32" },
    BuiltinInfo { name: "min_f32", signature: "fn min_f32(a: f32, b: f32) -> i32" },
    BuiltinInfo { name: "max_f32", signature: "fn max_f32(a: f32, b: f32) -> i32" },
    BuiltinInfo { name: "clone", signature: "fn clone(value: T) -> T" },
];

pub fn signature_help(
    document: &Document,
    position: Position,
    analysis: &AnalysisResult,
) -> Option<SignatureHelp> {
    let offset = document.position_to_offset(position);
    let text = document.text();
    let (callee, commas) = find_active_call(&text, offset)?;

    // Resolve signature
    let signature_str = if let Some(builtin) = BUILTINS.iter().find(|b| b.name == callee) {
        builtin.signature.to_string()
    } else {
        // Search in SymbolIndex
        // The callee can be raw name (like "add") or a path/method like "self.move_by" -> "move_by"
        let search_name = if let Some(dot_idx) = callee.rfind('.') {
            &callee[dot_idx + 1..]
        } else if let Some(colon_idx) = callee.rfind("::") {
            &callee[colon_idx + 2..]
        } else {
            &callee
        };

        let found = analysis.symbol_index.entries.iter().find(|entry| {
            (entry.kind == SymbolEntryKind::Function
                || entry.kind == SymbolEntryKind::ExportFunction
                || entry.kind == SymbolEntryKind::ExternFunction)
                && (entry.name == search_name
                    || entry.name.ends_with(&format!("::{}", search_name)))
        })?;

        found.detail.clone()?
    };

    // Parse parameters from signature_str
    let mut parameters = Vec::new();
    if let Some(open_paren) = signature_str.find('(') {
        if let Some(close_paren) = signature_str.find(')') {
            let params_str = &signature_str[open_paren + 1..close_paren];
            let mut current_offset = open_paren + 1;
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if !trimmed.is_empty() {
                    let param_start =
                        signature_str[current_offset..].find(trimmed).unwrap_or(0) + current_offset;
                    let param_end = param_start + trimmed.len();
                    parameters.push(ParameterInformation {
                        label: ParameterLabel::LabelOffsets([param_start as u32, param_end as u32]),
                        documentation: None,
                    });
                }
                current_offset += param.len() + 1;
            }
        }
    }

    #[allow(deprecated)]
    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: signature_str,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: None,
        }],
        active_signature: Some(0),
        active_parameter: Some(commas as u32),
    })
}

fn find_active_call(text: &str, offset: usize) -> Option<(String, usize)> {
    if offset == 0 || offset > text.len() {
        return None;
    }

    let mut nesting = 0;
    let mut commas = 0;
    let mut start = offset - 1;
    let bytes = text.as_bytes();

    while start > 0 {
        let c = bytes[start];
        if c == b')' {
            nesting += 1;
        } else if c == b'(' {
            nesting -= 1;
            if nesting < 0 {
                break;
            }
        } else if c == b',' && nesting == 0 {
            commas += 1;
        }
        start -= 1;
    }

    if nesting < 0 {
        // We found the opening '(' at start!
        let mut callee_end = start;
        while callee_end > 0 && bytes[callee_end - 1].is_ascii_whitespace() {
            callee_end -= 1;
        }

        let mut callee_start = callee_end;
        while callee_start > 0
            && (bytes[callee_start - 1].is_ascii_alphanumeric()
                || bytes[callee_start - 1] == b'_'
                || bytes[callee_start - 1] == b':'
                || bytes[callee_start - 1] == b'.')
        {
            callee_start -= 1;
        }

        if callee_start < callee_end {
            let callee = text[callee_start..callee_end].trim().to_string();
            return Some((callee, commas));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;

    #[test]
    fn test_signature_help_all_cases() {
        let uri = Url::parse("file:///sig.si").unwrap();
        let text = r#"
extern fn draw(pos: &Position, size: f32);
export fn update(dt: f32) {}
fn add(a: i32, b: i32) -> i32 { return a + b; }
fn main() {
    add(1, 2);
    update(0.1);
    draw(1, 2);
    // test nested
    add(add(1, 2), 3);
}
"#;
        let document = Document::new(uri.clone(), Some(1), text);
        let analysis =
            crate::analysis::analyze_source(&uri, Some(1), std::sync::Arc::new(text.to_string()));

        // 1. simple fn call: add(1, 2);
        // Position of cursor inside add(1, 2) argument list:
        // "    add(1, 2);" -> line 5, char 9 is inside.
        let help = signature_help(&document, Position::new(5, 9), &analysis).unwrap();
        assert!(help.signatures[0].label.contains("fn add(a: i32, b: i32)"));
        assert_eq!(help.active_parameter, Some(0));

        // 2. paramètre actif après virgule: add(1, 2) at char 12 (after comma)
        let help = signature_help(&document, Position::new(5, 12), &analysis).unwrap();
        assert_eq!(help.active_parameter, Some(1));

        // 3. export fn: update(0.1)
        let help = signature_help(&document, Position::new(6, 12), &analysis).unwrap();
        assert!(help.signatures[0].label.contains("export fn update"));

        // 4. extern fn: draw(1, 2)
        let help = signature_help(&document, Position::new(7, 10), &analysis).unwrap();
        assert!(help.signatures[0].label.contains("extern fn draw"));

        // 5. appel imbriqué: add(add(1, 2), 3) -> inside inner add at char 12
        let help = signature_help(&document, Position::new(9, 12), &analysis).unwrap();
        assert_eq!(help.active_parameter, Some(0));

        // 6. fonction inconnue -> None
        assert!(signature_help(&document, Position::new(4, 4), &analysis).is_none());
    }
}
