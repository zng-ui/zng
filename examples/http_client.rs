use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let html = var("".to_text());
        window! {
            title = "http client example";
            content = text(html.clone());
            on_open = async_hn_once!(|ctx, _| {
                match task::http::get_text("https://httpbin.org/get").await {
                    Ok(h) => html.set(&ctx, h),
                    Err(e) => html.set(&ctx, e.to_string()),
                }
            });
        }
    })
}
