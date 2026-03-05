$ErrorActionPreference = "Stop"

$Repo      = "launay12u/blazinit"
$BinName   = "blazinit.exe"
$AssetName = "blazinit-x86_64-pc-windows-msvc.exe"
$InstallDir = if ($env:BLAZINIT_INSTALL_DIR) { $env:BLAZINIT_INSTALL_DIR } `
              else { "$env:LOCALAPPDATA\blazinit\bin" }

function Write-Info  { param($m) Write-Host $m -ForegroundColor Cyan }
function Write-Ok    { param($m) Write-Host "✓ $m" -ForegroundColor Green }
function Write-Warn  { param($m) Write-Host "! $m" -ForegroundColor Yellow }

# Fetch latest release
Write-Info "Fetching latest release..."
$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Version = $Release.tag_name
$Asset   = $Release.assets | Where-Object { $_.name -eq $AssetName }

if (-not $Asset) {
    Write-Error "No Windows binary found in release $Version (expected: $AssetName)"
    exit 1
}

Write-Info "Installing blazinit $Version..."

# Create install dir
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Download
$Dest = Join-Path $InstallDir $BinName
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $Dest

# Add to user PATH if missing
$CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$CurrentPath;$InstallDir", "User")
    Write-Warn "Added $InstallDir to PATH — restart your terminal for changes to take effect"
}

Write-Ok "blazinit $Version installed → $Dest"
