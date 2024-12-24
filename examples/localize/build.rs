fn main() {
    #[cfg(not(debug_assertions))]
    pack_l10n();
}

#[cfg(not(debug_assertions))]
/// Pack l10n dir for embedding using `l10N.load_tar`.
fn pack_l10n() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::process::Command::new("tar")
        .arg("-czf")
        .arg(out.join("l10n.tar.gz"))
        .current_dir("res")
        .arg("l10n")
        .status()
        .expect("failed to pack l10n resources");
    println!("cargo::rerun-if-changed=res/l10n")
}
