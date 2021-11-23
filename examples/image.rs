#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::{image::ImageLimits, timer::Timers};
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs, ImageLoadingArgs};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    //zero_ui_view::run_same_process(app_main);

    examples_util::print_info();

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
            image_error_view = view_generator!(|_, args: ImageErrorArgs| {
                text! {
                    text = args.error;
                    margin = 20;
                    align = Alignment::CENTER;
                    color = colors::RED;
                    drop_shadow = {
                        offset: (0, 0),
                        blur_radius: 4,
                        color: colors::DARK_RED
                    };
                }
            });
            content = h_stack! {
                spacing = 30;
                items = widgets![
                    v_stack! {
                        spacing = 20;
                        items = widgets![
                            title("image_source"),
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
                    },
                    v_stack! {
                        spacing = 20;
                        items = widgets![
                            title("fit"),
                            demo_image(
                                "None",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::None;
                                }
                            ),
                            demo_image(
                                "Fill",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Fill;
                                }
                            ),
                            demo_image(
                                "Contain",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Contain;
                                }
                            ),
                            demo_image(
                                "Cover",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                }
                            ),
                            demo_image(
                                "ScaleDown",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::ScaleDown;
                                }
                            ),
                        ];
                    },
                    v_stack! {
                        spacing = 20;
                        items = widgets![
                            title("filter"),
                            demo_image(
                                "grayscale",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    grayscale = true;
                                }
                            ),
                            demo_image(
                                "sepia",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    sepia = true;
                                }
                            ),
                            demo_image(
                                "blur",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    blur = 4;
                                }
                            ),
                            demo_image(
                                "opacity",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    opacity = 50.pct();
                                }
                            ),
                            demo_image(
                                "invert_color",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    invert_color = true;
                                }
                            ),
                            demo_image(
                                "hue_rotate",
                                image! {
                                    source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                                    size = (200, 100);
                                    fit = ImageFit::Cover;
                                    hue_rotate = -(90.deg());
                                }
                            ),
                        ]
                    }
                ]
            }
        }
    })
}

fn demo_image(title: &'static str, image: impl Widget) -> impl Widget {
    v_stack(widgets![
        text! {
            text = title;
            margin = (0, 0, 4, 0);
            font_weight = FontWeight::BOLD;
        },
        container! {
            content = image;
            content_align = unset!;
            background = transparency();
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
                background = transparency();
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
                        tracing::error!(target: "unexpected", "{}", args.error);
                    })
                };
            });
        });
    }
}

fn sprite(timers: &mut Timers) -> impl Widget {
    let timer = timers.interval((1.0 / 24.0).secs(), false);
    let label = var_from("play");

    v_stack! {
        align = Alignment::CENTER;
        items = widgets![
            button! {
                content = text(label.clone());
                align = Alignment::CENTER;
                padding = (2, 3);
                on_click = hn!(timer, |ctx, _| {
                    let t = timer.get(ctx);
                    let enabled = !t.is_enabled();
                    t.set_enabled(enabled);
                    label.set(ctx, if enabled { "stop" } else { "play" });
                });
            },
            image! {
                source = "examples/res/image/player_combat_sheet-10-96x84-CC0.png";
                size = (96, 84);
                crop = timer.map(|n| {
                    if n.count() == 10 {
                        n.set_count(0);
                    }
                    let offset = n.count() as i32 * 96;
                    Rect::new((offset.px(), 0.px()), (96.px(), 84.px()))
                });
            },
        ]
    }
}

/// Image loading animation.
fn image_loading(ctx: &mut WidgetContext, _: ImageLoadingArgs) -> impl Widget {
    let mut dots_count = 3;
    let msg = ctx.timers.interval(300.ms(), true).map(move |_| {
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
        align = Alignment::CENTER;
        width = 80;
        font_style = FontStyle::Italic;
        drop_shadow = {
            offset: (0, 0),
            blur_radius: 4,
            color: colors::GRAY,
        };
    }
}

fn transparency() -> impl Widget {
    checkerboard! {
        colors = rgb(20, 20, 20), rgb(40, 40, 40);
        cb_size = (16, 16);
    }
}

fn title(s: &'static str) -> impl Widget {
    text! {
        text = s;
        font_size = 20;
    }
}
