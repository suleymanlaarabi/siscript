#![forbid(unsafe_code)]

use lsp_types::{CodeActionResponse, Range};

use crate::analysis::AnalysisResult;

pub fn code_actions(_range: Range, _analysis: &AnalysisResult) -> Option<CodeActionResponse> {
    Some(Vec::new())
}
