use zero_ui::prelude::*;
use zero_ui_core::window::HeadlessAppWindowExt;

fn main() {
    if cfg!(debug_assertions) {
        zero_ui_view::run_same_process(app_main);
    } else {
        zero_ui_view::init();
        app_main();
    }
}

fn app_main() {
    println!("-=Headless Example=-\n");
    // This example uses a headless window to render an image.

    // open headless with renderer flag, this causes the view process
    // to still start.
    let mut app = App::default().run_headless(true);

    // open the window that is our image.
    let window_id = app.open_window(|_| image());

    // copy the first frame.
    let frame = app.wait_window_frame(window_id);

    // save the frame.
    print!("saving ./screenshot.png ... ");
    flush_stdout();

    app.block_on_fut(frame.save("screenshot.png"), 10.secs()).expect("error saving screenshot").unwrap();
    println!("done");

    // you need to close all windows before dropping the `app`.
    app.close_window(window_id);
}

// A 800x600 "Hello World!" with a fancy background.
fn image() -> Window {
    window! {
        size = (800, 600);

        background = z_stack({
            fn gradient(angle: i32, mut color: Rgba) -> impl UiNode {
                color.alpha = 0.3;
                let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                linear_gradient(angle.deg(), stops)
            }

            nodes![
                fill_color(colors::WHITE),
                gradient(0, colors::RED),
                gradient(20, colors::RED),
                gradient(40, colors::RED),
                gradient(120, colors::GREEN),
                gradient(140, colors::GREEN),
                gradient(160, colors::GREEN),
                gradient(240, colors::BLUE),
                gradient(260, colors::BLUE),
                gradient(280, colors::BLUE),
            ]
        });

        content = text! {
            text = "Hello World!";
            font_size = 72;
            font_family = ["Consolas", "monospace"];
            color = colors::WHITE;
        };
    }
}

fn flush_stdout() {
    use std::io::Write;
    std::io::stdout().lock().flush().ok();
}
