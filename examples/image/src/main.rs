//! Demonstrates image loading, displaying, animated sprites, rendering, pasting.

use std::path::PathBuf;

use zng::{
    app,
    checkerboard::Checkerboard,
    clipboard,
    color::{
        filter::{Filter, drop_shadow, filter, mix_blend},
        gradient::stops,
    },
    image::{self, IMAGES, ImageFit, ImageLimits, ImgErrorArgs, img_error_fn, img_loading_fn, mask::mask_image},
    layout::{align, margin, padding, size},
    mouse,
    prelude::*,
    scroll::ScrollMode,
    task::http,
    widget::{BorderSides, background_color, border},
    window::{RenderMode, WindowState},
};
use zng_wgt_webrender_debug as wr;

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();

    APP.defaults().run_window(async {
        // by default all "ImageSource::Download" requests are blocked and "ImageSource::Read"
        // is limited to only the `zng::env::res`. The limits can be set globally in here and overridden 
        // for each image with the "img_limits" property.
        IMAGES.limits().modify(|l| {
            let l = l.value_mut();
            l.allow_uri = image::UriFilter::AllowAll;
            l.allow_path = image::PathFilter::AllowAll;
        });

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
                                cells = {
                                    fn img(source: &str) -> UiNode {
                                        Image! {
                                            grid::cell::at = grid::cell::AT_AUTO;
                                            source = zng::env::res(source);
                                        }
                                    }
                                    ui_vec![
                                        img("Luma8.png"),
                                        img("Luma16.png"),
                                        img("LumaA8.png"),
                                        img("LumaA16.png"),
                                        img("RGB8.png"),
                                        img("RGB16.png"),
                                        img("RGBA8.png"),
                                        img("RGBA16.png"),
                                    ]
                                }
                            },

                            sub_title("Web"),
                            Image! {
                                source = "https://httpbin.org/image";
                                size = (200, 150);
                            },

                            sub_title("Web With Format"),
                            Image! {
                                source = (http::Uri::from_static("https://httpbin.org/image"), "image/png");
                                size = (200, 150);
                            },
                            sub_title("Render"),
                            Image! {
                                size = (180, 120);
                                source = ImageSource::render_node(RenderMode::Software, |_| Container! {
                                    size = (180, 120);
                                    widget::background_gradient = layout::Line::to_bottom_left(), stops![hex!(#34753a), 40.pct(), hex!(#597d81)];
                                    text::font_size = 24;
                                    child_align = Align::CENTER;
                                    child = Text!("Rendered!");
                                })
                            },
                            sub_title("Render Mask"),
                            Image! {
                                source = zng::env::res("zdenek-machacek-unsplash.jpg");
                                size = (200, 120);
                                mask_image = ImageSource::render_node(RenderMode::Software, |_| Text! {
                                    txt = "Mask";
                                    txt_align = Align::CENTER;
                                    font_size = 78;
                                    font_weight = FontWeight::BOLD;
                                    size = (200, 120);
                                });
                            },
                            sub_title("SVG"),
                            Image! {
                                source = zng::env::res("Ghostscript_Tiger.svg");
                                size = (200, 150);
                            },
                        ]
                    ),

                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 30;
                        children = ui_vec![
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
                                "Mix-Blend",
                                ui_vec![
                                    Image! {
                                        source = zng::env::res("zdenek-machacek-unsplash.jpg");
                                        size = (200, 100);
                                        widget::foreground = Text! {
                                            mix_blend = color::MixBlendMode::ColorDodge;
                                            font_color = colors::RED;
                                            txt = "Blend";
                                            txt_align = Align::CENTER;
                                            font_size = 58;
                                            font_weight = FontWeight::BOLD;
                                        };
                                    }
                                ]
                            )
                        ]
                    },

                    section(
                        "Filter",
                        ui_vec![
                            img_filter(Filter::new_grayscale(true)),
                            img_filter(Filter::new_sepia(true)),
                            img_filter(Filter::new_opacity(50.pct())),
                            img_filter(Filter::new_invert(true)),
                            img_filter(Filter::new_hue_rotate(-(90.deg()))),
                            img_filter({
                                #[rustfmt::skip]
                                let custom = Filter::new_color_matrix([
                                    2.0, 1.0, 1.0, 1.0, 0.0,
                                    0.0, 1.0, 0.0, 0.0, 0.0,
                                    0.0, 0.0, 1.0, 0.0, 0.0,
                                    0.0, 0.0, 0.0, 1.0, 0.0,
                                ]);
                                custom
                            }),
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
                                    repeat_image(),
                                    open_or_paste_image(),
                                    exif_rotated(),
                                    ppi_scaled(),
                                    color_profiles(),
                                ]
                            )
                        ];
                    }
                ]
            },
        )
    })
}

fn img_fit(fit: impl IntoVar<ImageFit>) -> UiNode {
    let fit = fit.into_var();

    Stack! {
        direction = StackDirection::top_to_bottom();
        children_align = Align::TOP_LEFT;
        spacing = 5;

        children = ui_vec![
            sub_title(fit.map_debug(false)),
            Image! {
                source = zng::env::res("zdenek-machacek-unsplash.jpg");
                size = (200, 100);
                img_fit = fit;
                border = {
                    widths: 1,
                    sides: BorderSides::dashed(colors::GRAY),
                };
            }
        ];
    }
}

fn img_filter(filter: impl IntoVar<Filter>) -> UiNode {
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
                source = zng::env::res("zdenek-machacek-unsplash.jpg");
                size = (200, 100);
                filter;
            }
        ];
    }
}

fn sprite() -> UiNode {
    let timer = timer::TIMERS.interval((1.0 / 24.0).secs(), true);
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
                source = zng::env::res("player_combat_sheet-10-96x84-CC0.png");
                size = (96, 84);
                border = {
                    widths: 1,
                    sides: BorderSides::dashed(colors::GRAY),
                };
                widget::corner_radius = 4;
                img_crop = timer.map(|n| {
                    if n.count() == 10 {
                        n.set_count(0);
                    }
                    let offset = n.count() as i32 * 96;
                    (96.px(), 84.px()).at(offset.px(), 0.px())
                });
            },
        ];
    }
}

fn large_image() -> UiNode {
    let title = "Wikimedia - Starry Night - 30,000 × 23,756 pixels, file size: 205.1 MB, decoded: 2.8 GB";
    let source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
    let thumbnail_source = "https://upload.wikimedia.org/wikipedia/commons/thumb/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg/757px-Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
    Button! {
        child = Text!("Large Image (205MB download)");
        on_click = hn!(|_| {
            WINDOWS.open(async move {
                let mouse_pan = var(false);
                let mode = var(ScrollMode::NONE);
                ImgWindow! {
                    title;
                    child_align = Align::FILL;
                    child = Scroll! {
                        mode = mode.clone();
                        mouse_pan = mouse_pan.clone();
                        ctrl_scroll = true;
                        child = Image! {
                            source;
                            img_limits = Some(
                                ImageLimits::none()
                                    .with_max_encoded_len(300.megabytes())
                                    .with_max_decoded_len(3.gigabytes()),
                            );

                            on_error = hn!(|args| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                            on_load = hn!(|_| {
                                mode.set(ScrollMode::ZOOM);
                                mouse_pan.set(true);
                            });

                            img_loading_fn = wgt_fn!(|_| {
                                // thumbnail
                                Stack! {
                                    children = ui_vec![
                                        Image! {
                                            source = thumbnail_source;
                                        },
                                        loading(),
                                    ];
                                }
                            });

                            // let actual image scale, better performance when
                            // showing entire image as it does not need to render a full size
                            // texture just to downscale. Renderer implements mipmaps only for images.
                            zng::scroll::zoom_size_only = true;

                            // better for photo viewers
                            img_auto_scale = false;
                        };
                    };
                }
            });
        });
    }
}

fn panorama_image() -> UiNode {
    let title = "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB";
    let source =
        "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
    Button! {
        child = Text!("Panorama Image (100MB download)");
        on_click = hn!(|_| {
            WINDOWS.open(async move {
                ImgWindow!(
                    title,
                    Scroll! {
                        mode = ScrollMode::HORIZONTAL;
                        mouse_pan = true;
                        ctrl_scroll = true;
                        child = Image! {
                            img_fit = ImageFit::Fill;
                            source;
                            img_limits = Some(
                                ImageLimits::none()
                                    .with_max_encoded_len(130.megabytes())
                                    .with_max_decoded_len(1.gigabytes()),
                            );
                            on_error = hn!(|args| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                        };
                    }
                )
            });
        });
    }
}

fn block_window_load_image() -> UiNode {
    let title = "Wikimedia - Along the River During the Qingming Festival - 56,531 × 1,700 pixels, file size: 99.32 MB";
    let source =
        "https://upload.wikimedia.org/wikipedia/commons/2/2c/Along_the_River_During_the_Qingming_Festival_%28Qing_Court_Version%29.jpg";
    let enabled = var(true);
    Button! {
        child = Text!(enabled.map(|e| {
            if *e {
                "Block Window Load (100MB download)"
            } else {
                "Blocking new window until image loads.."
            }
            .into()
        }));
        widget::enabled = enabled.clone();
        on_click = hn!(|_| {
            enabled.set(false);
            WINDOWS.open(async_clmv!(enabled, {
                ImgWindow! {
                    title;
                    state = WindowState::Normal;

                    child = Scroll! {
                        mouse_pan = true;
                        ctrl_scroll = true;
                        child = Image! {
                            // block window load until the image is ready to present or 5 minutes have elapsed.
                            // usually you want to set a shorter deadline, `true` converts to 1 second.
                            img_block_window_load = 5.minutes();

                            img_fit = ImageFit::Fill;
                            source;
                            img_limits = Some(
                                ImageLimits::none()
                                    .with_max_encoded_len(130.megabytes())
                                    .with_max_decoded_len(1.gigabytes()),
                            );

                            on_error = hn!(|args| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                        };
                    };

                    on_load = hn!(enabled, |_| {
                        enabled.set(true);
                    });
                }
            }));
        });
    }
}

fn repeat_image() -> UiNode {
    let title = "Wikimedia - Turtle seamless pattern - 1,000 × 1,000 pixels, file size: 1.49 MB";
    let source = "https://upload.wikimedia.org/wikipedia/commons/9/91/Turtle_seamless_pattern.jpg";
    Button! {
        child = Text!("Repeat Image (2 MB download)");
        on_click = hn!(|_| {
            WINDOWS.open(async move {
                let show_pattern = var(false);
                ImgWindow!(
                    title,
                    Scroll! {
                        mode = ScrollMode::HORIZONTAL;
                        // demo `background_img`
                        child = Wgt! {
                            widget::background_img = source;
                            widget::background_img_fit = ImageFit::None;
                            widget::background_img_repeat = true;
                            widget::background_img_repeat_spacing =
                                show_pattern
                                    .map(|&s| layout::Size::from(if s { 10 } else { 0 }))
                                    .easing(300.ms(), easing::linear),
                            ;
                            size = (10000, 100.pct());
                            mouse::on_mouse_input = hn!(show_pattern, |args| {
                                show_pattern.set(matches!(args.state, mouse::ButtonState::Pressed));
                            });
                            zng::image::on_error = hn!(|args| {
                                tracing::error!(target: "unexpected", "{}", args.error);
                            });
                        };
                        // demo `Image!`
                        // child = Image! {
                        //     source;
                        //     img_fit = ImageFit::None;
                        //     img_repeat = true;
                        //     img_repeat_spacing = show_pattern
                        //         .map(|&s| layout::Size::from(if s { 10 } else { 0 }))
                        //         .easing(300.ms(), easing::linear);
                        //     size = (10000, 100.pct());
                        //     mouse::on_mouse_input = hn!(show_pattern, |args| {
                        //         show_pattern.set(matches!(args.state, mouse::ButtonState::Pressed));
                        //     });
                        //     on_error = hn!(|args| {
                        //         tracing::error!(target: "unexpected", "{}", args.error);
                        //     });
                        // };
                    }
                )
            });
        });
    }
}

fn open_or_paste_image() -> UiNode {
    Button! {
        child = Text!("Open or Paste Image");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                let source = var(ImageSource::flood(layout::PxSize::splat(layout::Px(1)), colors::BLACK, None));
                ImgWindow! {
                    title = "Open or Paste Image";

                    app::on_open = async_hn!(source, |_| {
                        if let Some(img) = open_dialog().await {
                            source.set(img);
                        }
                    });
                    clipboard::on_paste = hn!(source, |_| {
                        if let Some(img) = clipboard::CLIPBOARD.image().ok().flatten() {
                            source.set(img);
                        }
                    });

                    child_align = Align::FILL;
                    child = {
                        use layout::PxSize;
                        let img_size = var_getter::<PxSize>();
                        let img_wgt_size = var_getter::<PxSize>();
                        let menu_wgt_size = var_getter::<PxSize>();
                        let show_menu = merge_var!(img_size.clone(), img_wgt_size.clone(), menu_wgt_size.clone(), |img, wgt, menu| {
                            img.height < wgt.height - menu.height
                        });
                        Stack!(ui_vec![
                            Image! {
                                img_fit = ImageFit::ScaleDown;
                                source;
                                get_img_layout_size = img_size;
                                layout::actual_size_px = img_wgt_size;
                                on_error = hn!(|args| {
                                    tracing::error!(target: "unexpected", "{}", args.error);
                                });
                            },
                            Stack! {
                                children = {
                                    let cmd_btn = |cmd: zng::event::Command| {
                                        let cmd = cmd.scoped(WINDOW.id());
                                        Button! {
                                            padding = (2, 5);
                                            child_left = cmd.icon().present_data(());
                                            child = Text!(cmd.name_with_shortcut());
                                            cmd;
                                        }
                                    };
                                    ui_vec![
                                        cmd_btn(app::OPEN_CMD.scoped(WINDOW.id())),
                                        cmd_btn(clipboard::PASTE_CMD.scoped(WINDOW.id())),
                                    ]
                                };

                                layout::actual_size_px = menu_wgt_size;

                                align = Align::TOP;
                                direction = StackDirection::left_to_right();
                                spacing = 5;
                                margin = 5;

                                #[easing(200.ms())]
                                color::filter::opacity = 10.pct();
                                when *#gesture::is_hovered || *#{show_menu} {
                                    color::filter::opacity = 100.pct();
                                }
                            }
                        ])
                    };
                }
            });
        });
    }
}

fn exif_rotated() -> UiNode {
    Button! {
        child = Text!("Exif Rotated");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                fn example(file: &'static str) -> UiNode {
                    Image! {
                        zng::container::child_top = Text! {
                            txt = file;
                            txt_align = Align::CENTER;
                            font_weight = FontWeight::BOLD;
                        };
                        source = zng::env::res(file);
                    }
                }
                Window! {
                    title = "Exif Rotated";
                    child_top = Text! {
                        txt = "all arrows must point right";
                        txt_align = Align::CENTER;
                        font_size = 2.em();
                        margin = 20;
                    };
                    auto_size = true;
                    padding = 10;
                    child = Stack!(
                        left_to_right,
                        10,
                        ui_vec![example("exif rotated.jpg"), example("exif rotated.tif"),]
                    );
                }
            });
        });
    }
}

fn ppi_scaled() -> UiNode {
    use zng::image::ImageAutoScale;
    Button! {
        child = Text!("PPI Scaled");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                fn example(file: &'static str) -> UiNode {
                    Image! {
                        zng::container::child_top = Text! {
                            txt = file;
                            txt_align = Align::CENTER;
                            font_weight = FontWeight::BOLD;
                        };
                        source = zng::env::res(format!("{file}.png"));
                    }
                }
                let enabled = var(ImageAutoScale::Factor);
                Window! {
                    title = "PPI Scaled";
                    child_top = Toggle! {
                        layout::align = Align::CENTER;
                        checked = enabled.map_bidi(
                            |a| matches!(a, ImageAutoScale::Density),
                            |e| if *e { ImageAutoScale::Density } else { ImageAutoScale::Factor },
                        );
                        child = Text!(enabled.map(|e| formatx!("image_scale_density = {e:?}")));
                        margin = 20;
                    };
                    auto_size = true;
                    padding = 10;
                    child = Stack! {
                        direction = StackDirection::left_to_right();
                        spacing = 10;
                        zng::image::img_auto_scale = enabled;
                        children = ui_vec![example("300x300@96dpi"), example("600x600@192dpi"),];
                    };
                }
            });
        });
    }
}

fn color_profiles() -> UiNode {
    Button! {
        child = Text!("Color Profiles");
        on_click = hn!(|_| {
            WINDOWS.open(async {
                Window! {
                    title = "Color Profiles";
                    child_top = Text! {
                        txt = "All rows must have the same color";
                        txt_align = Align::CENTER;
                        margin = 20;
                    };
                    auto_size = true;
                    padding = 10;
                    child = Grid! {
                        align = Align::TOP;
                        rows = ui_vec![grid::Row!(), grid::Row!(1.lft()),];
                        auto_grow_mode = grid::AutoGrowMode::columns();
                        auto_grow_fn = wgt_fn!(|_| grid::Column!(50));
                        text::font_size = 12;
                        text::txt_align = Align::CENTER;
                        cells = ui_vec![
                            Text! {
                                grid::cell::at = (0, 0);
                                txt = "Colors";
                            },
                            Stack! {
                                grid::cell::at = (0, 1);
                                direction = StackDirection::top_to_bottom();
                                children = [hex!(#c08800), hex!(#88c088), hex!(#ff8888), hex!(#4088c0)].into_iter().map(|c| {
                                    Wgt! {
                                        size = 50;
                                        background_color = c;
                                    }
                                });
                            },
                            Text! {
                                grid::cell::at = (1, 0);
                                txt = "GIF";
                            },
                            Image! {
                                grid::cell::at = (1, 1);
                                source = zng::env::res("color_profiles/anon.gif");
                            },
                            Text! {
                                grid::cell::at = (2, 0);
                                txt = "PNG";
                            },
                            Image! {
                                grid::cell::at = (2, 1);
                                source = zng::env::res("color_profiles/anon.png");
                            },
                            Text! {
                                grid::cell::at = (3, 0);
                                txt = "sRGB PNG";
                            },
                            Image! {
                                grid::cell::at = (3, 1);
                                source = zng::env::res("color_profiles/srgb.png");
                            },
                            Text! {
                                grid::cell::at = (4, 0);
                                txt = "Adobe98 PNG";
                            },
                            Image! {
                                grid::cell::at = (4, 1);
                                source = zng::env::res("color_profiles/adobe.png");
                            },
                            Text! {
                                grid::cell::at = (5, 0);
                                txt = "Custom ICC PNG";
                            },
                            Image! {
                                grid::cell::at = (5, 1);
                                source = zng::env::res("color_profiles/odd.png");
                            },
                            Text! {
                                grid::cell::at = (6, 0);
                                txt = "Gamma 1.0 PNG";
                            },
                            Image! {
                                grid::cell::at = (6, 1);
                                source = zng::env::res("color_profiles/gamma.png");
                            },
                            Text! {
                                grid::cell::at = (7, 0);
                                txt = "JPEG";
                            },
                            Image! {
                                grid::cell::at = (7, 1);
                                source = zng::env::res("color_profiles/anon.jpg");
                            },
                            Text! {
                                grid::cell::at = (8, 0);
                                txt = "sRGB JPEG";
                            },
                            Image! {
                                grid::cell::at = (8, 1);
                                source = zng::env::res("color_profiles/srgb.jpg");
                            },
                            Text! {
                                grid::cell::at = (9, 0);
                                txt = "Rec. 709 JPEG";
                            },
                            Image! {
                                grid::cell::at = (9, 1);
                                source = zng::env::res("color_profiles/rec709.jpg");
                            },
                        ];
                    };
                }
            });
        });
    }
}

async fn open_dialog() -> Option<PathBuf> {
    DIALOG
        .open_file("Open Image", std::env::current_dir().unwrap_or_default(), "", {
            let mut f = dialog::FileDialogFilters::default();
            f.push_filter("Image Files", &IMAGES.available_decoders());
            f.push_filter("All Files", &["*"]);
            f
        })
        .wait_rsp()
        .await
        .into_path()
        .unwrap_or_else(|e| {
            tracing::error!("open file dialog error, {e}");
            None
        })
}

fn center_viewport(msg: impl IntoUiNode) -> UiNode {
    Container! {
        // center the message on the scroll viewport:
        //
        // the large images can take a moment to decode in debug builds, but the size
        // is already known after read, so the "loading.." message ends-up off-screen
        // because it is centered on the image.
        layout::x = merge_var!(SCROLL.horizontal_offset(), SCROLL.zoom_scale(), |&h, &s| h.0.fct_l()
            - 1.vw() / s * h);
        layout::y = merge_var!(SCROLL.vertical_offset(), SCROLL.zoom_scale(), |&v, &s| v.0.fct_l() - 1.vh() / s * v);
        layout::scale = SCROLL.zoom_scale().map(|&fct| 1.fct() / fct);
        layout::transform_origin = 0;
        widget::auto_hide = false;
        layout::max_size = (1.vw(), 1.vh());

        child_align = Align::CENTER;
        child = msg;
    }
}

#[zng::prelude_wgt::widget($crate::ImgWindow {
    ($title:expr, $child:expr $(,)?) => {
        title = $title;
        child = $child;
    }
})]
pub struct ImgWindow(Window);
impl ImgWindow {
    fn widget_intrinsic(&mut self) {
        zng::prelude_wgt::widget_set! {
            self;
            wr::renderer_debug = {
                // wr::DebugFlags::TEXTURE_CACHE_DBG | wr::DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED
                wr::RendererDebug::disabled()
            };

            // render_mode = RenderMode::Software;

            child_align = Align::CENTER;

            state = WindowState::Maximized;
            size = (1140, 770); // restore size

            icon = zng::env::res("zng-logo.png");
            widget::background = Checkerboard!();

            color_scheme = color::ColorScheme::Dark;

            // content shown by all images when loading.
            img_loading_fn = wgt_fn!(|_| loading());

            // content shown by all images that failed to load.
            img_error_fn = wgt_fn!(|args: ImgErrorArgs| {
                center_viewport(Text! {
                    txt = args.error;
                    margin = 8;
                    align = Align::CENTER;
                    font_color = error_color();
                    drop_shadow = {
                        offset: (0, 0),
                        blur_radius: 4,
                        color: error_color().darken(5.pct()),
                    };
                })
            });

            // button color
            zng::button::style_fn = Style! {
                color::base_color = light_dark(rgb(0, 0, 255 - 40), rgb(0, 0, 40));
            };
        }
    }
}
fn loading_color() -> color::Rgba {
    web_colors::LIGHT_GRAY
}

fn error_color() -> color::Rgba {
    colors::RED
}

pub fn loading() -> UiNode {
    let mut dots_count = 3;
    let msg = timer::TIMERS.interval(300.ms(), false).map(move |_| {
        dots_count += 1;
        if dots_count == 8 {
            dots_count = 0;
        }
        formatx!("loading{:.^dots_count$}", "")
    });

    center_viewport(Text! {
        txt = msg;
        font_color = loading_color();
        margin = 8;
        layout::width = 80;
        font_style = FontStyle::Italic;
        drop_shadow = {
            offset: (0, 0),
            blur_radius: 4,
            color: loading_color().darken(5.pct()),
        };
    })
}

fn section(title: impl IntoVar<Txt>, children: impl IntoUiNode) -> UiNode {
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
        ];
    }
}

fn title(txt: impl IntoVar<Txt>) -> UiNode {
    Text! {
        txt;
        font_size = 20;
        background_color = colors::BLACK;
        padding = (5, 10);
    }
}

fn sub_title(txt: impl IntoVar<Txt>) -> UiNode {
    Text! {
        txt;

        font_size = 14;

        background_color = colors::BLACK;
        padding = (2, 5);
    }
}
