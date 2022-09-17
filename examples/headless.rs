use zero_ui::core::window::{FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt};
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // zero_ui_view::run_same_process(headless_example);

    images_render();
    headless_example();
}

fn headless_example() {
    println!("-=Headless Example=-\n");
    // This example uses a headless window to render an image.

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = App::default().run_headless(true);

    app.run_window(|_| {
        window! {
            // the window content is the image.
            content = image();
            auto_size = true;

            // use the CPU only backend if available, by default the
            // same GPU used for headed windows is used.
            render_mode = RenderMode::Software;

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

                // and close the window, causing the app to exit.
                ctx.with(|ctx|Windows::req(ctx.services).close(ctx.path.window_id())).unwrap();
            });
        }
    });
}

// A 800x600 "Hello World!" with a fancy background.
fn image() -> impl Widget {
    container! {
        size = (800, 600);

        background = stack_nodes({
            fn gradient(angle: i32, mut color: Rgba) -> impl UiNode {
                color.alpha = 0.3;
                let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                linear_gradient(angle.deg(), stops)
            }

            nodes![
                flood(colors::WHITE),
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
            align = Align::CENTER;
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

/// You can also use the `Images` service to render to an image.
#[allow(unused)]
fn images_render() {
    println!("-=Images::render Example=-\n");

    use zero_ui::core::{app::ControlFlow, image::*};

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = App::default().run_headless(true);

    // request an image rendered from a node, the `Images` service will render the node and update the image
    // variable every time the node (re)renders.
    let img = Images::req(&mut app).render_node(RenderMode::Software, 1.fct(), |_| image());

    app.run_task(move |ctx| async move {
        while ctx.with(|ctx| img.get(ctx).is_loading()) {
            img.wait_new(&ctx).await;
        }
        let img = img.get_clone(&ctx);

        if img.is_loaded() {
            // we save it...
            print!("saving ./screenshot.Images.png ... ");
            flush_stdout();

            img.save("screenshot.Images.png").await.unwrap();

            println!("done");
        } else if let Some(err) = img.error() {
            eprintln!("[error]: {err}");
        }
    });

    // Internally the `Images` service uses a headless window for rendering too, but this method is more easy
    // to use, with the trade-off that you have less control over the headless window.
}
