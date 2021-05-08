use zero_ui::prelude::*;

fn main() {
    println!("-=Headless Example=-\n");
    // This example uses a headless window to render an image.
    //
    // Note: this is a demonstration of headless, if you just want
    //       to render an image you can use the renderer directly
    //       like in the "renderer.rs" example.

    // we only need the window services.
    let mut app = App::default().run_headless();

    // set the renderer flag, this causes headless windows to
    // still load a renderer.
    app.enable_renderer(true);

    // open the window that is our image.
    app.with_context(|ctx| {
        ctx.services.req::<Windows>().open(|_| image(), None);
    });
    app.update();

    // Block until the frame is rendered.
    app.wait_frame();

    // save the drawing using the screenshot mechanism.
    app.with_context(|ctx| {
        let wns = ctx.services.req::<Windows>();
        let id = {
            // find the image window.
            let wn = wns.windows().next().unwrap();

            // copy the frame pixel data and save it.
            print!("saving ./screenshot.png ... ");
            flush_stdout();
            wn.screenshot().save("screenshot.png").expect("error saving screenshot");
            println!("done");

            wn.id()
        };
        wns.close(id).unwrap();
    });

    // apply the window close request, you need to close all
    // windows before dropping the `app`.
    app.update();
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
