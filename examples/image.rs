#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::image_error_view;

fn main() {
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Image Example";
            image_error_view = view_generator!(|_, e: &str| {
                log::error!("Image error: {}", e);
                text!{
                    text = e.to_owned();
                    color = colors::RED;
                }
            });
            content = v_stack!{
                spacing = 20;
                items = widgets![
                    demo_image("Web", image("https://httpbin.org/image")),
                    demo_image("Error", image("404.png"))
                ];
            };
        }
    })
}

fn demo_image(title: impl IntoVar<Text> + 'static, image: impl Widget) -> impl Widget {
    v_stack(widgets![strong(title), image])
}
