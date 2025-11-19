#!/bin/bash

# Exit on error
set -e

# Target triple for the host machine
TARGET_TRIPLE=$(rustc -vV | grep host | cut -d ' ' -f 2)
echo "Detected target triple: $TARGET_TRIPLE"

# Build the frontend first (required for embedding)
echo "Building frontend dashboard..."
cd get_clipboard
./build-frontend.sh
cd ..

# Build the get_clipboard binary
echo "Building get_clipboard..."
cd get_clipboard
cargo build --release
cd ..

# Create the binaries directory if it doesn't exist
mkdir -p src-tauri/binaries

# Copy the binary to the expected location with the target triple suffix
echo "Copying binary to src-tauri/binaries/get_clipboard-$TARGET_TRIPLE"
cp get_clipboard/target/release/get_clipboard "src-tauri/binaries/get_clipboard-$TARGET_TRIPLE"

echo "Build and copy complete!"


