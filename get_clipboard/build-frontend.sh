#!/bin/bash
set -e

echo "🎨 Building Clippith Dashboard..."
cd frontend-app
bun install
bun run build
echo "✅ Build complete! Output in frontend-dist/"

