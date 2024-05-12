fn main() {
    // help `unexpected_cfgs` lint
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");
}
