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
        let countdown = count.map(|&n| {
            let r = if n > 0 { formatx!("{n}") } else { "Done!".to_text() };
            println!("{r}");
            r
        });
        let background_color = count.map(|&n| {
            let angle = (n + 3) as f32 / 10.0 * 360.0;
            hsl(angle.deg(), 80.pct(), 30.pct()).to_rgba()
        });

        let window_background = var(background_color.copy(ctx)).easing(150.ms(), easing::linear);
        background_color.bind(ctx, &window_background).perm();

        let icon_label = count.map(|&n| if n > 0 { formatx!("{n}") } else { "C".to_text() });

        let actual_icon = WindowVars::req(ctx.window_state).actual_icon();

        window! {
            title = "Countdown Example";
            size = (280, 120);
            start_position = StartPosition::CenterMonitor;
            resizable = false;

            icon = WindowIcon::render(RenderMode::Software, clone_move!(background_color, |_| container! {
                size = (36, 36);
                background_color = background_color.clone();
                corner_radius = 8;
                font_size = 28;
                font_weight = FontWeight::BOLD;
                content_align = Align::CENTER;
                content = text(icon_label.clone());
            }));
            visible = actual_icon.map(|om| om.as_ref().map(|m| !m.is_loading()).unwrap_or(false));

            font_size = 42.pt();
            background_color = window_background;
            content_align = Align::CENTER;
            content = text(countdown);
        }
    })
}
