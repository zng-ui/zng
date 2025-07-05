//! Demonstrates the TIMERS service, variable mapping and profiler trace recording.

use zng::{image, prelude::*, widget::background_color};

fn main() {
    unsafe {
        std::env::set_var("ZNG_RECORD_TRACE", "");
    }

    zng::env::init!();
    app_main();
}

fn app_main() {
    APP.defaults().run_window(async {
        let count = timer::TIMERS.interval(1.secs(), false).map(move |t| {
            let count = 10 - t.count();
            if count == 0 {
                t.stop();
            }
            count
        });

        let bkg = count.map(|&n| {
            let angle = (n + 3) as f32 / 10.0 * 360.0;
            hsl(angle.deg(), 80.pct(), 30.pct()).into()
        });

        Window! {
            title = "Countdown Example";
            size = (280, 120);
            start_position = window::StartPosition::CenterMonitor;
            resizable = false;
            enabled_buttons = !window::WindowButton::MAXIMIZE;

            color_scheme = color::ColorScheme::Dark;

            font_size = 42.pt();
            child_align = Align::CENTER;

            background_color = bkg.easing(150.ms(), easing::linear);

            child = Text!(count.map(|&n| {
                let r = if n > 0 { formatx!("{n}") } else { "Done!".to_txt() };
                tracing::info!("{r}");
                r
            }));

            icon = WindowIcon::render(move || {
                Container! {
                    image::render_retain = true;

                    layout::size = (36, 36);
                    widget::corner_radius = 8;
                    text::font_size = 28;
                    text::font_weight = FontWeight::BOLD;
                    child_align = Align::CENTER;

                    background_color = bkg.clone();

                    child = Text!(count.map(|&n| if n > 0 { formatx!("{n}") } else { "C".to_txt() }));
                }
            });
        }
    })
}
