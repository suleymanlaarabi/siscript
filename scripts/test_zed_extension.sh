#!/bin/bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"
GRAMMAR_DIR="$WORKSPACE_ROOT/editors/zed/tree-sitter-siscript"
QUERY_FILE="$WORKSPACE_ROOT/editors/zed/languages/siscript/highlights.scm"
TEST_GRAMMAR_DIR="$(mktemp -d /tmp/tree-sitter-siscript-test-XXXX)"
sample=""
trap 'rm -rf "$TEST_GRAMMAR_DIR" "$sample"' EXIT

tar \
    --exclude='./node_modules' \
    --exclude='./src' \
    -cf - \
    -C "$GRAMMAR_DIR" . | tar -xf - -C "$TEST_GRAMMAR_DIR"

cd "$TEST_GRAMMAR_DIR"
npm install
npm run generate

sample="$(mktemp /tmp/siscript-zed-highlight-XXXX.si)"

cat >"$sample" <<'SISCRIPT'
struct Position {
    x: f32 = 0,
    y: f32 = 0,

    fn default() -> Possition {
        Position {}
    }

    fn with_x(&mut self, value: f32) -> &mut Position {
        self.x = value;
    }
}
SISCRIPT

parse_output="$(npx tree-sitter parse "$sample")"
if grep -q "ERROR" <<<"$parse_output"; then
    echo "$parse_output"
    echo "Zed Siscript grammar produced ERROR nodes for the struct method sample" >&2
    exit 1
fi

query_output="$(npx tree-sitter query "$QUERY_FILE" "$sample")"

require_capture() {
    local capture="$1"
    local text="$2"
    if ! grep -q "capture: .* - $capture, .* text: \`$text\`" <<<"$query_output"; then
        echo "$query_output"
        echo "Missing highlight capture '$capture' for '$text'" >&2
        exit 1
    fi
}

require_capture "keyword" "fn"
require_capture "function" "default"
require_capture "function" "with_x"
require_capture "keyword" "mut"
require_capture "variable.special" "&mut self"
require_capture "type.builtin" "f32"
require_capture "property" "x"
require_capture "operator" "="
