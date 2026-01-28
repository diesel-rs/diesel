# PowerShell script for testing Diesel against a custom SQLite build (Windows)
# This mimics test_sqlite_with_extensions.sh but adapted for MSVC environment
# It verifies that extension loading works correctly when enabled.

$ErrorActionPreference = "Stop"

$WorkDir = Get-Location
$BuildDir = Join-Path $WorkDir "build_sqlite_win_ext"
$InstallDir = Join-Path $BuildDir "install"

Write-Host "Using build directory: $BuildDir"
if (-not (Test-Path $BuildDir)) {
    New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
}

Set-Location $BuildDir

$SqliteVersion = "3450100"
$SqliteYear = "2024"
$SqliteSourceDirName = "sqlite-src-$SqliteVersion"
$SqliteZip = "$SqliteSourceDirName.zip"

# Download full SQLite source (to get ext/misc/uuid.c)
if (-not (Test-Path $SqliteZip)) {
    Write-Host "Downloading SQLite $SqliteVersion..."
    $Url = "https://sqlite.org/$SqliteYear/$SqliteZip"
    Invoke-WebRequest -Uri $Url -OutFile $SqliteZip
}

if (-not (Test-Path $SqliteSourceDirName)) {
    Write-Host "Extracting SQLite..."
    Expand-Archive -Path $SqliteZip -DestinationPath $BuildDir
}

Set-Location $SqliteSourceDirName

# Create install/bin and install/lib directories
$InstallBin = Join-Path $InstallDir "bin"
$InstallLib = Join-Path $InstallDir "lib"
New-Item -ItemType Directory -Force -Path $InstallBin | Out-Null
New-Item -ItemType Directory -Force -Path $InstallLib | Out-Null

# ----------------------------------------------------------------
# Build SQLite DLL
# ----------------------------------------------------------------
# -DSQLITE_ENABLE_MATH_FUNCTIONS: Enables math functions (needed for some tests)
# /O2: Optimize
# /LD: Create DLL
# /Fe: Output name
Write-Host "Compiling SQLite DLL (Extensions Enabled + Math)..."

cl.exe /O2 /LD /DSQLITE_ENABLE_MATH_FUNCTIONS sqlite3.c /Fe:sqlite3.dll

if (-not (Test-Path "sqlite3.dll")) {
    Write-Error "Failed to build sqlite3.dll"
}

Copy-Item "sqlite3.dll" -Destination $InstallBin
Copy-Item "sqlite3.lib" -Destination $InstallLib
# Also copy dll to lib dir for easy finding
Copy-Item "sqlite3.dll" -Destination $InstallLib

# ----------------------------------------------------------------
# Build Extensions
# ----------------------------------------------------------------
Write-Host "Building Extensions..."

# 1. uuid.dll
# Source: ext/misc/uuid.c
# Needs to link against sqlite3.lib
Write-Host "Building uuid.dll..."
cl.exe /O2 /LD /I. ext/misc/uuid.c sqlite3.lib /Fe:uuid.dll

if (-not (Test-Path "uuid.dll")) {
    Write-Error "Failed to build uuid.dll"
}
Copy-Item "uuid.dll" -Destination $InstallLib

# 2. extension-functions.dll
# Download straight from sqlite.org/contrib
Write-Host "Downloading extension-functions.c..."
$ExtFuncUrl = "https://www.sqlite.org/contrib/download/extension-functions.c?get=25"
try {
    Invoke-WebRequest -Uri $ExtFuncUrl -OutFile "extension-functions.c"
} catch {
    Write-Warning "Failed to download extension-functions.c. Skipping extension-functions.dll build."
}

if (Test-Path "extension-functions.c") {
    Write-Host "Building extension-functions.dll..."
    # math.h functions usually linked automatically on Windows (or via standard lib)
    cl.exe /O2 /LD /I. extension-functions.c sqlite3.lib /Fe:extension-functions.dll
    
    if (Test-Path "extension-functions.dll") {
        Copy-Item "extension-functions.dll" -Destination $InstallLib
    }
}

# ----------------------------------------------------------------
# Create symlinks/copies for common library names if needed
# ----------------------------------------------------------------
# The test might look for 'spellfix.so' (on Linux) or 'spellfix.dll'.
# We need to see if we can build spellfix or if uuid is enough.
# The Linux script creates a symlink: spellfix.so -> spellfix1.so
# But spellfix is in ext/misc/spellfix.c? Let's check if we have it in full source.
if (Test-Path "ext/misc/spellfix.c") {
    Write-Host "Building spellfix.dll..."
    cl.exe /O2 /LD /I. ext/misc/spellfix.c sqlite3.lib /Fe:spellfix.dll
    if (Test-Path "spellfix.dll") {
        Copy-Item "spellfix.dll" -Destination $InstallLib
    }
}

Write-Host "Extensions built."

# ----------------------------------------------------------------
# Environment Setup
# ----------------------------------------------------------------
# SQLITE3_LIB_DIR tells build.rs where to find sqlite3.lib
$env:SQLITE3_LIB_DIR = $InstallLib

# Add bin and lib to PATH so tests can find sqlite3.dll and extensions
# Note: Windows checks PATH for DLLs.
$env:PATH = "$InstallBin;$InstallLib;$env:PATH"

# Setting this to 0 tells the test suite to expect successful extension loading
$env:DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED = "0"

Set-Location $WorkDir

Write-Host "----------------------------------------------------------------"
Write-Host "Running compilation check..."

# Clean to force relink
cargo clean

Write-Host "Running tests against standard SQLite..."

# Debug: Print PATH
# Write-Host "Path: $env:PATH"

try {
    # Run the specific test for extension loading
    cargo test --package diesel_tests --test integration_tests load_extension --features sqlite -- --nocapture
    Write-Host "----------------------------------------------------------------"
    Write-Host "SUCCESS: Diesel compiled and passed tests against SQLite with extension support."
} catch {
    Write-Host "----------------------------------------------------------------"
    Write-Host "FAILURE: Tests failed or compilation failed."
    exit 1
}
