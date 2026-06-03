fn main() {
    #[cfg(feature = "embedded_l10n")]
    pack_l10n();
}

#[cfg(feature = "embedded_l10n")]
/// Pack l10n dir for embedding using `l10N.load_tar`.
fn pack_l10n() {
    println!("cargo::rerun-if-changed=pack-l10n");
    println!("cargo::rerun-if-changed=res/l10n");

    let res_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("pack-l10n");
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("pack-l10n");
    // cargo zng res
    let cargo_zng_res = std::process::Command::new("cargo")
        .arg("zng")
        .arg("res")
        .arg("--metadata")
        .arg("Cargo.toml")
        .arg(res_dir)
        .arg(out_dir)
        .status()
        .expect("failed to pack l10n resources");
    assert!(cargo_zng_res.success());
}
