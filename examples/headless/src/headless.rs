use std::io::Write as _;
use zng::{
    color::{
        self,
        gradient::{GradientStops, linear_gradient},
    },
    prelude::*,
    window::{FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt},
};

/// This example uses a headless window to render an image.
pub fn run() {
    println!("-=Headless Example=-\n");

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);

    app.run_window(async {
        Window! {
            // the window content is the image.
            child = image();
            auto_size = true;

            // use the CPU only backend if available, by default the
            // same GPU used for headed windows is used.
            render_mode = window::RenderMode::Software;

            // capture the first frame.
            frame_capture_mode = FrameCaptureMode::Next;

            // this event will fire every time a frame is rendered (just once in this case).
            on_frame_image_ready = async_hn_once!(|args: FrameImageReadyArgs| {
                // in this case a `frame_image` was already captured.
                let img = args.frame_image.unwrap();

                // we save it...
                print!("saving ./screenshot.png ... ");
                std::io::stdout().lock().flush().ok();

                img.save("screenshot.png").await.unwrap();

                println!("done");
                APP.exit();
            });
        }
    });
}

fn image() -> UiNode {
    Container! {
        layout::size = (800, 600);

        widget::background = {
            fn gradient(angle: i32, mut color: color::Rgba) -> UiNode {
                color.alpha = 0.3;
                let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                linear_gradient(angle.deg(), stops).into_node()
            }

            ui_vec![
                color::flood(colors::WHITE),
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
        };

        child = Text! {
            layout::align = Align::CENTER;
            txt = "Hello World!";
            font_size = 72;
            font_family = ["Consolas", "monospace"];
            font_color = colors::WHITE;
        };
    }
}
