fn main() {
    macro_rules! enable {
        ($feature:tt) => {
            if !cfg!(feature = $feature) {
                println!(concat!("cargo:rustc-cfg=feature=\"", $feature, "\""))
            }
        };
    }

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        enable!("dyn_app_extension");
        enable!("dyn_node");
        enable!("dyn_closure");
        enable!("inspector");
        enable!("trace_recorder");
        enable!("trace_widget");
    } else if cfg!(feature = "inspector") {
        enable!("dyn_node");
    }
}
