fn main() {
    zero_ui_docs::html_in_header();

    if cfg!(debug_assertions) || cfg!(feature = "dyn_app_extension") {
        println!("cargo:rustc-cfg=dyn_app_extension");
    }
    if cfg!(debug_assertions) || cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
    }
    if cfg!(feature = "http") {
        println!("cargo:rustc-cfg=http");
    }
}
