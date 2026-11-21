fn main() {
    cfg_aliases::cfg_aliases! {
        android: { target_os = "android" },
        ipc: { all(feature = "ipc", not(android)) },
    }

    tp_licenses();
}

fn tp_licenses() {
    #[cfg(feature = "embed_licenses")]
    {
        let licenses = zng_tp_licenses::collect_cargo_about("./about.toml");
        zng_tp_licenses::write_embedding(&licenses);
    }
}
