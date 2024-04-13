fn main() {
    #[cfg(feature = "bundle_licenses")]
    {
        let licenses = zng_tp_licenses::collect_cargo_about("../.cargo/about.toml");
        zng_tp_licenses::write_bundle(&licenses);
    }
}
