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
                children = ui_list![
                    section(
                        "Sources",
                        ui_list![
                            sub_title("File"),
                            uniform_grid! {
                                columns = 4;
                                spacing = 2;
                                align = Align::CENTER;
                                children = ui_list![
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
                                image_scale_ppi = true;
                                source = ImageSource::render_node(RenderMode::Software, |_, _| container! {
                                    size = (180, 120);
                                    background_gradient = Line::to_bottom_left(), stops![hex!(#34753a), 40.pct(), hex!(#597d81)];
                                    font_size = 24;
                                    child_align = Align::CENTER;
                                    child = text("Rendered!");
                                })
                            }
                        ]
                    ),

                    section(
                        "Fit",
                        ui_list![
                            img_fit(ImageFit::None),
                            img_fit(ImageFit::Fill),
                            img_fit(ImageFit::Contain),
                            img_fit(ImageFit::Cover),
                            img_fit(ImageFit::ScaleDown),
                        ]
                    ),

                    section(
                        "Filter",
                        ui_list![
                            img_filter(filters::grayscale(true)),
                            img_filter(filters::sepia(true)),
                            img_filter(filters::opacity(50.pct())),
                            img_filter(filters::invert(true)),
                            img_filter(filters::hue_rotate(-(90.deg()))),
                            img_filter(filters::color_matrix([
                                2.0,  1.0,  1.0,  1.0,  0.0,
                                0.0,  1.0,  0.0,  0.0,  0.0,
                                0.0,  0.0,  1.0,  0.0,  0.0,
                                0.0,  0.0,  0.0,  1.0,  0.0,
                            ])),
                        ]
                    ),

                    v_stack! {
                        spacing = 30;
                        children = ui_list![
                            section(
                                "Errors",

                                ui_list![
                                    sub_title("File"),
                                    image("404.png"),

                                    sub_title("Web"),
                                    image("https://httpbin.org/delay/5"),
                                ]
                            ),
                            section(
                                "Sprite",
                                ui_list![sprite(ctx.timers)]
                            ),
                            section(
                                "Window",
                                ui_list![
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

fn img_fit(fit: impl IntoVar<ImageFit>) -> impl UiNode {
    let fit = fit.into_var();

    v_stack! {
        children_align = Align::TOP_LEFT;
        spacing = 5;

        children = ui_list![
            sub_title(fit.map_debug()),
            image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                image_fit = fit;
            }
        ]
    }
}

fn img_filter(filter: impl IntoVar<filters::Filter>) -> impl UiNode {
    let filter = filter.into_var();

    v_stack! {
        children_align = Align::TOP_LEFT;
        spacing = 2;

        children = ui_list![
            sub_title(filter.map(|f| {
                let s = format!("{f:?}");
                if s.starts_with("color_matrix") {
                    Text::from_static("color_matrix([...])")
                } else {
                    Text::from(s)
                }
            })),
            image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                filter;
            }
        ]
    }
}

fn sprite(timers: &mut Timers) -> impl UiNode {
    let timer = timers.interval((1.0 / 24.0).secs(), true);
    let label = var_from("play");

    v_stack! {
        align = Align::CENTER;
        children = ui_list![
            button! {
                child = text(label.clone());
                align = Align::CENTER;
                padding = (2, 3);
                on_click = hn!(timer, |ctx, _| {
                    let t = timer.get();
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
                image_crop = timer.map(|n| {
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

fn large_image() -> impl UiNode {
    button! {
        child = text("Large Image (205MB download)");
        on_click = hn!(|ctx, _| {
            Windows::req(ctx.services).open(|_|img_window(
                "Wikimedia - Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB",
                image! {
                    source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                    image_limits = Some(ImageLimits::none().with_max_encoded_size(300.megabytes()).with_max_decoded_size(3.gigabytes()));

                    on_error = hn!(|_, args: &ImageErrorArgs| {
                        tracing::error!(target: "unexpected", "{}", args.error);
                    })
                }
            ));
        });
    }
}

fn panorama_image() -> impl UiNode {
    button! {
        child = text("Panorama Image (100MB download)");
        on_click = hn!(|ctx, _| {
            Windows::req(ctx.services).open(|_|img_window(
                "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB",
                scroll! {
                    mode = ScrollMode::HORIZONTAL;
                    child = image! {
                        image_fit = ImageFit::Fill;
                        source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                        image_limits = Some(ImageLimits::none().with_max_encoded_size(130.megabytes()).with_max_decoded_size(1.gigabytes()));
                        on_error = hn!(|_, args: &ImageErrorArgs| {
                            tracing::error!(target: "unexpected", "{}", args.error);
                        });
                    };
                }
            ));
        });
    }
}

fn block_window_load_image() -> impl UiNode {
    let enabled = var(true);
    button! {
        child = text(enabled.map(|e| if *e { "Block Window Load (100MB download)" } else { "Blocking new window until image loads.." }.into()));
        enabled = enabled.clone();
        on_click = hn!(|ctx, _| {
            enabled.set(ctx, false);
            Windows::req(ctx.services).open(clone_move!(enabled, |_| img_window! {
                title = "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB";
                state = WindowState::Normal;

                child = scroll! {
                    child = image! {

                        // block window load until the image is ready to present or 5 minutes have elapsed.
                        // usually you want to set a shorter deadline, `true` converts to 1 second.
                        image_block_window_load = 5.minutes();

                        image_fit = ImageFit::Fill;
                        source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                        image_limits = Some(ImageLimits::none().with_max_encoded_size(130.megabytes()).with_max_decoded_size(1.gigabytes()));

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

fn center_viewport(msg: impl UiNode) -> impl UiNode {
    container! {
        // center the message on the scroll viewport:
        //
        // the large images can take a moment to decode in debug builds, but the size
        // is already known after read, so the "loading.." message ends-up off-screen
        // because it is centered on the image.
        x = zero_ui::widgets::scroll::SCROLL_HORIZONTAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vw() * fct);
        y = zero_ui::widgets::scroll::SCROLL_VERTICAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vh() * fct);
        zero_ui::core::widget_base::can_auto_hide = false;
        max_size = (1.vw(), 1.vh());
        child_align = Align::CENTER;

        child = msg;
    }
}

#[zero_ui::core::widget($crate::img_window)]
pub mod img_window {
    use super::*;

    inherit!(window);

    properties! {
        child_align = Align::CENTER;

        // render_mode = RenderMode::Software;

        state = WindowState::Maximized;
        window::properties::size = (1140, 770);// restore size

        background = checkerboard!();

        color_scheme = ColorScheme::Dark;

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
                text_color = loading_color();
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
                text_color = error_color();
                drop_shadow = {
                    offset: (0, 0),
                    blur_radius: 4,
                    color: error_color().darken(5.pct()),
                };
            })
        });

        // button color
        button::vis::base_colors = (rgb(0, 0, 40), rgb(0, 0, 255 - 40));
    }

    fn loading_color() -> Rgba {
        colors::LIGHT_GRAY
    }

    fn error_color() -> Rgba {
        colors::RED
    }
}
fn img_window(title: impl IntoVar<Text>, child: impl UiNode) -> Window {
    img_window!(title; child)
}

fn section(title: impl IntoVar<Text>, children: impl UiNodeList) -> impl UiNode {
    v_stack! {
        spacing = 5;
        children_align = Align::TOP_LEFT;

        children = ui_list![
            self::title(title),
            v_stack! {
                spacing = 5;
                children_align = Align::TOP_LEFT;

                children;
            }
        ]
    }
}

fn title(text: impl IntoVar<Text>) -> impl UiNode {
    text! {
        text;
        font_size = 20;
        background_color = colors::BLACK;
        padding = (5, 10);
    }
}

fn sub_title(text: impl IntoVar<Text>) -> impl UiNode {
    text! {
        text;

        font_size = 14;

        background_color = colors::BLACK;
        padding = (2, 5);
    }
}
