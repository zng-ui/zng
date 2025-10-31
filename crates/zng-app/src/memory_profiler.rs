#![cfg(all(
    feature = "memory_profiler",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios", test, doc))
))]

//! Memory profiler.

use std::path::PathBuf;

use parking_lot::Mutex;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

zng_app_context::hot_static! {
    static PROFILER: Mutex<Option<(dhat::Profiler, String)>> = Mutex::new(None);
}
fn profiler() -> parking_lot::MutexGuard<'static, Option<(dhat::Profiler, String)>> {
    zng_app_context::hot_static_ref!(PROFILER).lock()
}

zng_env::on_process_start!(|_| {
    let mut p = profiler();
    if p.is_none() {
        // first process sets the timestamp
        let timestamp = match std::env::var("ZNG_MEMORY_PROFILER_TIMESTAMP") {
            Ok(t) => t,
            Err(_) => {
                let process_start = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .expect("cannot define process start timestamp")
                    .as_micros();
                let t = process_start.to_string();
                // SAFETY: safe, only read by this pure Rust code in subsequent started processes.
                unsafe {
                    std::env::set_var("ZNG_MEMORY_PROFILER_TIMESTAMP", t.clone());
                }
                t
            }
        };

        *p = Some((dhat::Profiler::builder().file_name(PathBuf::new()).build(), timestamp));
        zng_env::on_process_exit(|_| stop_recording());
    }
});

/// Stop recording earlier.
///
/// Note that by default recording stops [`on_process_exit`].
///
/// [`on_process_exit`]: zng_env::on_process_exit
pub fn stop_recording() {
    if let Some((mut profiler, timestamp)) = profiler().take() {
        let profile = profiler.drop_and_get_memory_output();
        std::mem::forget(profiler);

        let p_id = std::process::id();
        let p_name = zng_env::process_name();

        let dir = std::env::var("ZNG_MEMORY_PROFILER_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().expect("`current_dir` error").join("zng-dhat"));

        let dir = dir.join(timestamp);
        std::fs::create_dir_all(&dir).expect("cannot create memory profile output dir");

        std::fs::write(dir.join(format!("{p_name}-{p_id}.json")), profile.as_bytes()).expect("cannot write profile to output dir");
    }
}
