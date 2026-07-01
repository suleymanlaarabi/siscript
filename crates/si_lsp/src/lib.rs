#![forbid(unsafe_code)]
#![allow(
    clippy::collapsible_if,
    clippy::field_reassign_with_default,
    clippy::match_like_matches_macro,
    clippy::single_match,
    clippy::unnecessary_map_or
)]

pub mod analysis;
pub mod analysis_index;
pub mod analysis_model;
pub mod backend;
pub mod code_actions;
pub mod completion;
pub mod config;
pub mod diagnostics;
pub mod document;
pub mod document_highlight;
pub mod goto;
pub mod hover;
pub mod references;
pub mod rename;
pub mod semantic_tokens;
pub mod server;
pub mod signature_help;
pub mod symbols;
pub mod workspace;
