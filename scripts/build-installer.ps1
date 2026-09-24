# scripts/build-installer.ps1
# Compiles Evergreen Browser and builds the standalone EvergreenBrowserSetup.exe installer.

$ErrorActionPreference = "Stop"

Write-Host "====================================================" -ForegroundColor Cyan
Write-Host "   Building Evergreen Browser Release & Installer   " -ForegroundColor Cyan
Write-Host "====================================================" -ForegroundColor Cyan

# 1. Generate High-DPI icon assets if missing or needed
if (-not (Test-Path "crates/app/ui/icon_64.rgba") -or -not (Test-Path "crates/app/ui/icon.ico")) {
    Write-Host "`n[1/4] Generating High-DPI icons..." -ForegroundColor Yellow
    & .\scripts\generate-high-dpi-icons.ps1
} else {
    Write-Host "`n[1/4] High-DPI icon suite verified." -ForegroundColor Green
}

# Configure path remapping to eliminate local machine/user paths from binary panics and symbols
$userProfileFwd = $env:USERPROFILE.Replace('\', '/')
$userProfileBack = $env:USERPROFILE
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
$repoFwd = $repoRoot.Replace('\', '/')
$env:RUSTFLAGS = "--remap-path-prefix=$userProfileBack=~ --remap-path-prefix=$userProfileFwd=~ --remap-path-prefix=$repoRoot=evergreen-browser --remap-path-prefix=$repoFwd=evergreen-browser"

# 2. Build evergreen-browser release binary
Write-Host "`n[2/4] Compiling evergreen-browser (Release profile)..." -ForegroundColor Yellow
cargo build --release -p evergreen-browser
if ($LASTEXITCODE -ne 0) { throw "evergreen-browser release compilation failed." }

$browserExe = "target/release/evergreen-browser.exe"
if (-not (Test-Path $browserExe)) { throw "Compiled browser binary not found at $browserExe" }
$browserSize = (Get-Item $browserExe).Length
Write-Host "  -> Browser binary size: $([Math]::Round($browserSize / 1MB, 2)) MB ($browserSize bytes)" -ForegroundColor Green

# 3. Stage payload into installer assets
Write-Host "`n[3/4] Staging browser payload into installer assets..." -ForegroundColor Yellow
$installerAssets = "crates/installer/assets"
if (-not (Test-Path $installerAssets)) { New-Item -ItemType Directory -Path $installerAssets -Force | Out-Null }
Copy-Item -Path $browserExe -Destination "$installerAssets/evergreen-browser.exe" -Force

# 4. Compile evergreen-installer release binary
Write-Host "`n[4/4] Compiling evergreen-installer (Release profile)..." -ForegroundColor Yellow
cargo build --release -p evergreen-installer
if ($LASTEXITCODE -ne 0) { throw "evergreen-installer release compilation failed." }

$installerExe = "target/release/evergreen-installer.exe"
if (-not (Test-Path $installerExe)) { throw "Compiled installer binary not found at $installerExe" }

# Assemble dist/ folder
$distDir = "dist"
if (-not (Test-Path $distDir)) { New-Item -ItemType Directory -Path $distDir -Force | Out-Null }

$finalSetup = "$distDir/EvergreenBrowserSetup.exe"
$finalBrowser = "$distDir/evergreen-browser.exe"

Copy-Item -Path $installerExe -Destination $finalSetup -Force
Copy-Item -Path $browserExe -Destination $finalBrowser -Force

$setupSize = (Get-Item $finalSetup).Length

Write-Host "`n====================================================" -ForegroundColor Green
Write-Host "   Build & Packaging Complete Successfully!        " -ForegroundColor Green
Write-Host "====================================================" -ForegroundColor Green
Write-Host "Installer Executable: $finalSetup"
Write-Host "  -> Installer Size:  $([Math]::Round($setupSize / 1MB, 2)) MB ($setupSize bytes)"
Write-Host "Standalone Binary:   $finalBrowser"
Write-Host "  -> Standalone Size: $([Math]::Round($browserSize / 1MB, 2)) MB ($browserSize bytes)"
Write-Host "`nUsers can now download and run EvergreenBrowserSetup.exe to install Evergreen Browser." -ForegroundColor Cyan

