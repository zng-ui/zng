use zero_ui::prelude::*;

fn main() {
    zero_ui_view::init();
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
