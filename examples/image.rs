#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs};

fn main() {
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Image Example";
            
            // Set a loading view generator used in all images in this window.
            image_loading_view = view_generator!(|ctx, _| {
                let mut dots_count = 3;
                let msg = ctx.timers.interval(300.ms()).map(move |_| {
                    dots_count += 1;
                    if dots_count == 8 {
                        dots_count = 0;
                    }
                    formatx!("loading{:.^1$}", "", dots_count)
                });
                text! {
                    text = msg;
                    color = colors::LIGHT_GRAY;
                    font_style = FontStyle::Italic;
                }
            });

            // Set a error view generator used in all images in this window.
            image_error_view = view_generator!(|_, args: &ImageErrorArgs| {
                log::error!("Image error: {}", args.error);
                text!{
                    text = args.error.clone();
                    color = colors::RED;
                }
            });
            content = v_stack!{
                spacing = 20;
                items = widgets![
                    demo_image("Web", image("https://httpbin.org/image")),
                    demo_image("Error", image("404.png")),
                    demo_image("Error Web", image("https://httpbin.org/delay/5"))
                ];
            };
        }
    })
}

fn demo_image(title: impl IntoVar<Text> + 'static, image: impl Widget) -> impl Widget {
    v_stack(widgets![strong(title), image])
}
