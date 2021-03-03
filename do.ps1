$env:DO_NAME = "do"
$env:DO_MANIFEST_PATH = "tools/do-tasks/Cargo.toml"
cargo run --manifest-path $env:DO_MANIFEST_PATH --release --quiet -- $args