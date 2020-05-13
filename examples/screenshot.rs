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
    let img = ctx.services.req::<Windows>().window(ctx.window_id).unwrap().screenshot();
    img.save("screenshot.png").unwrap();
}
