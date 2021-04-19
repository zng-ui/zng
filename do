#!/bin/bash

export DO_NAME="do"
export DO_MANIFEST_PATH="tools/do-tasks/Cargo.toml"
cargo run --manifest-path $DO_MANIFEST_PATH --release --quiet -- $*