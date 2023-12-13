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
        enable!("dyn_closure");
    } else {
        if cfg!(featue = "dyn_closure") {
            println!("cargo:rustc-cfg=dyn_closure");
        }
    }
}
