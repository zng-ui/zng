fn main() {
    if cfg!(feature = "debug_default") {
        println!(r#"cargo:warning=feature "debug_default" is deprecated, enable needed features directly"#);
    }
}
