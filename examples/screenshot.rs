#![recursion_limit = "256"]

use zero_ui::core::{context::WidgetContext, window::Windows};
use zero_ui::prelude::*;

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        window! {
            title: "Screenshot Example";
            => button! {
                on_click: |a|take_screenshot(a.ctx());
                align: Alignment::CENTER;

                => text("Window screenshot")
            }
        }
    })
}

fn take_screenshot(ctx: &mut WidgetContext) {
    let windows = ctx.services.req::<Windows>();
    let size = windows.window(ctx.window_id).unwrap().size();
    let img = windows.screenshot(ctx.window_id, LayoutRect::from_size(size)).unwrap();
    img.save("screenshot.png").unwrap();
}
