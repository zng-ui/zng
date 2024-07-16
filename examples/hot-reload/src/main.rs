//! Demonstrates the `"hot_reload"` feature.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::prelude::*;

fn main() {
    zng::env::init!();

    // examples/Cargo.toml enables the `"hot_reload"` feature for `zng`,
    // so the hot reload extension is available in `APP.defaults()`.
    APP.defaults().run_window(async {
        // default rebuild is just `cargo build`, the rebuilder must match the Cargo feature set
        // used to run the program, it will rebuild only until the dylib is finished.
        //
        // do run hot-reload uses the --manifest-path
        let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        zng::hot_reload::HOT_RELOAD.rebuilder(|a| a.build_manifest(manifest_path));

        let state = var(true);
        let example = Container! {
            // hot reloading node, edit the code in `examples/hot-reload-lib` to see updates.
            child = hot_reload_lib::hot_node();

            // hot reloading property, the state does not change, but changes in the property code update.
            hot_reload_lib::hot_prop = state.clone();
            gesture::on_click = hn!(|_| {
                state.set(!state.get());
            });
        };

        Window! {
            title = "Hot Reload Example";
            always_on_top = true;

            child = example;

            // layout affects the hot node correctly.
            child_align = Align::CENTER;
            // context values propagate to the hot node correctly.
            text::font_size = 2.em();
        }
    })
}
