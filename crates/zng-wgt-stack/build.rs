fn main() {
    if cfg!(feature = "dyn_closure") {
        println!(r#"cargo:warning=feature "dyn_closure" is deprecated, no longer needed"#);
    }
}
