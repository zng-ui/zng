#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs, ImageLoadingArgs};

fn main() {
    zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Image Example";
            state = WindowState::Maximized;

            // Set a loading view generator used in all images in this window.
            image_loading_view = ViewGenerator::new(image_loading);

            // Set a error view generator used in all images in this window.
            image_error_view = view_generator!(|_, args: &ImageErrorArgs| {
                log::error!(target: "expected", "{}", args.error);
                text!{
                    text = args.error.clone();
                    margin = 20;
                    color = colors::RED;
                }
            });
            content = v_stack!{
                spacing = 20;
                items = widgets![
                    demo_image("File", image("examples/res/image/RGB8.png")),
                    demo_image("Web", image("https://httpbin.org/image")),
                    demo_image("Web (accept)", image((Uri::from_static("https://httpbin.org/image"), "image/png"))),
                    demo_image("Error File", image("404.png")),
                    demo_image("Error Web", image("https://httpbin.org/delay/5")),
                    large_image(),
                ];
            };
        }
    })
}

fn demo_image(title: impl IntoVar<Text> + 'static, image: impl Widget) -> impl Widget {
    v_stack(widgets![
        strong(title),
        container! {
            content = image;
            content_align = unset!;
            padding = 2;
            background_color = colors::BLACK;
        }
    ])
}

fn large_image() -> impl Widget {
    button! {
        content = text("Large Image");
        on_click = hn!(|ctx, _| {
            ctx.services.windows().open(|_|window! {
                title = "Large Image - Starry Night";
                image_loading_view = ViewGenerator::new(image_loading);
                background_color = colors::BLACK;
                content = image! {
                    source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                    image_loading_view = ViewGenerator::new(image_loading);
                    on_error = hn!(|_, args: &ImageErrorArgs| {
                        log::error!(target: "unexpected", "{}", args.error);
                    })
                };
            });
        });
    }
}

/// Image loading animation.
fn image_loading(ctx: &mut WidgetContext, _: &ImageLoadingArgs) -> impl Widget {
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
        margin = 20;
        font_style = FontStyle::Italic;
    }
}
