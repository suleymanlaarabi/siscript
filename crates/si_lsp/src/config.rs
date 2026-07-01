#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspConfig {
    pub diagnostics_on_open: bool,
    pub diagnostics_on_change: bool,
    pub max_file_size: usize,
    pub trace: bool,
    pub hover: bool,
    pub completion: bool,
    pub completion_snippets: bool,
    pub completion_builtins: bool,
    pub completion_keywords: bool,
    pub completion_max_items: usize,
    pub signature_help: bool,
    pub resolve_documentation: bool,
    pub goto_definition: bool,
    pub references: bool,
    pub rename: bool,
    pub semantic_tokens: bool,
    pub symbols: bool,
    pub code_actions: bool,
    pub forbid_abi_rename: bool,
    pub fallback_lexer_tokens: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            diagnostics_on_open: true,
            diagnostics_on_change: true,
            max_file_size: 2 * 1024 * 1024,
            trace: false,
            hover: true,
            completion: true,
            completion_snippets: false,
            completion_builtins: true,
            completion_keywords: true,
            completion_max_items: 100,
            signature_help: true,
            resolve_documentation: false,
            goto_definition: true,
            references: true,
            rename: true,
            semantic_tokens: true,
            symbols: true,
            code_actions: true,
            forbid_abi_rename: true,
            fallback_lexer_tokens: true,
        }
    }
}
