# PowerShell script for testing Diesel against a restricted SQLite build (Windows)
# This mimics test_sqlite_no_extensions.sh but adapted for MSVC environment

$ErrorActionPreference = "Stop"

$WorkDir = Get-Location
$BuildDir = Join-Path $WorkDir "build_sqlite_win"
$InstallDir = Join-Path $BuildDir "install"

Write-Host "Using build directory: $BuildDir"
if (-not (Test-Path $BuildDir)) {
    New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
}

Set-Location $BuildDir

$SqliteVersion = "3450100"
$SqliteYear = "2024"
$SqliteSource = "sqlite-amalgamation-$SqliteVersion"
$SqliteZip = "$SqliteSource.zip"

# Download SQLite amalgamation
if (-not (Test-Path $SqliteZip)) {
    Write-Host "Downloading SQLite $SqliteVersion..."
    $Url = "https://sqlite.org/$SqliteYear/$SqliteZip"
    Invoke-WebRequest -Uri $Url -OutFile $SqliteZip
}

if (-not (Test-Path $SqliteSource)) {
    Write-Host "Extracting SQLite..."
    Expand-Archive -Path $SqliteZip -DestinationPath $BuildDir
}

Set-Location $SqliteSource

# Compile SQLite with extension loading disabled
# We assume cl.exe (MSVC) is in the path (run from Developer Command Prompt or similar)
Write-Host "Compiling SQLite (DSQLITE_OMIT_LOAD_EXTENSION)..."

# Using /c (compile only), /O2 (optimize), /DSQLITE_OMIT_LOAD_EXTENSION
cl.exe /c /O2 /DSQLITE_OMIT_LOAD_EXTENSION sqlite3.c

# Create static library
lib.exe /OUT:sqlite3.lib sqlite3.obj

Write-Host "Compilation complete."

# Prepare environment for Cargo
$AbsSqliteDir = (Get-Location).Path

# Set environment variables for libsqlite3-sys
$env:SQLITE3_LIB_DIR = $AbsSqliteDir
$env:SQLITE3_STATIC = "1"
# For runtime check in tests
$env:DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED = "1"

Set-Location $WorkDir

Write-Host "----------------------------------------------------------------"
Write-Host "Running compilation check..."

# Clean to force relink
cargo clean

Write-Host "Running tests against restricted SQLite..."

# We need to ensure we link against system libraries that sqlite3 might depend on if not fully static
# but standard sqlite3.c usually keeps to itself or basics.

try {
    cargo test --package diesel_tests --test integration_tests load_extension --features sqlite
    Write-Host "----------------------------------------------------------------"
    Write-Host "SUCCESS: Diesel compiled and passed tests against SQLite without extension support."
} catch {
    Write-Host "----------------------------------------------------------------"
    Write-Host "FAILURE: Tests failed or compilation failed."
    exit 1
}
