#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|ctx| {
        let count = ctx.timers.interval(1.secs(), true).map(move |t| {
            let count = 10 - t.count();
            if count == 0 {
                t.stop();
            }
            count
        });
        let countdown = count.map(move |&n| {
            let r = if n > 0 { formatx!("{}", n) } else { "Done!".to_text() };
            println!("{}", r);
            r
        });
        let background_color = count.map(|&n| {
            let angle = (n + 3) as f32 / 10.0 * 360.0;
            hsl(angle.deg(), 80.pct(), 30.pct()).to_rgba()
        });
        window! {
            title = "Countdown Example";
            size = (280, 120);
            start_position = StartPosition::CenterMonitor;
            resizable = false;

            font_size = 42.pt();
            background_color;
            content = text(countdown);
        }
    })
}
