#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::core::task::http;
use zero_ui::core::{
    image::{ImageLimits, Images},
    timer::Timers,
};
use zero_ui::prelude::*;
use zero_ui::widgets::image::properties::{image_error_view, image_loading_view, ImageErrorArgs};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("image");

    zero_ui_view::run_same_process(app_main);

    // app_main();
    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        // by default all "ImageSource::Download" requests are blocked, the limits can be set globally
        // in here and overridden for each image with the "limits" property.
        Images::req(ctx.services).limits.allow_uri = zero_ui::core::image::UriFilter::AllowAll;

        // setup a file cache so we don't download the images every run.
        http::set_default_client_init(move || {
            http::Client::builder()
                .cache(http::FileSystemCache::new(examples_util::temp_dir("image")).unwrap())
                .cache_mode(img_cache_mode)
                .build()
        })
        .unwrap();

        img_window(
            "Image Example",
            h_stack! {
                spacing = 30;
                items = widgets![
                    section(
                        "Sources",
                        widgets![
                            sub_title("File"),
                            uniform_grid! {
                                columns = 4;
                                spacing = 2;
                                align = Align::CENTER;
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
                            image! {
                                source = "https://httpbin.org/image";
                                size = (200, 150);
                            },

                            sub_title("Web With Format"),
                            image! {
                                source = (Uri::from_static("https://httpbin.org/image"), "image/png");
                                size = (200, 150);
                            },
                            sub_title("Render"),
                            image! {
                                scale_ppi = true;
                                source = ImageSource::render(RenderMode::Software, |_| container! {
                                    size = (180, 120);
                                    background_gradient = Line::to_bottom_left(), stops![hex!(#34753a), 40.pct(), hex!(#597d81)];
                                    font_size = 24;
                                    content_align = Align::CENTER;
                                    content = text("Rendered!");
                                })
                            }
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
                                "Window",
                                widgets![
                                    panorama_image(),
                                    block_window_load_image(),
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
        items_align = Align::TOP_LEFT;
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
        items_align = Align::TOP_LEFT;
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

fn sprite(timers: &mut Timers) -> impl Widget {
    let timer = timers.interval((1.0 / 24.0).secs(), true);
    let label = var_from("play");

    v_stack! {
        align = Align::CENTER;
        items = widgets![
            button! {
                content = text(label.clone());
                align = Align::CENTER;
                padding = (2, 3);
                on_click = hn!(timer, |ctx, _| {
                    let t = timer.get(ctx);
                    if t.is_paused() {
                        t.play(false);
                    } else {
                        t.pause();
                    }
                    label.set(ctx, if t.is_paused() { "play" } else { "pause" });
                });
            },
            image! {
                source = "examples/res/image/player_combat_sheet-10-96x84-CC0.png";
                size = (96, 84);
                border = {
                    widths: 1,
                    sides: BorderSides::dashed(colors::GRAY),
                };
                corner_radius = 4;
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
            Windows::req(ctx.services).open(|_|img_window(
                "Wikimedia - Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB",
                image! {
                    source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                    limits = Some(ImageLimits::none().with_max_encoded_size(300.megabytes()).with_max_decoded_size(3.gigabytes()));

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
            Windows::req(ctx.services).open(|_|img_window(
                "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB",
                scroll! {
                    mode = ScrollMode::HORIZONTAL;
                    content = image! {
                        fit = ImageFit::Fill;
                        source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                        limits = Some(ImageLimits::none().with_max_encoded_size(130.megabytes()).with_max_decoded_size(1.gigabytes()));
                        on_error = hn!(|_, args: &ImageErrorArgs| {
                            tracing::error!(target: "unexpected", "{}", args.error);
                        });
                    };
                }
            ));
        });
    }
}

fn block_window_load_image() -> impl Widget {
    let enabled = var(true);
    button! {
        content = text(enabled.map(|e| if *e { "Block Window Load (100MB download)" } else { "Blocking new window until image loads.." }.into()));
        enabled = enabled.clone();
        on_click = hn!(|ctx, _| {
            enabled.set(ctx, false);
            Windows::req(ctx.services).open(clone_move!(enabled, |_| img_window! {
                title = "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB";
                state = WindowState::Normal;

                content = scroll! {
                    content = image! {

                        // block window load until the image is ready to present or 10 seconds have elapsed.
                        // usually you want to set a shorter deadline, `true` converts to 1 second.
                        block_window_load = 5.minutes();

                        fit = ImageFit::Fill;
                        source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                        limits = Some(ImageLimits::none().with_max_encoded_size(130.megabytes()).with_max_decoded_size(1.gigabytes()));

                        on_error = hn!(|_, args: &ImageErrorArgs| {
                            tracing::error!(target: "unexpected", "{}", args.error);
                        });
                    }
                };

                on_load = hn!(enabled, |ctx, _| {
                    enabled.set(ctx, true);
                });
            }));
        });
    }
}

fn img_cache_mode(req: &task::http::Request) -> http::CacheMode {
    if let Some(a) = req.uri().authority() {
        if a.host().contains("wikimedia.org") {
            // Wikimedia not configured for caching.
            return http::CacheMode::Permanent;
        }
    }
    http::CacheMode::default()
}

fn center_viewport(msg: impl Widget) -> impl Widget {
    container! {
        // center the message on the scroll viewport:
        //
        // the large images can take a moment to decode in debug builds, but the size
        // is already known after read, so the "loading.." message ends-up off-screen
        // because it is centered on the image.
        x = zero_ui::widgets::scroll::ScrollHorizontalOffsetVar::new().map(|&fct| Length::Relative(fct) - 1.vw() * fct);
        y = zero_ui::widgets::scroll::ScrollVerticalOffsetVar::new().map(|&fct| Length::Relative(fct) - 1.vh() * fct);
        max_size = (1.vw(), 1.vh());
        content_align = Align::CENTER;

        content = msg;
    }
}

#[zero_ui::core::widget($crate::img_window)]
pub mod img_window {
    use super::*;

    inherit!(zero_ui::widgets::window);

    properties! {
        content_align = Align::CENTER;

        // render_mode = zero_ui::core::window::RenderMode::Software;

        state = WindowState::Maximized;
        size = (1140, 770);// restore size

        background = checkerboard! {
            colors = rgb(20, 20, 20), rgb(40, 40, 40);
            cb_size = (16, 16);
        };

        // content shown by all images when loading.
        image_loading_view = view_generator!(|ctx, _| {
            let mut dots_count = 3;
            let msg = ctx.timers.interval(300.ms(), false).map(move |_| {
                dots_count += 1;
                if dots_count == 8 {
                    dots_count = 0;
                }
                formatx!("loading{:.^dots_count$}", "")
            });

            center_viewport(text! {
                text = msg;
                color = loading_color();
                margin = 8;
                width = 80;
                font_style = FontStyle::Italic;
                drop_shadow = {
                    offset: (0, 0),
                    blur_radius: 4,
                    color: loading_color().darken(5.pct()),
                };
            })
        });

        // content shown by all images that failed to load.
        image_error_view = view_generator!(|_, args: ImageErrorArgs| {
            center_viewport(text! {
                text = args.error;
                margin = 8;
                align = Align::CENTER;
                color = error_color();
                drop_shadow = {
                    offset: (0, 0),
                    blur_radius: 4,
                    color: error_color().darken(5.pct()),
                };
            })
        });


    }

    fn new_child_context(child: impl UiNode) -> impl UiNode {
        // button theme
        container! {
            content = child;

            button::theme::background_color = button_color().lighten(4.pct());
            button::theme::border = {
                widths: 1,
                sides: button_color().lighten(4.pct()),
            };

            button::theme::hovered::background_color = button_color().lighten(2.fct());
            button::theme::hovered::border_sides = button_color().lighten(1.fct());

            button::theme::pressed::background_color = button_color().lighten(3.fct());
            button::theme::pressed::border_sides = button_color().lighten(2.fct());
        }
    }

    fn button_color() -> Rgba {
        rgb(0, 0, 20)
    }

    fn loading_color() -> Rgba {
        colors::LIGHT_GRAY
    }

    fn error_color() -> Rgba {
        colors::RED
    }
}
fn img_window(title: impl IntoVar<Text>, content: impl UiNode) -> Window {
    img_window!(title; content)
}

fn section(title: impl IntoVar<Text>, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items_align = Align::TOP_LEFT;

        items = widgets![
            self::title(title),
            v_stack! {
                spacing = 5;
                items_align = Align::TOP_LEFT;

                items;
            }
        ]
    }
}

fn title(text: impl IntoVar<Text>) -> impl Widget {
    text! {
        text;
        font_size = 20;
        background_color = colors::BLACK;
        padding = (5, 10);
    }
}

fn sub_title(text: impl IntoVar<Text>) -> impl Widget {
    text! {
        text;

        font_size = 14;

        background_color = colors::BLACK;
        padding = (2, 5);
    }
}
