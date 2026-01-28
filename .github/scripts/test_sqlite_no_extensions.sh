#!/bin/bash
set -euo pipefail

# This script verifies that Diesel can compile and run against a custom build of SQLite
# that has extension loading disabled (compiled with -DSQLITE_OMIT_LOAD_EXTENSION).
# It ensures that our runtime check for extension support works correctly and that
# we don't have broken symbol dependencies.

WORKDIR=$(pwd)
BUILD_DIR="${WORKDIR}/build_sqlite"
INSTALL_DIR="${BUILD_DIR}/install"

echo "Using build directory: ${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"
cd "${BUILD_DIR}"

SQLITE_VERSION="3450100"
SQLITE_YEAR="2024"
SQLITE_SOURCE="sqlite-autoconf-${SQLITE_VERSION}"
SQLITE_TAR="${SQLITE_SOURCE}.tar.gz"

# Download sqlite autoconf if not present
if [ ! -f "${SQLITE_TAR}" ]; then
    echo "Downloading SQLite ${SQLITE_VERSION}..."
    wget -q "https://sqlite.org/${SQLITE_YEAR}/${SQLITE_TAR}"
else
    echo "SQLite source archive found."
fi

if [ ! -d "${SQLITE_SOURCE}" ]; then
    echo "Extracting SQLite..."
    tar xzf "${SQLITE_TAR}"
fi

cd "${SQLITE_SOURCE}"

# Configure with extension loading disabled
if [ ! -f "Makefile" ]; then
    echo "Configuring SQLite build (DSQLITE_OMIT_LOAD_EXTENSION)..."
    ./configure --prefix="${INSTALL_DIR}" CFLAGS="-DSQLITE_OMIT_LOAD_EXTENSION" > /dev/null
fi

echo "Building SQLite..."
make -j$(nproc) > /dev/null
make install > /dev/null

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
# We set DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED=1 to tell the test suite 
# that we expect "no such function: load_extension" instead of "not authorized".
export DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED=1

# Clean first to ensure we link against the new library
cargo clean

echo "Running tests against restricted SQLite..."
if cargo test --package diesel_tests --test integration_tests load_extension --features sqlite; then
    echo "----------------------------------------------------------------"
    echo "SUCCESS: Diesel compiled and passed tests against SQLite without extension support."
else
    echo "----------------------------------------------------------------"
    echo "FAILURE: Tests failed or compilation failed."
    exit 1
fi
