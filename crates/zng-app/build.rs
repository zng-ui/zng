fn main() {
    if cfg!(feature = "dyn_node") {
        println!(r#"cargo:warning=feature "dyn_node" is deprecated, no longer needed"#);
    }
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
        enable!("dyn_app_extension");
        enable!("inspector");
        enable!("trace_recorder");
        enable!("trace_widget");
    }
}
