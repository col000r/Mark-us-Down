#!/bin/bash

echo "🐧 Building Mark-us-Down for Linux..."

# Check for required dependencies
echo "🔍 Checking dependencies..."
deps_missing=false

check_dep() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ Missing: $1"
        deps_missing=true
    else
        echo "✅ Found: $1"
    fi
}

check_dep "rustc"
check_dep "cargo"
check_dep "node"
check_dep "npm"

# Check for system libraries
if ! pkg-config --exists webkit2gtk-4.0; then
    echo "❌ Missing: webkit2gtk-4.0"
    echo "   Install with: sudo apt install libwebkit2gtk-4.0-dev"
    deps_missing=true
fi

if [ "$deps_missing" = true ]; then
    echo "❌ Please install missing dependencies"
    exit 1
fi

echo "🧹 Cleaning previous builds..."
rm -rf dist src-tauri/target

echo "🏗️ Building frontend..."
if npm run build; then
    echo "✅ Frontend build completed"
else
    echo "❌ Frontend build failed"
    exit 1
fi

echo "📦 Building Linux packages..."
if npm run tauri:build:linux; then
    echo "✅ Linux build completed!"
    echo "📍 AppImage: src-tauri/target/release/bundle/appimage/"
    echo "📍 Debian package: src-tauri/target/release/bundle/deb/"
else
    echo "❌ Linux build failed"
    exit 1
fi