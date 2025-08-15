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
        trace_recorder: { all(feature = "crash_handler", not(any(android, wasm))) },
    }

    macro_rules! enable {
        ($feature:tt) => {
            if !cfg!(feature = $feature) {
                println!(concat!("cargo:rustc-cfg=feature=\"", $feature, "\""))
            }
        };
    }

    if cfg!(debug_assertions) && cfg!(feature = "debug_default") {
        enable!("dyn_app_extension");
        enable!("dyn_node");
        enable!("dyn_closure");
        enable!("inspector");
        enable!("trace_widget");
        enable!("trace_recorder");
        enable!("var_type_names");
    } else if cfg!(feature = "inspector") {
        enable!("dyn_node");
    }
}
