#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::units::{DipPoint, DipSize};
use zero_ui::core::window::WindowVars;
use zero_ui::prelude::*;

// use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-window.json.gz", &[("example", &"window")], |_| true);

    zero_ui_view::run_same_process(app_main);
    // app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(main_window);
}

fn main_window(ctx: &mut WindowContext) -> Window {
    let window_vars = ctx.window_state.req(WindowVarsKey);
    let window_id = *ctx.window_id;

    //ctx.services.windows().shutdown_on_last_close = false;

    let title = merge_var!(
        window_vars.actual_position(),
        window_vars.actual_size(),
        move |p: &DipPoint, s: &DipSize| { formatx!("Window Example {} - position: {p:.0?}, size: {s:.0?}", window_id.sequential()) }
    );

    let background = var(rgb(0.1, 0.1, 0.1));

    window! {
        background_color = background.clone();
        clear_color = rgba(0, 0, 0, 0);
        title;
        on_state_changed = hn!(|_, args: &WindowChangedArgs| {
            println!("state: {:?}", args.new_state().unwrap());
        });
        content = h_stack! {
            spacing = 40;
            items = widgets![
                state_commands(window_id),
                v_stack! {
                    spacing = 20;
                    items = widgets![
                        state(window_vars),
                        visibility(window_vars),
                        chrome(window_vars),
                    ];
                },
                v_stack! {
                    spacing = 20;
                    items = widgets![
                        icon(window_vars),
                        background_color(background),
                    ];
                },
                v_stack! {
                    spacing = 20;
                    items = widgets![
                        screenshot(),
                        misc(window_id, window_vars),
                    ];
                },
            ];
        };
    }
}

fn background_color(color: impl Var<Rgba>) -> impl Widget {
    let color_btn = |c: Rgba| {
        let color = color.clone();
        button! {
            content = h_stack! {
                spacing = 4;
                items = widgets![
                    blank! {
                        background_color = c;
                        size = (16, 16);
                    },
                    text(c.to_text()),
                ];
            };
            on_click = hn!(color, |ctx, _| {
                color.set_ne(ctx, c).unwrap();
            });
            when *#{color} == c {
                background_color = selected_bkg();
            }
        }
    };

    section(
        "Background Color",
        widgets![
            color_btn(rgb(0.1, 0.1, 0.1)),
            color_btn(rgb(0.3, 0.0, 0.0)),
            color_btn(rgb(0.0, 0.3, 0.0)),
            color_btn(rgb(0.0, 0.0, 0.3)),
            color_btn(rgba(0, 0, 240, 20.pct())),
        ],
    )
}

fn screenshot() -> impl Widget {
    fn of_window() -> impl Widget {
        use std::time::Instant;
        let enabled = var(true);
        button! {
            content = text(enabled.map(|&enabled| {
                if enabled {
                    "screenshot".to_text()
                } else {
                    "saving..".to_text()
                }
            }));
            on_click = async_hn!(enabled, |ctx, _| {
                // disable button until screenshot is saved.
                enabled.set(&ctx, false);

                println!("taking `screenshot.png`..");

                let t = Instant::now();
                let img = ctx.with(|ctx|{
                    ctx.services.windows().frame_image(ctx.path.window_id()).get_clone(ctx.vars)
                });
                img.awaiter().await;
                println!("taken in {:?}, saving..", t.elapsed());

                let t = Instant::now();

                match img.save("screenshot.png").await {
                    Ok(_) => {
                        println!("saved in {:?}", t.elapsed());
                    }
                    Err(e) => {
                        eprintln!("error {e}")
                    }
                }


                enabled.set(&ctx, true);
            });
            enabled;
        }
    }

    fn of_headless_temp() -> impl Widget {
        use zero_ui::core::window::{FrameCaptureMode, FrameImageReadyArgs};

        let enabled = var(true);
        button! {
            content = text(enabled.map(|&enabled| {
                if enabled {
                    "headless".to_text()
                } else {
                    "saving..".to_text()
                }
            }));
            enabled = enabled.clone();
            on_click = hn!(|ctx, _| {
                enabled.set(ctx.vars, false);

                println!("taking `screenshot.png` using a new headless window ..");
                ctx.services.windows().open_headless(clone_move!(enabled, |_| window! {
                        size = (500, 400);
                        background_color = colors::DARK_GREEN;
                        font_size = 72;
                        content = text("No Head!");

                        frame_capture_mode = FrameCaptureMode::Next;
                        on_frame_image_ready = async_hn_once!(|ctx, args: FrameImageReadyArgs| {
                            println!("saving screenshot..");
                            match args.frame_image.unwrap().save("screenshot.png").await {
                                Ok(_) => println!("saved"),
                                Err(e) => eprintln!("{e}")
                            }

                            let window_id = args.window_id;
                            ctx.with(|ctx| ctx.services.windows().close(window_id).unwrap());

                            enabled.set(&ctx, true);
                        });
                    }),
                    true
                );
            });
        }
    }

    section("Screenshot", widgets![of_window(), of_headless_temp(),])
}

fn icon(window_vars: &WindowVars) -> impl Widget {
    let icon_btn = |label: &'static str, ico: WindowIcon| {
        let icon = window_vars.icon().clone();

        button! {
            content = text(label);
            on_click = hn!(icon, ico, |ctx, _| {
                icon.set_ne(ctx, ico.clone());
            });

            when *#{icon} == ico {
                background_color = selected_bkg();
            }
        }
    };

    section(
        "Icon",
        widgets![
            icon_btn("Default", WindowIcon::Default),
            icon_btn("Png File", "examples/res/window/icon-file.png".into()),
            icon_btn("Png Bytes", include_bytes!("res/window/icon-bytes.png").into()),
            icon_btn("Raw BGRA", {
                use zero_ui::core::image::*;

                let color = [0, 0, 255, 255 / 2];

                let size = PxSize::new(Px(32), Px(32));
                let len = size.width.0 * size.height.0 * 4;
                let bgra: Vec<u8> = color.iter().copied().cycle().take(len as usize).collect();

                (bgra, ImageDataFormat::from(size)).into()
            }),
            icon_btn("Render", WindowIcon::render(|_| text! {
                size = (36, 36);
                font_size = 28;
                font_weight = FontWeight::BOLD;
                text = "W";
                drop_shadow = {
                    offset: (2, 2),
                    blur_radius: 5,
                    color: colors::BLACK,
                };
            }))
        ],
    )
}

fn chrome(window_vars: &WindowVars) -> impl Widget {
    let chrome_btn = |c: WindowChrome| {
        let chrome = window_vars.chrome().clone();

        button! {
            content = text(formatx!("{c:?}"));
            on_click = hn!(chrome, c, |ctx, _| {
                chrome.set_ne(ctx, c.clone());
            });

            when *#{chrome} == c {
                background_color = selected_bkg();
            }
        }
    };

    section(
        "Chrome",
        widgets![
            chrome_btn(WindowChrome::Default),
            chrome_btn(WindowChrome::None),
            text("TODO custom"),
        ],
    )
}

fn state_commands(window_id: WindowId) -> impl Widget {
    use zero_ui::widgets::window::commands::*;

    section(
        "Commands",
        widgets![
            cmd_btn(MinimizeCommand.scoped(window_id)),
            separator(),
            cmd_btn(RestoreCommand.scoped(window_id)),
            cmd_btn(MaximizeCommand.scoped(window_id)),
            separator(),
            cmd_btn(FullscreenCommand.scoped(window_id)),
            cmd_btn(ExclusiveFullscreenCommand.scoped(window_id)),
            separator(),
            cmd_btn(CloseCommand.scoped(window_id)),
        ],
    )
}

fn state(window_vars: &WindowVars) -> impl Widget {
    let state_btn = |s: WindowState| {
        let state = window_vars.state().clone();
        button! {
            content = text(formatx!("{s:?}"));
            on_click = hn!(state, |ctx, _| {
                state.set_ne(ctx, s);
            });

            when *#{state} == s {
                background_color = selected_bkg();
            }
        }
    };

    section(
        "State",
        widgets![
            state_btn(WindowState::Minimized),
            separator(),
            state_btn(WindowState::Normal),
            state_btn(WindowState::Maximized),
            separator(),
            state_btn(WindowState::Fullscreen),
            state_btn(WindowState::Exclusive),
            text("TODO video mode"),
        ],
    )
}

fn visibility(window_vars: &WindowVars) -> impl Widget {
    let visible = window_vars.visible();
    let btn = button! {
        enabled = visible.clone();
        content = text("Hide for 1s");
        on_click = async_hn!(visible, |ctx, _| {
            visible.set(&ctx, false);
            println!("visible=false");
            task::timeout(1.secs()).await;
            visible.set(&ctx, true);
            println!("visible=true");
        });
    };

    section("Visibility", widgets![btn])
}

fn misc(window_id: WindowId, window_vars: &WindowVars) -> impl Widget {
    fn flag_btn(label: &'static str, flag: RcVar<bool>) -> impl Widget {
        let c_false = rgb(60, 40, 40);
        let c_false_border = rgb(80, 40, 40);
        let c_true = rgb(40, 60, 40);
        let c_true_border = rgb(40, 80, 40);

        button! {
            content = text(label);
            on_click = hn!(flag, |ctx, _| {
                flag.modify(ctx, |f| **f = !**f);
            });

            background_color = c_false;
            when *#{flag.clone()} {
                background_color = c_true;
            }

            when self.is_hovered && !*#{flag.clone()} {
                background_color = c_false;

                // TODO allow setting only border.sides
                border = {
                    sides: c_true_border,
                    widths: button::theme::BorderWidthsVar,
                };
            }

            when self.is_hovered && *#{flag} {
                background_color = c_true;

                border = {
                    sides: c_false_border,
                    widths: button::theme::BorderWidthsVar,
                };
            }
        }
    }

    section(
        "Misc.",
        widgets![
            flag_btn("Taskbar Visible", window_vars.taskbar_visible().clone()),
            flag_btn("Always on Top", window_vars.always_on_top().clone()),
            separator(),
            cmd_btn(zero_ui::widgets::window::commands::InspectCommand.scoped(window_id)),
            separator(),
            button! {
                content = text("Open Another Window");
                on_click = hn!(|ctx, _| {
                    ctx.services.windows().open(main_window);
                })
            }
        ],
    )
}

fn cmd_btn(cmd: impl Command) -> impl Widget {
    button! {
        content = text(cmd.name_with_shortcut());
        enabled = cmd.enabled();
        visibility = cmd.has_handlers().map_into();
        on_click = hn!(|ctx, _| {
            cmd.notify_cmd(ctx, None);
        })
    }
}

fn section(header: &'static str, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items = widgets![text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn separator() -> impl Widget {
    line_w! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        style = LineStyle::Dashed;
    }
}

fn selected_bkg() -> Rgba {
    rgb(40, 40, 60)
}
