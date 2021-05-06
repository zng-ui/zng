//! # renderer
//!
//! This example shows how to use the `zero_ui::core::render::Renderer` type directly to generate
//! images using the widgets for drawing. This is a more convenient way of generating images then
//! using the the full headless app.

// Types needed to create a headless renderer.
use zero_ui::core::render::{RenderSize, Renderer, RendererConfig};

// Renderer is async and uses a callback to signal a frame is rendered.
// we use an std channel to await the frame.
use std::sync::mpsc::channel;

// We will use normal widgets to draw the [`image`], the prelude contains
// all the widgets, properties and types we need for that.
use zero_ui::prelude::*;

fn main() {
    println!("-=Renderer Example=-\n");

    let (sender, receiver) = channel();
    let on_frame_ready = move |_| {
        let _ = sender.send(());
    };

    let mut renderer =
        Renderer::new(RenderSize::new(800, 600), 1.0, RendererConfig::default(), on_frame_ready).expect("error creating renderer");

    // The renderer will initialize the `image`, do updates, layout using the render size
    // and then start rendering the widget. The actual rendering is async, the `on_frame_ready`
    // callback will be called from another thread when the rendering is done.
    renderer.render_new_ui(|_| image());

    print!("rendering ... ");
    flush_stdout();

    let _ = receiver.recv();
    let frame = renderer.frame_pixels().expect("error reading frame");

    println!("done");

    print!("saving ./screenshot.png ... ");
    flush_stdout();

    frame.save("screenshot.png").expect("error saving PNG");

    println!("done");
}

fn image() -> impl Widget {
    container! {
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
