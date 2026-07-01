#!/bin/bash
set -e

# Get workspace root directory
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"

echo "=== 1. Building Siscript LSP Server ==="
cargo build --release -p si_lsp --bin si-lsp
mkdir -p "$HOME/.local/bin"
cp "$WORKSPACE_ROOT/target/release/si-lsp" "$HOME/.local/bin/si-lsp"
chmod +x "$HOME/.local/bin/si-lsp"
LSP_BIN_PATH="$HOME/.local/bin/si-lsp"
echo "LSP installed at: $LSP_BIN_PATH"

echo "=== 2. Testing Zed Tree-sitter Grammar ==="
./scripts/test_zed_extension.sh

echo "=== 3. Installing Zed Language Extension ==="
ZED_EXT_DIR="$HOME/.local/share/zed/extensions/installed/siscript"
ZED_SRC_DIR="$WORKSPACE_ROOT/editors/zed"
rm -rf "$ZED_EXT_DIR"
rm -rf "$HOME/.local/share/zed/extensions/work/siscript"
mkdir -p "$ZED_EXT_DIR"

echo "=== 4. Building Zed Wasm Extension ==="
cd "$ZED_SRC_DIR"
cargo build --release --target wasm32-wasip2
cp "target/wasm32-wasip2/release/siscript.wasm" "extension.wasm"
cd "$WORKSPACE_ROOT"

tar \
    --exclude='./target' \
    --exclude='./tree-sitter-siscript/node_modules' \
    --exclude='./tree-sitter-siscript/src' \
    -cf - \
    -C "$ZED_SRC_DIR" . | tar -xf - -C "$ZED_EXT_DIR"

GRAMMAR_DIR="$ZED_EXT_DIR/tree-sitter-siscript"
(
    cd "$GRAMMAR_DIR"
    npm install
    npm run generate
    mkdir -p "$ZED_EXT_DIR/grammars"
    npx tree-sitter build --wasm -o "$ZED_EXT_DIR/grammars/siscript.wasm"
    rm -rf .git
    git init -q
    git add grammar.js package.json package-lock.json tree-sitter.json src
    git -c user.name="Siscript Installer" -c user.email="siscript@example.invalid" commit -q -m "Install Siscript grammar"
)
GRAMMAR_REV="$(git -C "$GRAMMAR_DIR" rev-parse HEAD)"
GRAMMAR_URL="file://$GRAMMAR_DIR"

node -e "
const fs = require('fs');
const file = '$ZED_EXT_DIR/extension.toml';
let toml = fs.readFileSync(file, 'utf8');
toml = toml.replace(
    /\\[grammars\\.siscript\\]\\nrepository = .*\\nrev = .*\\n/,
    '[grammars.siscript]\\nrepository = \"${GRAMMAR_URL}\"\\nrev = \"${GRAMMAR_REV}\"\\n'
);
fs.writeFileSync(file, toml, 'utf8');
"

echo "Installed extension to: $ZED_EXT_DIR"
echo "Installed grammar from: $GRAMMAR_URL@$GRAMMAR_REV"

echo "=== 5. Setting LSP Path in Zed Settings ==="
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
settings.languages.Siscript.semantic_tokens = 'combined';

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

rm -rf "$HOME/.local/share/zed/extensions/work/siscript"
echo "=== Success! Please restart Zed to use the Siscript extension. ==="
