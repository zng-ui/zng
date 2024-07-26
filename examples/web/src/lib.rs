#![cfg(target_arch = "wasm32")]

mod app;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn main() {
    app::run();

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
