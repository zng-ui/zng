#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|ctx| {
        let mut count = 10;
        let count = ctx.timers.interval(1.secs()).into_map(move |t| {
            let r = count;
            if r == 0 {
                t.destroy();
            }
            count -= 1;
            r
        });
        let countdown = count.map(move |&n| {
            let r = if n > 0 { formatx!("{}", n) } else { "Done!".to_text() };
            println!("{}", r);
            r
        });
        let background_color = count.into_map(|&n| {
            let angle = (n + 3) as f32 / 10.0 * 360.0;
            hsl(angle.deg(), 80.pct(), 30.pct()).to_rgba()
        });
        window! {
            title = "Countdown Example";
            size = (280, 120);
            start_position = StartPosition::CenterScreen;
            resizable = false;

            font_size = 42.pt();
            background_color;
            content = text(countdown);
        }
    })
}
