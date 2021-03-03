#!/bin/bash

DO_NAME="do"
DO_MANIFEST_PATH="tools/do-tasks/Cargo.toml"
cargo run --manifest-path $DO_MANIFEST_PATH --release --quiet -- $*