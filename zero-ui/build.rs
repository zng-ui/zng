fn main() {
    macro_rules! enable {
        ($feature:tt) => {
            if !cfg!(feature = $feature) {
                println!(concat!("cargo:rustc-cfg=feature=\"", $feature, "\""))
            }
            println!(concat!("cargo:rustc-cfg=", $feature))
        };
    }

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        enable!("dyn_app_extension");
        enable!("dyn_node");
        enable!("dyn_closure");
        enable!("inspector");
        enable!("trace_widget");
    } else if cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
        enable!("dyn_node");
    } else {
        if cfg!(featue = "dyn_node") {
            println!("cargo:rustc-cfg=dyn_node");
        }
        if cfg!(featue = "dyn_closure") {
            println!("cargo:rustc-cfg=dyn_closure");
        }
        if cfg!(feature = "dyn_widget") {
            println!("cargo:rustc-cfg=dyn_widget");
        }
        if cfg!(feature = "dyn_property") {
            println!("cargo:rustc-cfg=dyn_property");
        }
    }

    if cfg!(feature = "http") {
        println!("cargo:rustc-cfg=http");
    }
}
