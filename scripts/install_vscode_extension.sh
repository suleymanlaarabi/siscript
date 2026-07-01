#!/bin/bash
set -e

# Get workspace root directory
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"

echo "=== 1. Building Siscript LSP Server ==="
cargo build --release
LSP_BIN_PATH="$WORKSPACE_ROOT/target/release/si-lsp"
echo "LSP compiled at: $LSP_BIN_PATH"

echo "=== 2. Compiling VS Code Extension ==="
cd "$WORKSPACE_ROOT/editors/vscode"
npm install
npm run compile

echo "=== 3. Installing Extension to Local VS Code ==="
VSCODE_EXT_DIR="$HOME/.vscode/extensions/siscript.siscript-0.1.0"
rm -rf "$VSCODE_EXT_DIR"
mkdir -p "$VSCODE_EXT_DIR"

# Copy required files to the VS Code extensions folder
cp -r package.json language-configuration.json syntaxes out node_modules "$VSCODE_EXT_DIR/"
echo "Extension installed in: $VSCODE_EXT_DIR"

echo "=== 4. Setting LSP Path in VS Code User Settings ==="
SETTINGS_FILE="$HOME/.config/Code/User/settings.json"
if [ -d "$(dirname "$SETTINGS_FILE")" ]; then
    node -e "
const fs = require('fs');
const file = '$SETTINGS_FILE';
let settings = {};
if (fs.existsSync(file)) {
    try {
        settings = JSON.parse(fs.readFileSync(file, 'utf8'));
    } catch (e) {
        console.error('Failed to parse settings.json:', e);
    }
}
settings['siscript.lsp.path'] = '$LSP_BIN_PATH';
fs.writeFileSync(file, JSON.stringify(settings, null, 2), 'utf8');
console.log('Updated siscript.lsp.path in: ' + file);
"
else
    echo "VS Code settings folder not found. You will need to manually set 'siscript.lsp.path' to: $LSP_BIN_PATH"
fi

echo "=== Success! Please restart/reload VS Code to use the Siscript extension. ==="
