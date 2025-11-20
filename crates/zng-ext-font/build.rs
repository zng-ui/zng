#![recursion_limit = "256"]

fn main() {
    cfg_aliases::cfg_aliases! {
        wasm: { target_arch = "wasm32" },
        android: { target_os = "android" },
        ipc: { all(feature = "ipc", not(any(android, wasm))) },
    }
}
