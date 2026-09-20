#!/bin/bash
set -eu

cd "$SRC/diesel"
# the base image's RUSTUP_TOOLCHAIN (nightly) overrides the rust-toolchain pin, so we name none
cargo fuzz build -O --fuzz-dir fuzz

targets=$(cargo fuzz list --fuzz-dir fuzz)
if [ -z "$targets" ]; then
    echo "cargo fuzz list named no target" >&2
    exit 1
fi

target_dir=fuzz/target/x86_64-unknown-linux-gnu/release
for name in $targets; do
    cp "$target_dir/$name" "$OUT/"
done

# cifuzz unpacks <target>_seed_corpus.zip before fuzzing
for dir in fuzz/corpus/*/; do
    name=$(basename "$dir")
    zip -j -q "$OUT/${name}_seed_corpus.zip" "$dir"*
done
