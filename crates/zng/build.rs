#![recursion_limit = "512"]

fn main() {
    cfg_aliases::cfg_aliases! {
        wasm: { target_arch = "wasm32" },
        android: { target_os = "android" },
        ipc: { all(feature = "ipc", not(any(android, wasm))) },
        view: { all(feature = "view", not(wasm)) },
        view_prebuilt: { all(feature = "view_prebuilt", not(any(android, wasm))) },
        hot_reload: { all(feature = "hot_reload", not(any(android, wasm))) },
        single_instance: { all(feature = "single_instance", not(any(android, wasm))) },
        crash_handler: { all(feature = "crash_handler", not(any(android, wasm))) },
        trace_recorder: { all(feature = "trace_recorder", not(any(android, wasm))) },
    }

    // TODO(breaking) remove feature
    if cfg!(feature = "view_bundle_licenses") && !cfg!(feature = "_all_features") {
        println!(
            "cargo::warning=feature \"view_bundle_licenses\" is deprecated, zng-view licenses are already collected when compiling with \"view\" and using `collect_cargo_about`"
        );
    }
}
