fn main() {
    // TODO(breaking) remove feature
    if cfg!(feature = "bundle") && !cfg!(feature = "_all_features") {
        println!("cargo::warning=feature \"bundle\" is deprecated, renamed to \"embed\"");
    }
}
