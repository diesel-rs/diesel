# PowerShell script for testing Diesel against a restricted SQLite build (Windows)
# This mimics test_sqlite_no_extensions.sh but adapted for MSVC environment

$ErrorActionPreference = "Stop"

$WorkDir = Get-Location
$BuildDir = Join-Path $WorkDir "build_sqlite_win_restricted"
$InstallDir = Join-Path $BuildDir "install"

Write-Host "Using build directory: $BuildDir"
if (-not (Test-Path $BuildDir)) {
    New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
}

Set-Location $BuildDir

$SqliteVersion = "3450100"
$SqliteYear = "2024"
$SqliteAmalgamation = "sqlite-amalgamation-$SqliteVersion"
$SqliteAmalgamationZip = "$SqliteAmalgamation.zip"

# Download SQLite Amalgamation
if (-not (Test-Path $SqliteAmalgamationZip)) {
    Write-Host "Downloading SQLite Amalgamation $SqliteVersion..."
    $Url = "https://sqlite.org/$SqliteYear/$SqliteAmalgamationZip"
    Invoke-WebRequest -Uri $Url -OutFile $SqliteAmalgamationZip
}

if (-not (Test-Path $SqliteAmalgamation)) {
    Write-Host "Extracting SQLite..."
    Expand-Archive -Path $SqliteAmalgamationZip -DestinationPath $BuildDir
}

Set-Location $SqliteAmalgamation

# Create install/bin and install/lib directories
$InstallBin = Join-Path $InstallDir "bin"
$InstallLib = Join-Path $InstallDir "lib"
New-Item -ItemType Directory -Force -Path $InstallBin | Out-Null
New-Item -ItemType Directory -Force -Path $InstallLib | Out-Null

# Compile SQLite with extension loading disabled as a DLL
# We build a DLL so that GetProcAddress logic in diesel can work reliably (or fail reliably)
# /O2 = Optimize
# /LD = Create DLL
# /DSQLITE_OMIT_LOAD_EXTENSION = The feature we are testing
# /Fe: = File executable (output name)
Write-Host "Compiling SQLite DLL (DSQLITE_OMIT_LOAD_EXTENSION)..."

cl.exe /O2 /LD /DSQLITE_OMIT_LOAD_EXTENSION sqlite3.c /Fe:sqlite3.dll

if (-not (Test-Path "sqlite3.dll")) {
    Write-Error "Failed to build sqlite3.dll"
}

# Move artifacts to install locations
Copy-Item "sqlite3.dll" -Destination $InstallBin
Copy-Item "sqlite3.lib" -Destination $InstallLib
# Also copy dll to lib dir for running convenience if needed, though PATH handles it
Copy-Item "sqlite3.dll" -Destination $InstallLib 

Write-Host "Compilation complete."

# Prepare environment for Cargo
# SQLITE3_LIB_DIR tells build.rs where to find sqlite3.lib
$env:SQLITE3_LIB_DIR = $InstallLib

# Add bin to PATH so the test executable can find sqlite3.dll at runtime
$env:PATH = "$InstallBin;$env:PATH"

# Setting this to 1 tells the test suite to expect "no such function: load_extension"
$env:DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED = "1"

Set-Location $WorkDir

Write-Host "----------------------------------------------------------------"
Write-Host "Running compilation check..."

# Clean to force relink with our new library
cargo clean

Write-Host "Running tests against restricted SQLite..."

try {
    # Run the specific test for extension loading
    cargo test --package diesel_tests --test integration_tests load_extension --features sqlite -- --nocapture
    Write-Host "----------------------------------------------------------------"
    Write-Host "SUCCESS: Diesel compiled and passed tests against SQLite without extension support."
} catch {
    Write-Host "----------------------------------------------------------------"
    Write-Host "FAILURE: Tests failed or compilation failed."
    exit 1
}
