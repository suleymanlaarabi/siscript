use zed_extension_api::{self as zed, Result};

struct SiscriptExtension;

impl zed::Extension for SiscriptExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let mut path = "/home/suleyman/.local/bin/si-lsp".to_string();

        if let Ok(settings) =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree)
        {
            if let Some(binary) = settings.binary {
                if let Some(binary_path) = binary.path {
                    path = binary_path;
                }
            }
        }

        if path.is_empty() {
            path = worktree.which("si-lsp").unwrap_or_else(|| "/usr/local/bin/si-lsp".to_string());
        }

        Ok(zed::Command { command: path, args: vec![], env: Default::default() })
    }
}

zed::register_extension!(SiscriptExtension);
