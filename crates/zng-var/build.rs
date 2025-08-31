fn main() {
    if cfg!(feature = "dyn_closure") {
        println!(r#"cargo:warning=feature "dyn_closure" is deprecated, no longer needed"#);
    }

    macro_rules! enable {
        ($feature:tt) => {
            if !cfg!(feature = $feature) {
                println!(concat!("cargo:rustc-cfg=feature=\"", $feature, "\""))
            }
        };
    }

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        enable!("type_names");
    }
}
