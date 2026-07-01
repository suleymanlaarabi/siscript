.PHONY: install-vscode install-zed install-cli install-all

install-vscode:
	@echo "Installing VS Code Extension..."
	@./scripts/install_vscode_extension.sh

install-zed:
	@echo "Installing Zed Extension..."
	@./scripts/install_zed_extension.sh

install-cli:
	@echo "Installing Siscript CLI..."
	@./scripts/install_cli.sh

install-all: install-cli install-vscode install-zed
	@echo "All installations complete!"
