fn main() {
    #[cfg(feature = "bundle_licenses")]
    {
        let licenses = zng_view_api::third_party::collect_cargo_about("../.cargo/about.toml");
        zng_view_api::third_party::write_bundle(&licenses);
    }
}
