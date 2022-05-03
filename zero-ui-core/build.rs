fn main() {
    zero_ui_docs::html_in_header();

    if cfg!(debug_assertions) || cfg!(feature = "dyn_app_extension") {
        println!("cargo:rustc-cfg=dyn_app_extension");
    }
    if cfg!(feature = "dyn_widget") {
        println!("cargo:rustc-cfg=dyn_widget");
    }
    if cfg!(feature = "dyn_property") {
        println!("cargo:rustc-cfg=dyn_property");
    }
    if cfg!(debug_assertions) || cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
        println!("cargo:rustc-cfg=dyn_widget");
        println!("cargo:rustc-cfg=dyn_property");
    }
    if cfg!(feature = "http") {
        println!("cargo:rustc-cfg=http");
    }
}
