#!/bin/sh

# Run Task
DO_NAME=do
DO_MANIFEST_PATH=tools/do-tasks/Cargo.toml
cargo run --manifest_path $DO_MANIFEST_PATH --release --quiet -- $*