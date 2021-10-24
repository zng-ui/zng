#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::{image::ImageLimits, timer::Timers};
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs, ImageLoadingArgs};

fn main() {
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|ctx| {
        window! {
            title = "Image Example";
            state = WindowState::Maximized;

            // Set a loading view generator used in all images in this window.
            image_loading_view = ViewGenerator::new(image_loading);

            //transparent = true;

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
                    demo_image("File", uniform_grid! {
                        columns = 4;
                        spacing = 2;
                        align = Alignment::CENTER;
                        items = widgets![
                            image("examples/res/image/Luma8.png"),
                            image("examples/res/image/Luma16.png"),
                            image("examples/res/image/LumaA8.png"),
                            image("examples/res/image/LumaA16.png"),
                            image("examples/res/image/RGB8.png"),
                            image("examples/res/image/RGB16.png"),
                            image("examples/res/image/RGBA8.png"),
                            image("examples/res/image/RGBA16.png"),
                        ]
                    }),
                    demo_image("Web", image("https://httpbin.org/image")),
                    demo_image("Web (accept)", image((Uri::from_static("https://httpbin.org/image"), "image/png"))),
                    demo_image("Error File", image("404.png")),
                    demo_image("Error Web", image("https://httpbin.org/delay/5")),
                    demo_image("Sprite", sprite(ctx.timers)),
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
                title = "Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB";
                image_loading_view = ViewGenerator::new(image_loading);
                background_color = colors::BLACK;
                content = image! {
                    //source = {
                    //    let data = vec![255; 300 * 200 * 4];
                    //    (data, PxSize::new(Px(300), Px(200)))
                    //};
                    source = {
                        // same size but skip decoding.
                        let data = vec![255; 30_000 * 23_756 * 4];
                        (data, PxSize::new(Px(30_000), Px(23_756)))
                    };
                    //source = "large-image.jpg";
                    //source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                    loading_view = ViewGenerator::new(image_loading);
                    limits = Some(ImageLimits { max_encoded_size: 300.megabytes(), max_decoded_size: 3.gigabytes() });
                    on_error = hn!(|_, args: &ImageErrorArgs| {
                        log::error!(target: "unexpected", "{}", args.error);
                    })
                };
            });
        });
    }
}

fn sprite(timers: &mut Timers) -> impl Widget {
    let timer30fps = timers.interval((1.0 / 30.0).secs());
    let crop = timer30fps.map(|n| {
        if n.count() == 10 {
            n.set_count(0);
        }
        let offset = n.count() as i32 * 96;
        Rect::new((offset.px(), 0.px()), (96.px(), 84.px()))
    });

    image! {
        source = "examples/res/image/player_combat_sheet-10-96x84-CC0.png";
        on_click = hn!(|ctx, _| {
            let timer = timer30fps.get(ctx);
            timer.set_enabled(!timer.is_enabled());
        });
        crop;
        size = (96, 84);
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
