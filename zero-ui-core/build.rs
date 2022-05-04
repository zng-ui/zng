fn main() {
    zero_ui_docs::html_in_header();

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
        enable!("dyn_widget");
        enable!("dyn_node");
        enable!("inspector");
    } else if cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
        enable!("dyn_widget");
        enable!("dyn_node");
    } else {
        if cfg!(feature = "dyn_widget") {
            println!("cargo:rustc-cfg=dyn_widget");
        }
        if cfg!(featue = "dyn_node") {
            println!("cargo:rustc-cfg=dyn_node");
        }
    }

    if cfg!(feature = "http") {
        println!("cargo:rustc-cfg=http");
    }
}
