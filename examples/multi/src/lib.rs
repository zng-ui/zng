#[cfg(any(target_arch = "wasm32", target_os = "android"))]
mod app;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::app;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    fn main() {
        // Wasm requires a Javascript attribute in the "window" object, see the
        // zng::env::init! docs for more details.
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
    use zng::view_process::default::*;

    #[no_mangle]
    fn android_main(app: android::AndroidApp) {
        zng::env::init!();
        android::init_android_app(app);

        zng::app::print_tracing(tracing::Level::INFO);
        tracing::info!("Hello Android!");

        zng::env::android_install_res(|| app.asset_manager().open("res.tar.gz"));
        run_same_process(app::run);
    }
}
