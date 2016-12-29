#!/bin/bash
set -e

###
# Environment
###

ROOT=`pwd`

if [[ $TRAVIS == "true" ]]; then
  CARGO_TEST="travis-cargo test --"
  RUST_VERSION=$TRAVIS_RUST_VERSION
else
  CARGO_TEST="cargo test"
  RUST_VERSION=stable
  BACKEND=sqlite
fi

echo "RUST_VERSION: $RUST_VERSION"
echo "BACKEND: $BACKEND"

###
# Helpers
#
# The following helpers are written to be composed together to form an chain of
# commands that reads nicely and makes it obvious under which conditions a test
# command will be executed.
###

# Execute something in a specific directory
#
# This is used to start a chain of commands, thus is includes some logging.
in_dir() {
  printf "=> in directory \``pwd`\` "
  (cd "$ROOT/$1" && ${@:2})
  echo "========================================"
}

# Just a separater to make the logging output nicer
run() {
  printf "run "
  $@
}

# Only execute the commands following it if we are using a nightly rustc version
on_nightly() {
  if [[ "$RUST_VERSION" == nightly* ]]; then
    printf "on nightly "
    $@
  else
    printf "do nothing (not on nightly)\n"
  fi
}

# Only execute the commands following it if we are using a non-nightly rustc
#
# (This way, beta counts as stable as it does not have the nightly features.)
on_stable() {
  if [[ "$RUST_VERSION" == stable ]] || [[ "$RUST_VERSION" == beta ]]; then
    printf "on stable "
    $@
  else
    printf "do nothing (not on stable)\n"
  fi
}

# Only execute the commands following it if we are using PostgreSQL as a backend
using_postgres() {
  if [[ "$BACKEND" == postgres ]]; then
    printf "using postgres "
    $@ postgres
  else
    printf "do nothing (not using postgres)\n"
  fi
}

# Only execute the commands following it if we are using SQLite as a backend
using_sqlite() {
  if [[ "$BACKEND" == sqlite ]]; then
    printf "using sqlite "
    $@ sqlite
  else
    printf "do nothing (not using sqlite)\n"
  fi
}

# Execute a cargo test command
cargo_test() {
  printf "test with features \`$*\`\n"
  echo "----------------------------------------"
  $CARGO_TEST --no-default-features --features "$*"
}
