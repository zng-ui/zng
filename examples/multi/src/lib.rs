#[cfg(any(target_arch = "wasm32", target_os = "android"))]
mod app;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::app;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    fn main() {
        zng::env::init!();
        app::run_headless();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let val = document.create_element("p").unwrap();
        val.set_inner_html("Hello from Rust!");

        body.append_child(&val).unwrap();
    }

    zng::env::on_process_start!(|_| {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let val = document.create_element("p").unwrap();
        val.set_inner_html("on_process_start!");

        body.append_child(&val).unwrap();
    });
}

#[cfg(target_os = "android")]
mod android {
    use super::app;

    #[no_mangle]
    fn android_main(app: zng::view_process::default::android::AndroidApp) {
        zng::env::init!();
        zng::app::print_tracing(tracing::Level::INFO);

        if let Some(p) = app.internal_data_path() {
            zng::env::init_config(p);
        }

        tracing::info!("Hello Android!");
        zng::view_process::default::run_same_process(app, app::run);
    }
}
