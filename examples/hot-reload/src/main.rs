//! Demonstrates the `"hot_reload"` feature.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::prelude::*;

fn main() {
    examples_util::print_info();
    zng::env::init!();
    zng::app::crash_handler::init_debug();
    app_main();
}

fn app_main() {
    // examples/Cargo.toml enables the `"hot_reload"` feature for `zng`,
    // so the hot reload extension is available in `APP.defaults()`.
    APP.defaults().run_window(async {
        // default rebuild is just `cargo build`, the rebuilder must match the Cargo feature set
        // used to run the program, it will rebuild only until the dylib is finished.
        zng::hot_reload::HOT_RELOAD.rebuilder(|a| a.build_example(Some("examples"), "hot_reload"));

        let state = var(true);
        let example = Container! {
            // hot reloading node, edit the code in `examples/hot-reload-lib` to see updates.
            child = examples_hot_reload::hot_node();

            // hot reloading property, the state does not change, but changes in the property code update.
            examples_hot_reload::hot_prop = state.clone();
            gesture::on_click = hn!(|_| { state.set(!state.get()); });
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
