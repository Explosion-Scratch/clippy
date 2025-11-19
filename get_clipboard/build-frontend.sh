#!/bin/bash
set -e

echo "🎨 Building Clippith Dashboard..."
cd frontend-app
npm install
npm run build
echo "✅ Build complete! Output in frontend-dist/"

