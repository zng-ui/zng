fn main() {
    if cfg!(feature = "dyn_node") {
        println!(r#"cargo:warning=feature "dyn_node" is deprecated, no longer needed"#);
    }
    if cfg!(feature = "dyn_closure") {
        println!(r#"cargo:warning=feature "dyn_closure" is deprecated, no longer needed"#);
    }
    if cfg!(feature = "debug_default") {
        println!(r#"cargo:warning=feature "debug_default" is deprecated, enable needed features directly"#);
    }
}
