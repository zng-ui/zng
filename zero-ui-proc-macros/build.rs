fn main() {
    if cfg!(debug_assertions) || cfg!(feature = "inspector") {
        println!("cargo:rustc-cfg=inspector")
    } else {
        if cfg!(feature = "dyn_property") {
            println!("cargo:rustc-cfg=dyn_property")
        }
        if cfg!(feature = "dyn_widget") {
            println!("cargo:rustc-cfg=dyn_widget")
        }
    }
}
