# PowerShell script for building on Windows
Write-Host "🏗️ Building Mark-us-Down for Windows..." -ForegroundColor Blue

# Clean previous builds
Write-Host "🧹 Cleaning previous builds..." -ForegroundColor Yellow
Remove-Item -Path "dist" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "src-tauri\target" -Recurse -Force -ErrorAction SilentlyContinue

# Build frontend
Write-Host "📦 Building frontend..." -ForegroundColor Yellow
npm run build

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Frontend build completed" -ForegroundColor Green
    
    # Build Tauri app
    Write-Host "🚀 Building Windows app..." -ForegroundColor Yellow
    npm run tauri:build:windows
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Windows build completed!" -ForegroundColor Green
        Write-Host "📍 MSI installer: src-tauri\target\release\bundle\msi\" -ForegroundColor Cyan
        Write-Host "📍 EXE installer: src-tauri\target\release\bundle\nsis\" -ForegroundColor Cyan
    } else {
        Write-Host "❌ Windows build failed" -ForegroundColor Red
    }
} else {
    Write-Host "❌ Frontend build failed" -ForegroundColor Red
}