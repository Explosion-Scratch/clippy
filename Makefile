.PHONY: install build clean install-binary build-binary

TARGET_TRIPLE := $(shell rustc -vV 2>/dev/null | grep host | cut -d ' ' -f 2)
BINARY_NAME := get_clipboard
APP_NAME := clippy
INSTALL_PREFIX := $(HOME)/.local
BIN_DIR := $(INSTALL_PREFIX)/bin
APP_BUNDLE := src-tauri/target/release/bundle/macos/$(APP_NAME).app
APPLICATIONS_DIR := /Applications

install:
	@echo "Building Tauri app..."
	@bun install
	@./build-sidecar.sh
	@bun run tauri build
	@echo "Installing $(APP_NAME) to $(APPLICATIONS_DIR)..."
	@if [ -d "$(APP_BUNDLE)" ]; then \
		rm -rf "$(APPLICATIONS_DIR)/$(APP_NAME).app"; \
		cp -R "$(APP_BUNDLE)" "$(APPLICATIONS_DIR)/$(APP_NAME).app"; \
		echo "✅ $(APP_NAME) installed to $(APPLICATIONS_DIR)/$(APP_NAME).app"; \
	else \
		echo "❌ Error: App bundle not found at $(APP_BUNDLE)"; \
		echo "   Please check src-tauri/target/release/bundle/ for the built app"; \
		exit 1; \
	fi

install-binary: build-binary
	@echo "Installing $(BINARY_NAME) to $(BIN_DIR)..."
	@mkdir -p $(BIN_DIR)
	@cp get_clipboard/target/release/$(BINARY_NAME) $(BIN_DIR)/$(BINARY_NAME)
	@chmod +x $(BIN_DIR)/$(BINARY_NAME)
	@echo "✅ $(BINARY_NAME) installed to $(BIN_DIR)/$(BINARY_NAME)"

build:
	@echo "Building Tauri app..."
	@bun install
	@./build-sidecar.sh
	@bun run tauri build
	@echo "✅ $(APP_NAME).app built at $(APP_BUNDLE)"

build-binary:
	@echo "Building $(BINARY_NAME)..."
	@cd get_clipboard && ./build-frontend.sh
	@cd get_clipboard && cargo build --release
	@echo "✅ $(BINARY_NAME) built successfully"

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf src-tauri/target
	@rm -rf get_clipboard/target
	@rm -rf dist
	@rm -rf node_modules
	@echo "✅ Clean complete"
