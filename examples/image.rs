#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::core::task::http;
use zero_ui::widgets::image::{img_error_fn, img_loading_fn, ImageErrorArgs};
use zero_ui::{
    core::image::{ImageLimits, IMAGES},
    prelude::*,
};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::init();

    // let rec = examples_util::record_profile("image");

    zero_ui_view::run_same_process(app_main);

    // app_main();
    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        // by default all "ImageSource::Download" requests are blocked, the limits can be set globally
        // in here and overridden for each image with the "img_limits" property.
        IMAGES.limits().modify(|l| {
            l.to_mut().allow_uri = zero_ui::core::image::UriFilter::AllowAll;
        });

        // setup a file cache so we don't download the images every run.
        http::set_default_client_init(move || {
            http::Client::builder()
                .cache(http::FileSystemCache::new(examples_util::temp_dir("image")).unwrap())
                .cache_mode(img_cache_mode)
                .build()
        })
        .unwrap();

        ImgWindow!(
            "Image Example",
            Stack! {
                direction = StackDirection::left_to_right();
                spacing = 30;
                children = ui_vec![
                    section(
                        "Sources",
                        ui_vec![
                            sub_title("File"),
                            Grid! {
                                columns = ui_vec![grid::Column!(1.lft()); 4];
                                auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
                                spacing = 2;
                                align = Align::CENTER;
                                cells= ui_vec![
                                    Image!("examples/res/image/Luma8.png"),
                                    Image!("examples/res/image/Luma16.png"),
                                    Image!("examples/res/image/LumaA8.png"),
                                    Image!("examples/res/image/LumaA16.png"),
                                    Image!("examples/res/image/RGB8.png"),
                                    Image!("examples/res/image/RGB16.png"),
                                    Image!("examples/res/image/RGBA8.png"),
                                    Image!("examples/res/image/RGBA16.png"),
                                ]
                          },

                            sub_title("Web"),
                            Image! {
                                source = "https://httpbin.org/image";
                                size = (200, 150);
                            },

                            sub_title("Web With Format"),
                            Image! {
                                source = (Uri::from_static("https://httpbin.org/image"), "image/png");
                                size = (200, 150);
                            },
                            sub_title("Render"),
                            Image! {
                                img_scale_ppi = true;
                                source = ImageSource::render_node(RenderMode::Software, |_| Container! {
                                    size = (180, 120);
                                    background_gradient = Line::to_bottom_left(), stops![hex!(#34753a), 40.pct(), hex!(#597d81)];
                                    font_size = 24;
                                    child_align = Align::CENTER;
                                    child = Text!("Rendered!");
                                })
                            },
                            // sub_title("AVIF"),
                            // Image! {
                            //     source = "https://raw.githubusercontent.com/link-u/avif-sample-images/master/fox.profile0.8bpc.yuv420.avif";
                            //     size = (200, 150);
                            // },
                        ]
                    ),

                    section(
                        "Fit",
                        ui_vec![
                            img_fit(ImageFit::None),
                            img_fit(ImageFit::Fill),
                            img_fit(ImageFit::Contain),
                            img_fit(ImageFit::Cover),
                            img_fit(ImageFit::ScaleDown),
                        ]
                    ),

                    section(
                        "Filter",
                        ui_vec![
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

                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 30;
                        children = ui_vec![
                            section(
                                "Errors",

                                ui_vec![
                                    sub_title("File"),
                                    Image!("404.png"),

                                    sub_title("Web"),
                                    Image!("https://httpbin.org/delay/5"),
                                ]
                            ),
                            section(
                                "Sprite",
                                ui_vec![sprite()]
                            ),
                            section(
                                "Window",
                                ui_vec![
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

    Stack! {
        direction = StackDirection::top_to_bottom();
        children_align = Align::TOP_LEFT;
        spacing = 5;

        children = ui_vec![
            sub_title(fit.map_debug()),
            Image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                img_fit = fit;
                border = {
                    widths: 1,
                    sides: BorderSides::dashed(colors::GRAY),
                };
            }
        ]
    }
}

fn img_filter(filter: impl IntoVar<filters::Filter>) -> impl UiNode {
    let filter = filter.into_var();

    Stack! {
        direction = StackDirection::top_to_bottom();
        children_align = Align::TOP_LEFT;
        spacing = 2;

        children = ui_vec![
            sub_title(filter.map(|f| {
                let s = format!("{f:?}");
                if s.starts_with("color_matrix") {
                    Txt::from_static("color_matrix([...])")
                } else {
                    Txt::from(s)
                }
            })),
            Image! {
                source = "examples/res/image/zdenek-machacek-unsplash.jpg";
                size = (200, 100);
                filter;
            }
        ]
    }
}

fn sprite() -> impl UiNode {
    let timer = TIMERS.interval((1.0 / 24.0).secs(), true);
    let label = var_from("play");

    Stack! {
        direction = StackDirection::top_to_bottom();
        align = Align::CENTER;
        children = ui_vec![
            Button! {
                child = Text!(label.clone());
                align = Align::CENTER;
                padding = (2, 3);
                on_click = hn!(timer, |_| {
                    let t = timer.get();
                    if t.is_paused() {
                        t.play(false);
                    } else {
                        t.pause();
                    }
                    label.set(if t.is_paused() { "play" } else { "pause" });
                });
            },
            Image! {
                source = "examples/res/image/player_combat_sheet-10-96x84-CC0.png";
                size = (96, 84);
                border = {
                    widths: 1,
                    sides: BorderSides::dashed(colors::GRAY),
                };
                corner_radius = 4;
                img_crop = timer.map(|n| {
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
    Button! {
        child = Text!("Large Image (205MB download)");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                ImgWindow! {
                    title = "Wikimedia - Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB, downscale to fit 8,000 × 8,000";
                    child_align = Align::FILL;
                    child = Image! {
                        source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                        img_limits = Some(ImageLimits::none().with_max_encoded_len(300.megabytes()).with_max_decoded_len(3.gigabytes()));
                        img_downscale = Px(8000);

                        on_error = hn!(|args: &ImageErrorArgs| {
                            tracing::error!(target: "unexpected", "{}", args.error);
                        });

                        img_loading_fn = wgt_fn!(|_| {
                            // thumbnail
                            Stack! {
                                children = ui_vec![
                                    Image! {
                                        source = "https://upload.wikimedia.org/wikipedia/commons/thumb/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg/757px-Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
                                    },
                                    loading(),
                                ];
                            }
                        });
                    }
                }
            });
        });
    }
}

fn panorama_image() -> impl UiNode {
    Button! {
        child = Text!("Panorama Image (100MB download)");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                ImgWindow!(
                    "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB",
                    Scroll! {
                        mode = ScrollMode::HORIZONTAL;
                        child = Image! {
                            img_fit = ImageFit::Fill;
                            source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                            img_limits = Some(ImageLimits::none().with_max_encoded_len(130.megabytes()).with_max_decoded_len(1.gigabytes()));
                            on_error = hn!(|args: &ImageErrorArgs| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                        };
                    }
                )
            });
        });
    }
}

fn block_window_load_image() -> impl UiNode {
    let enabled = var(true);
    Button! {
        child = Text!(enabled.map(|e| if *e { "Block Window Load (100MB download)" } else { "Blocking new window until image loads.." }.into()));
        enabled = enabled.clone();
        on_click = hn!(|_| {
            enabled.set(false);
            WINDOWS.open(async_clmv!(enabled, {
                ImgWindow! {
                    title = "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB";
                    state = WindowState::Normal;

                    child = Scroll! {
                        child = Image! {

                            // block window load until the image is ready to present or 5 minutes have elapsed.
                            // usually you want to set a shorter deadline, `true` converts to 1 second.
                            img_block_window_load = 5.minutes();

                            img_fit = ImageFit::Fill;
                            source = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
                            img_limits = Some(ImageLimits::none().with_max_encoded_len(130.megabytes()).with_max_decoded_len(1.gigabytes()));

                            on_error = hn!(|args: &ImageErrorArgs| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                        }
                    };

                    on_load = hn!(enabled, |_| {
                        enabled.set(true);
                    });
                }
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
    Container! {
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

#[zero_ui::core::widget($crate::ImgWindow {
    ($title:expr, $child:expr $(,)?) => {
        title = $title;
        child = $child;
    }
})]
pub struct ImgWindow(Window);
impl ImgWindow {
    #[zero_ui::core::widget(on_start)]
    fn on_start(&mut self) {
        zero_ui::core::widget_dft! {
            self;
            // renderer_debug = {
            //     use zero_ui::core::render::webrender_api::DebugFlags;
            //     DebugFlags::TEXTURE_CACHE_DBG | DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED
            // };

            // render_mode = RenderMode::Software;

            child_align = Align::CENTER;


            state = WindowState::Maximized;
            size = (1140, 770);// restore size

            icon = "examples/res/image/RGB8.png";
            background = Checkerboard!();

            color_scheme = ColorScheme::Dark;

            // content shown by all images when loading.
            img_loading_fn = wgt_fn!(|_| loading());

            // content shown by all images that failed to load.
            img_error_fn = wgt_fn!(|args: ImageErrorArgs| {
                center_viewport(Text! {
                    txt = args.error;
                    margin = 8;
                    align = Align::CENTER;
                    txt_color = error_color();
                    drop_shadow = {
                        offset: (0, 0),
                        blur_radius: 4,
                        color: error_color().darken(5.pct()),
                    };
                })
            });

            // button color
            button::base_colors = (rgb(0, 0, 40), rgb(0, 0, 255 - 40));
        }
    }
}
fn loading_color() -> Rgba {
    colors::LIGHT_GRAY
}

fn error_color() -> Rgba {
    colors::RED
}

pub fn loading() -> impl UiNode {
    let mut dots_count = 3;
    let msg = TIMERS.interval(300.ms(), false).map(move |_| {
        dots_count += 1;
        if dots_count == 8 {
            dots_count = 0;
        }
        formatx!("loading{:.^dots_count$}", "")
    });

    center_viewport(Text! {
        txt = msg;
        txt_color = loading_color();
        margin = 8;
        width = 80;
        font_style = FontStyle::Italic;
        drop_shadow = {
            offset: (0, 0),
            blur_radius: 4,
            color: loading_color().darken(5.pct()),
        };
    })
}

fn section(title: impl IntoVar<Txt>, children: impl UiNodeList) -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children_align = Align::TOP_LEFT;

        children = ui_vec![
            self::title(title),
            Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                children_align = Align::TOP_LEFT;

                children;
            }
        ]
    }
}

fn title(txt: impl IntoVar<Txt>) -> impl UiNode {
    Text! {
        txt;
        font_size = 20;
        background_color = colors::BLACK;
        padding = (5, 10);
    }
}

fn sub_title(txt: impl IntoVar<Txt>) -> impl UiNode {
    Text! {
        txt;

        font_size = 14;

        background_color = colors::BLACK;
        padding = (2, 5);
    }
}
