use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        let txt = var("loading…".to_text());
        window! {
            title = "http client example";
            content = text(txt.clone());
            on_open = async_hn_once!(|ctx, _| {
                match task::http::get_text("https://httpbin.org/get").await {
                    Ok(h) => txt.set(&ctx, h),
                    Err(e) => txt.set(&ctx, e.to_string()),
                }
            });
        }
    })
}
