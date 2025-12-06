default:
    @just --list

# Install all dependencies
install:
    bun install
    cd get_clipboard/frontend-app && npm install
    cd src-tauri && cargo fetch
    cd get_clipboard && cargo fetch

# Build everything
build:
    bun install
    ./build-sidecar.sh
    bun run tauri build

# Build frontend
build-frontend:
    cd get_clipboard && ~/.config/scripts/hash_if build-frontend frontend-app ./build-frontend.sh

# Build get_clipboard
build-sidecar:
    ~/.config/scripts/hash_if build-sidecar get_clipboard ./build-sidecar.sh

# Restart the get_clipboard service
restart: bundle
    -pkill get_clipboard
    cd get_clipboard && cargo run -- service stop
    cd get_clipboard && cargo run -- service uninstall
    cd get_clipboard && cargo run -- service install
    cd get_clipboard && cargo run -- service start
    sleep 2
    cd get_clipboard && cargo run -- service status

api: bundle
    cd get_clipboard && cargo run -- api

# Run prebuild scripts
bundle: build-frontend build-sidecar

# Launch tauri app in dev mode
dev: bundle
    bun run tauri dev

# Compile/Check templates
compile-templates:
    @echo "Checking templates..."
    # For now, just ensure the directory exists and has files. 
    # In the future, we could run a handlebars validator here.
    ls -la get_clipboard/templates

# Install the application
install-app:
    just build
    cp -r src-tauri/target/release/bundle/macos/Clippy.app /Applications/

# Install the binary
install-binary:
    cd get_clipboard && just install-binary

# Create a GitHub release with version from Cargo.toml
release:
    ./create-release.sh

# Show current versions
version:
    @echo "Clippy App:    v$(grep '^version' src-tauri/Cargo.toml | head -1 | sed 's/version = \"\(.*\)\"/\1/')"
    @echo "get_clipboard: v$(grep '^version' get_clipboard/Cargo.toml | head -1 | sed 's/version = \"\(.*\)\"/\1/')"
