#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|ctx| {
        let mut count = 11;
        let countdown = ctx.sync.update_every_secs(1).into_map(move |t| {
            count -= 1;
            let text = if count > 0 {
                formatx!("{}", count)
            } else {
                t.stop();
                "Done!".to_text()
            };
            println!("{}", text);
            text
        });
        window! {
            title = "Countdown Example";
            font_size = 32.pt();
            content = text(countdown);
        }
    })
}
