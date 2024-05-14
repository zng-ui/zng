use std::path::PathBuf;

pub fn build(manifest_dir: &str) {
    tracing::info!("rebuilding `{manifest_dir}`");

    let manifest_path = format!("{manifest_dir}/Cargo.toml");

    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output();

    todo!()
}

/// Get compiled dyn lib name from manifest dir.
pub fn lib_name(manifest_dir: &str) -> Option<PathBuf> {
    let manifest_path = format!("{manifest_dir}/Cargo.toml");

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output();

    todo!()
}
