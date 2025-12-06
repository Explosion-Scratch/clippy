#!/usr/bin/env bash
set -euo pipefail

export PATH="/Users/tjs/.local/share/mise/installs/rust/1.89.0/bin:/Users/tjs/.local/share/mise/installs/bun/latest/bin:/Users/tjs/.local/share/mise/installs/node/latest/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔══════════════════════════════════════════╗"
echo "║        Clippy Release Builder            ║"
echo "╚══════════════════════════════════════════╝"
echo ""

get_cargo_version() {
    local cargo_file="$1"
    grep '^version' "$cargo_file" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

APP_VERSION=$(get_cargo_version "src-tauri/Cargo.toml")
SIDECAR_VERSION=$(get_cargo_version "get_clipboard/Cargo.toml")

echo "📦 Versions:"
echo "   • Clippy App:      v${APP_VERSION}"
echo "   • get_clipboard:   v${SIDECAR_VERSION}"
echo ""

TAURI_VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*"\([0-9.]*\)".*/\1/')
if [ "$TAURI_VERSION" != "$APP_VERSION" ]; then
    echo "⚠️  Warning: tauri.conf.json version ($TAURI_VERSION) differs from Cargo.toml ($APP_VERSION)"
    echo "   Updating tauri.conf.json to match..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/\"version\": \"[0-9.]*\"/\"version\": \"$APP_VERSION\"/" src-tauri/tauri.conf.json
    else
        sed -i "s/\"version\": \"[0-9.]*\"/\"version\": \"$APP_VERSION\"/" src-tauri/tauri.conf.json
    fi
    echo "   ✓ Updated tauri.conf.json"
fi
echo ""

echo "🔨 Building sidecar..."
./build-sidecar.sh

echo ""
echo "🔨 Building Tauri app..."
bun install
bun run tauri build

echo ""
echo "📁 Preparing release artifacts..."
RELEASE_DIR="$SCRIPT_DIR/release"
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

APP_BUNDLE="src-tauri/target/release/bundle/macos/clippy.app"
SIDECAR_BIN="get_clipboard/target/release/get_clipboard"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "❌ Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi

if [ ! -f "$SIDECAR_BIN" ]; then
    echo "❌ Error: Sidecar binary not found at $SIDECAR_BIN"
    exit 1
fi

echo "   • Setting executable permissions..."
chmod +x "$APP_BUNDLE/Contents/MacOS/clippy"
chmod +x "$APP_BUNDLE/Contents/MacOS/get_clipboard"
chmod +x "$SIDECAR_BIN"

echo "   • Creating clippy.app.zip..."
cd "src-tauri/target/release/bundle/macos"
zip -r -y "$RELEASE_DIR/clippy.app.zip" clippy.app
cd "$SCRIPT_DIR"

echo "   • Copying standalone get_clipboard binary..."
cp "$SIDECAR_BIN" "$RELEASE_DIR/get_clipboard"
chmod +x "$RELEASE_DIR/get_clipboard"

echo "   • Verifying artifacts..."
ls -lh "$RELEASE_DIR/"

echo ""
echo "✅ Release artifacts created in: $RELEASE_DIR"
echo ""
echo "   📦 clippy.app.zip  - Full application bundle"
echo "   📦 get_clipboard   - Standalone CLI binary"
echo ""

if command -v gh &> /dev/null; then
    echo "🚀 Creating GitHub release v${APP_VERSION}..."
    
    if gh release view "v${APP_VERSION}" &>/dev/null; then
        echo "   Release v${APP_VERSION} already exists. Uploading assets..."
        gh release upload "v${APP_VERSION}" \
            "$RELEASE_DIR/clippy.app.zip" \
            "$RELEASE_DIR/get_clipboard" \
            --clobber
    else
        gh release create "v${APP_VERSION}" \
            --title "Clippy v${APP_VERSION}" \
            --notes "## Clippy v${APP_VERSION}

### Downloads
- **clippy.app.zip** - macOS application bundle
- **get_clipboard** - Standalone CLI binary

### First-time Setup (macOS)
If you encounter permission errors after downloading:
\`\`\`bash
xattr -cr /Applications/clippy.app
\`\`\`

### Versions
- Clippy App: v${APP_VERSION}
- get_clipboard: v${SIDECAR_VERSION}" \
            "$RELEASE_DIR/clippy.app.zip" \
            "$RELEASE_DIR/get_clipboard"
    fi
    
    echo ""
    echo "✅ GitHub release created successfully!"
    echo "   https://github.com/Explosion-Scratch/clippy/releases/tag/v${APP_VERSION}"
else
    echo "ℹ️  GitHub CLI (gh) not installed."
    echo ""
    echo "To create a GitHub release manually:"
    echo ""
    echo "1. Install gh: brew install gh"
    echo "2. Authenticate: gh auth login"
    echo "3. Run this script again, or manually create:"
    echo ""
    echo "   gh release create \"v${APP_VERSION}\" \\"
    echo "       --title \"Clippy v${APP_VERSION}\" \\"
    echo "       \"$RELEASE_DIR/clippy.app.zip\" \\"
    echo "       \"$RELEASE_DIR/get_clipboard\""
fi

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║            Release Complete!             ║"
echo "╚══════════════════════════════════════════╝"
