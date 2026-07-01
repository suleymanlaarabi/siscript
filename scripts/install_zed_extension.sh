#!/bin/bash
set -e

# Get workspace root directory
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"

echo "=== 1. Building Siscript LSP Server ==="
cargo build --release
LSP_BIN_PATH="$WORKSPACE_ROOT/target/release/si-lsp"
echo "LSP compiled at: $LSP_BIN_PATH"

echo "=== 2. Installing Zed Language Extension ==="
ZED_EXT_DIR="$HOME/.local/share/zed/extensions/installed/siscript"
rm -rf "$ZED_EXT_DIR"
mkdir -p "$(dirname "$ZED_EXT_DIR")"

echo "=== 1.5 Building Zed Wasm Extension ==="
cd "$WORKSPACE_ROOT/editors/zed"
cargo build --release --target wasm32-wasip2
cp "target/wasm32-wasip2/release/siscript.wasm" "extension.wasm"
cd "$WORKSPACE_ROOT"

# Symlink the local extension directory to Zed extensions folder
ln -sf "$WORKSPACE_ROOT/editors/zed" "$ZED_EXT_DIR"
echo "Symlinked extension to: $ZED_EXT_DIR"

echo "=== 3. Setting LSP Path in Zed Settings ==="
SETTINGS_FILE="$HOME/.config/zed/settings.json"
if [ -f "$SETTINGS_FILE" ] || [ -d "$(dirname "$SETTINGS_FILE")" ]; then
    node -e "
const fs = require('fs');
const file = '$SETTINGS_FILE';
let settings = {};
if (fs.existsSync(file)) {
    try {
        const content = fs.readFileSync(file, 'utf8').trim();
        // Remove trailing commas if any, or just parse if valid JSON
        settings = JSON.parse(content || '{}');
    } catch (e) {
        console.error('Failed to parse settings.json:', e);
    }
}

if (!settings.languages) {
    settings.languages = {};
}
if (!settings.languages.Siscript) {
    settings.languages.Siscript = {};
}
settings.languages.Siscript.language_servers = ['siscript-lsp'];

if (!settings.lsp) {
    settings.lsp = {};
}
settings.lsp['siscript-lsp'] = {
    binary: {
        path: '$LSP_BIN_PATH'
    }
};

fs.writeFileSync(file, JSON.stringify(settings, null, 2), 'utf8');
console.log('Updated siscript-lsp path in Zed settings: ' + file);
"
else
    echo "Zed settings folder not found. You will need to manually configure your zed settings.json."
fi

echo "=== Success! Please restart Zed to use the Siscript extension. ==="
