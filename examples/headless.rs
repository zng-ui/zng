use zero_ui::core::window::{FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt};
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    println!("-=Headless Example=-\n");
    // This example uses a headless window to render an image.

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = App::default().run_headless(true);

    app.run_window(|_| {
        window! {
            // the window content is the image.
            content = image();
            auto_size = true;

            // capture the first frame.
            frame_capture_mode = FrameCaptureMode::Next;

            // this event will fire every time a frame is rendered (just once in this case).
            on_frame_image_ready = async_hn!(|ctx, args: FrameImageReadyArgs| {
                // in this case a `frame_image` was already captured.
                let img = args.frame_image.unwrap();

                // we save it...
                print!("saving ./screenshot.png ... ");
                flush_stdout();

                img.save("screenshot.png").await.unwrap();

                println!("done");

                // and close the window, causing the app to shutdown.
                ctx.with(|ctx|ctx.services.windows().close(ctx.path.window_id())).unwrap();
            });
        }
    });
}

// A 800x600 "Hello World!" with a fancy background.
fn image() -> impl Widget {
    container! {
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
