#!/bin/bash
set -e

# Get workspace root directory
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"

echo "=== Building Siscript CLI ==="
cargo build --release --bin si

CLI_BIN_PATH="$WORKSPACE_ROOT/target/release/si"

# Target installation path
INSTALL_DIR="/usr/local/bin"
INSTALL_PATH="$INSTALL_DIR/siscript"

echo "=== Installing to $INSTALL_PATH ==="
if [ -w "$INSTALL_DIR" ]; then
    cp "$CLI_BIN_PATH" "$INSTALL_PATH"
else
    echo "Requires root permissions to write to $INSTALL_DIR. Running sudo cp..."
    sudo cp "$CLI_BIN_PATH" "$INSTALL_PATH"
fi

echo "=== Success! Siscript CLI installed as 'siscript'. ==="
