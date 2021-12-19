#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::{image::ImageLimits, timer::Timers};
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|ctx| {
        img_window(
            "Image Example",
            h_stack! {
                spacing = 30;
                items = widgets![
                    section(
                        "Image Sources",
                        widgets![
                            sub_title("File"),
                            uniform_grid! {
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
                            },
    
                            sub_title("Web"),
                            image("https://httpbin.org/image"),
    
                            sub_title("Web With Format"),
                            image((Uri::from_static("https://httpbin.org/image"), "image/png")),
                        ]
                    ),

                    section(
                        "Fit",
                        widgets![
                            img_fit(ImageFit::None),
                            img_fit(ImageFit::Fill),
                            img_fit(ImageFit::Contain),
                            img_fit(ImageFit::Cover),
                            img_fit(ImageFit::ScaleDown),
                        ]
                    ),

                    section(
                        "Filter",
                        widgets![
                            img_filter(color::grayscale(true)),
                            img_filter(color::sepia(true)),
                            img_filter(color::opacity(50.pct())),
                            img_filter(color::invert(true)),
                            img_filter(color::hue_rotate(-(90.deg()))),
                        ]
                    ),

                    v_stack! {
                        spacing = 30;
                        items = widgets![
                            section(
                                "Errors",
    
                                widgets![
                                    sub_title("File"),
                                    image("404.png"),
    
                                    sub_title("Web"),
                                    image("https://httpbin.org/delay/5"),
                                ]
                            ),
                            section(
                                "Sprite",
                                widgets![sprite(ctx.timers)]
                            ),
                            section(
                                "Large",
                                widgets![
                                    panorama_image(),
                                    large_image(),
                                ]
                            )                        
                        ];
                    }
                ]
            },
        )
    })
}

fn img_fit(fit: impl IntoVar<ImageFit>) -> impl Widget {
    let fit = fit.into_var();

    v_stack! {
        items_align = Alignment::TOP_LEFT;
        spacing = 5;

        items = widgets![
            sub_title(fit.map_debug()),
            image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                fit;
            }
        ]
    }
}

fn img_filter(filter: impl IntoVar<color::Filter>) -> impl Widget {
    let filter = filter.into_var();

    v_stack! {
        items_align = Alignment::TOP_LEFT;
        spacing = 2;

        items = widgets![
            sub_title(filter.map_debug()),
            image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                filter;
            }
        ]
    }
}

fn img_window(title: impl IntoVar<Text>, content: impl UiNode) -> Window {
    window! {
        title;
        content;

        state = WindowState::Maximized;
        background = checkerboard! {
            colors = rgb(20, 20, 20), rgb(40, 40, 40);
            cb_size = (16, 16);
        };

        // content show by all images when loading.
        image_loading_view = view_generator!(|ctx, _| {
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
        });

        // content show by all images that failed to load.
        image_error_view = view_generator!(|_, args: ImageErrorArgs| {
            text! {
                text = args.error;
                margin = 8;
                align = Alignment::CENTER;
                color = colors::RED;
                drop_shadow = {
                    offset: (0, 0),
                    blur_radius: 4,
                    color: colors::DARK_RED
                };
            }
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

fn large_image() -> impl Widget {
    button! {
        content = text("Large Image (205MB download)");
        on_click = hn!(|ctx, _| {
            ctx.services.windows().open(|_|img_window(
                "Wikimedia - Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB",
                image! {
                    source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                    limits = Some(ImageLimits { max_encoded_size: 300.megabytes(), max_decoded_size: 3.gigabytes() });

                    on_error = hn!(|_, args: &ImageErrorArgs| {
                        tracing::error!(target: "unexpected", "{}", args.error);
                    })
                }
            ));
        });
    }
}

fn panorama_image() -> impl Widget {
    button! {
        content = text("Panorama Image (100MB download)");
        on_click = hn!(|ctx, _| {
            ctx.services.windows().open(|_|img_window(
                "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB",
                scrollable! {
                    mode = ScrollMode::HORIZONTAL;
                    content = image! {
                        fit = ImageFit::Fill;
                        source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                        limits = Some(ImageLimits { max_encoded_size: 130.megabytes(), max_decoded_size: 1.gigabytes() });
                        on_error = hn!(|_, args: &ImageErrorArgs| {
                            tracing::error!(target: "unexpected", "{}", args.error);
                        });
                    };
                }
            ));
        });
    }
}

fn section(title: impl IntoVar<Text>, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items_align = Alignment::TOP_LEFT;

        items = widgets![
            text! {
                text = title;
                font_size = 20;
                background_color = colors::BLACK;
                border = {
                    widths: (5, 10),
                    sides: colors::BLACK,
                    radius: 0,
                };
            },
            v_stack! {
                spacing = 5;
                items_align = Alignment::TOP_LEFT;

                items;
            }
        ]
    }
}

fn sub_title(text: impl IntoVar<Text>) -> impl Widget {
    text! {
        text;

        font_size = 14;

        background_color = colors::BLACK;
        border = {
            widths: (2, 5),
            sides: colors::BLACK,
            radius: 0,
        };
    }
}
