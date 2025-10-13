#!/bin/bash

set -e

echo "🚀 Setting up EventBook WASM Client..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Check if Node.js is available
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install Node.js 18+ first."
    exit 1
fi

# Build WASM module
echo "🔨 Building WASM module..."
cd ../wasm
wasm-pack build --target web --out-dir ../web-client/src/wasm

# Go back to web-client directory
cd ../web-client

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    npm install
fi

echo "✅ Setup complete!"
echo ""
echo "🎯 Next steps:"
echo "  1. Make sure your Rust server is running: cargo run -p eventbook-server"
echo "  2. Start the dev server: npm run dev"
echo "  3. Open http://localhost:5173"
echo ""
echo "🔄 To rebuild WASM: npm run build:wasm"
echo "🚀 To run everything: npm run dev:full"
