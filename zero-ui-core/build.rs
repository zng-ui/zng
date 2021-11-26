fn main() {
    if cfg!(debug_assertions) || cfg!(feature = "dyn_app_extension") {
        println!("cargo:rustc-cfg=dyn_app_extension");
    }
    if cfg!(debug_assertions) || cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
    }
}
