#!/bin/bash

echo "🧹 Cleaning previous builds..."
rm -rf dist src-tauri/target

echo "🏗️  Building frontend..."
if npx vite build --logLevel warn; then
    echo "✅ Frontend build completed"
else
    echo "❌ Frontend build failed"
    exit 1
fi

echo "📦 Building Tauri app..."
if npx tauri build; then
    echo "✅ Tauri build completed"
    echo "📱 App location: src-tauri/target/release/bundle/macos/Mark-us-Down.app"
    echo "💿 DMG location: src-tauri/target/release/bundle/dmg/"
else
    echo "❌ Tauri build failed"
    exit 1
fi