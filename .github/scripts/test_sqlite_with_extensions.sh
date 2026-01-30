#!/bin/bash
set -euo pipefail

# This script verifies that Diesel can compile and run against a custom build of SQLite
# that has extension loading enabled.
# It ensures that our extension loading logic works correctly when the underlying
# mechanism is available.

WORKDIR=$(pwd)
BUILD_DIR="${WORKDIR}/build_sqlite_ext"
INSTALL_DIR="${BUILD_DIR}/install"

echo "Using build directory: ${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"
cd "${BUILD_DIR}"

SQLITE_VERSION="3450100"
SQLITE_YEAR="2024"
SQLITE_SOURCE="sqlite-src-${SQLITE_VERSION}"
SQLITE_TAR="${SQLITE_SOURCE}.zip"

# Download full sqlite source (not autoconf) to get access to extension source code
if [ ! -f "${SQLITE_TAR}" ]; then
    echo "Downloading SQLite ${SQLITE_VERSION}..."
    wget -qO ${SQLITE_TAR} "https://sqlite.org/${SQLITE_YEAR}/sqlite-src-${SQLITE_VERSION}.zip"
else
    echo "SQLite source archive found."
fi

if [ ! -d "${SQLITE_SOURCE}" ]; then
    echo "Extracting SQLite..."
    unzip -q "${SQLITE_TAR}"
    # The zip usually extracts to sqlite-src-VERSION
    # If the directory name doesn't match what we expect, or if we need to rename it
    if [ -d "sqlite-src-${SQLITE_VERSION}" ] && [ "sqlite-src-${SQLITE_VERSION}" != "${SQLITE_SOURCE}" ]; then
        mv "sqlite-src-${SQLITE_VERSION}" "${SQLITE_SOURCE}"
    fi
fi

cd "${SQLITE_SOURCE}"

# Configure with extension loading ENABLED
if [ ! -f "Makefile" ]; then
    echo "Configuring SQLite build (Default - Extensions Enabled)..."
    # We include -DSQLITE_ENABLE_MATH_FUNCTIONS to get standard math functions without external deps
    ./configure --prefix="${INSTALL_DIR}" CFLAGS="-O2 -DSQLITE_ENABLE_MATH_FUNCTIONS" CLIBS="-ldl" > /dev/null
fi

echo "Building SQLite..."
make -j$(nproc) > /dev/null
make install > /dev/null

# ----------------------------------------------------------------
# Build Extensions
# ----------------------------------------------------------------
echo "Building Extensions..."
mkdir -p "${INSTALL_DIR}/lib"

# 1. uuid
# Source: ext/misc/uuid.c
# We need to link against the sqlite3 library we just built, or allow undefined symbols if loading into that same process.
# Simpler to compile as a standalone shared object allowing undefined symbols (since they will be resolved by the host executable)
gcc -g -fPIC -shared ext/misc/uuid.c -o "${INSTALL_DIR}/lib/uuid.so"

# 2. extension-functions
# Only compile this if the file exists (it is NOT in standard sqlite source).
# Since we enabled -DSQLITE_ENABLE_MATH_FUNCTIONS above, we might not strictly need this for math,
# but the specific test looks for "extension-functions".
# Since we don't have the source for extension-functions.c in the standard tree, we will skip building it
# and rely on the math functions enabled in core if the test adapts, OR we download it.
# For now, let's download it to satisfy the specific "extension-functions" load test.
wget -qO extension-functions.c "https://www.sqlite.org/contrib/download/extension-functions.c?get=25" || echo "Failed to download extension-functions.c"
if [ -f "extension-functions.c" ]; then
    gcc -g -fPIC -shared extension-functions.c -o "${INSTALL_DIR}/lib/extension-functions.so" -lm
fi

# ----------------------------------------------------------------
# Create symlinks for common variations of library names
# ----------------------------------------------------------------
# Some systems/loaders might look for .so, others might want different names.
# Ensure 'spellfix1' is findable.
cd "${INSTALL_DIR}/lib"
ln -sf spellfix1.so spellfix.so
cd "${WORKDIR}"

# Export environment variables to link against our custom SQLite
export PKG_CONFIG_PATH="${INSTALL_DIR}/lib/pkgconfig"
export LD_LIBRARY_PATH="${INSTALL_DIR}/lib"

# Verify we picked up the right sqlite
echo "Verifying custom sqlite3 version..."
"${INSTALL_DIR}/bin/sqlite3" --version

echo "----------------------------------------------------------------"
echo "Running compilation check..."

# Run cargo test
# We set DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED=0 to tell the test suite 
# that we expect successful loading (or "not authorized" if disabled at runtime but present)
export DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED=0

# Clean first to ensure we link against the new library
cargo clean

echo "Running tests against standard SQLite..."
# Note: --nocapture allows us to see output
if cargo test --package diesel_tests --test integration_tests load_extension --features sqlite -- --nocapture; then
    echo "----------------------------------------------------------------"
    echo "SUCCESS: Diesel compiled and passed tests against SQLite with extension support."
else
    echo "----------------------------------------------------------------"
    echo "FAILURE: Tests failed or compilation failed."
    exit 1
fi
