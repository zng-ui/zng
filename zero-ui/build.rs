fn main() {
    zero_ui_docs::html_in_header();

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        println!("cargo:rustc-cfg=feature=\"inspector\"");
        println!("cargo:rustc-cfg=inspector");
    } else if cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector");
    }
}
