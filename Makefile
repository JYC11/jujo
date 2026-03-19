.PHONY: build test ci install uninstall fmt check

fmt: ## Format code (use CHECK=1 for check-only)
	@if [ -n "$(CHECK)" ]; then \
		cargo fmt --all -- --check; \
	else \
		cargo fmt --all; \
	fi

check: ## Type-check and lint (use CLIPPY=1 for clippy)
	@if [ -n "$(CLIPPY)" ]; then \
		cargo clippy --all-targets -- -D warnings; \
	else \
		cargo check; \
	fi

build: ## Build (use RELEASE=1 for release)
	@if [ -n "$(RELEASE)" ]; then \
		cargo build --release; \
	else \
		cargo build; \
	fi

test: ## Run all tests
	cargo test

ci: ## Run full CI pipeline: fmt check, clippy, tests
	@echo "=== Format check ==="
	cargo fmt --all -- --check
	@echo ""
	@echo "=== Clippy ==="
	cargo clippy --all-targets -- -D warnings
	@echo ""
	@echo "=== Tests ==="
	cargo test

install: ## Build and install to ~/.local/bin (use DEST=/path for custom)
	@INSTALL_DIR="$${DEST:-$$HOME/.local/bin}"; \
	echo "Building jujo (release)..."; \
	cargo build --release; \
	mkdir -p "$$INSTALL_DIR"; \
	cp target/release/jujo "$$INSTALL_DIR/jujo"; \
	chmod +x "$$INSTALL_DIR/jujo"; \
	if [ "$$(uname -s)" = "Darwin" ]; then \
		codesign --sign - --force "$$INSTALL_DIR/jujo" 2>/dev/null || true; \
	fi; \
	echo "Installed jujo to $$INSTALL_DIR/jujo"; \
	if ! echo "$$PATH" | tr ':' '\n' | grep -qx "$$INSTALL_DIR"; then \
		echo ""; \
		echo "Warning: $$INSTALL_DIR is not on your PATH."; \
		echo "Add it with:  export PATH=\"$$INSTALL_DIR:\$$PATH\""; \
	fi

uninstall: ## Remove jujo from ~/.local/bin (use DEST=/path for custom)
	@INSTALL_DIR="$${DEST:-$$HOME/.local/bin}"; \
	TARGET="$$INSTALL_DIR/jujo"; \
	if [ ! -f "$$TARGET" ]; then \
		echo "jujo not found at $$TARGET — nothing to uninstall."; \
		exit 0; \
	fi; \
	rm "$$TARGET"; \
	echo "Removed $$TARGET"; \
	echo ""; \
	echo "Note: existing .jujo/ project directories were NOT removed."; \
	echo "To remove a project's data, delete its .jujo/ directory manually."

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
