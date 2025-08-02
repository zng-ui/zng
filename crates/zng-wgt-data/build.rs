fn main() {
    macro_rules! enable {
        ($feature:tt) => {
            if !cfg!(feature = $feature) {
                println!(concat!("cargo:rustc-cfg=feature=\"", $feature, "\""))
            }
        };
    }

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        enable!("var_type_names");
    }
}
