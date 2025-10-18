use std::io::Write as _;
use zng::{image::Img, prelude::*};

/// This example uses the `IMAGES` service to render to an image.
pub fn run() {
    println!("-=IMAGES.render Example=-\n");

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);

    // request an image rendered from a node, the `IMAGES` service will render the node and update the image
    // variable every time the node (re)renders.
    let img = zng::image::IMAGES.render_node(window::RenderMode::Integrated, 1.fct(), None, logo);

    app.run_task(async move {
        while img.with(Img::is_loading) {
            img.wait_update().await;
        }
        let img = img.get();

        if img.is_loaded() {
            // we save it...
            print!("saving ./examples/image/res/zng-logo.png ... ");
            std::io::stdout().lock().flush().ok();

            img.save("examples/image/res/zng-logo-icon.png").await.unwrap();

            println!("done");
        } else if let Some(err) = img.error() {
            eprintln!("[error]: {err}");
        }
    });

    // Internally the `IMAGES` service uses a headless window for rendering too, but this method is more easy
    // to use, with the trade-off that you have less control over the headless window.
}

fn logo() -> UiNode {
    let logo = Container! {
        layout::size = 200;
        layout::perspective = 500;

        child = Stack! {
            layout::transform_style = layout::TransformStyle::Preserve3D;
            text::font_size = 180;
            text::font_family = "Arial";
            text::font_weight = FontWeight::EXTRA_BOLD;
            text::txt_align = Align::CENTER;
            text::font_color = colors::WHITE;
            layout::transform = layout::Transform::new_rotate_y(-45.deg()).rotate_x(-35.deg()).translate_z(-100);
            children = ui_vec![
                Text! {
                    txt = "Z";
                    layout::padding = (-40, 0, 0, 0);
                    layout::transform = layout::Transform::new_translate_z(100);
                    widget::background_color = colors::RED.darken(50.pct());
                    widget::border = (0, 0, 12, 12), colors::WHITE;
                },
                Text! {
                    txt = "Z";
                    layout::padding = (-40, 0, 0, 0);
                    layout::transform = layout::Transform::new_translate_z(100).rotate_x(90.deg());
                    widget::background_color = colors::GREEN.darken(50.pct());
                    widget::border = (12, 0, 0, 12), colors::WHITE;
                },
                Text! {
                    txt = "g";
                    layout::padding = (-75, 0, 0, 0);
                    layout::transform = layout::Transform::new_translate_z(100).rotate_y(90.deg());
                    widget::background_color = colors::BLUE.darken(50.pct());
                    widget::border = (0, 12, 12, 0), colors::WHITE;
                },
            ];
        };
    };

    Container! {
        layout::size = 278;

        child_align = Align::CENTER;
        padding = (-27, 0, 0, 0);

        child = logo;
    }
}
