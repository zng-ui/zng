#![recursion_limit = "512"]

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
}
