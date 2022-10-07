#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // zero_ui_view::run_same_process(app_main);
    app_main();
}

fn app_main() {
    App::default().run_window(|ctx| {
        let count = ctx.timers.interval(1.secs(), false).map(move |t| {
            let count = 10 - t.count();
            if count == 0 {
                t.stop();
            }
            count
        });

        let bkg = count.map(|&n| {
            let angle = (n + 3) as f32 / 10.0 * 360.0;
            hsl(angle.deg(), 80.pct(), 30.pct()).to_rgba()
        });

        window! {
            title = "Countdown Example";
            size = (280, 120);
            start_position = StartPosition::CenterMonitor;
            resizable = false;

            color_scheme = ColorScheme::Dark;

            font_size = 42.pt();
            content_align = Align::CENTER;

            background_color = bkg.easing(150.ms(), easing::linear);

            content = text(count.map(|&n| {
                let r = if n > 0 { formatx!("{n}") } else { "Done!".to_text() };
                println!("{r}");
                r
            }));

            icon = WindowIcon::render(move |_,| container! {
                zero_ui::core::image::render_retain = true;

                size = (36, 36);
                corner_radius = 8;
                font_size = 28;
                font_weight = FontWeight::BOLD;
                content_align = Align::CENTER;

                background_color = bkg.clone();

                content = text(count.map(|&n| if n > 0 { formatx!("{n}") } else { "C".to_text() }));
            });
        }
    })
}
