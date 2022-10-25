#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::app::EXIT_CMD;
use zero_ui::core::units::{DipPoint, DipSize};
use zero_ui::core::window::WindowVars;
use zero_ui::prelude::*;

// use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::init();

    // let rec = examples_util::record_profile("window");

    zero_ui_view::run_same_process(app_main);
    // app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(main_window);
}

fn main_window(ctx: &mut WindowContext) -> Window {
    let window_vars = WindowVars::req(&ctx.window_state);
    let window_id = *ctx.window_id;

    // Windows::req(ctx.services).exit_on_last_close = false;

    let title = merge_var!(
        window_vars.actual_position(),
        window_vars.actual_size(),
        move |p: &DipPoint, s: &DipSize| { formatx!("Window Example - position: {p:.0?}, size: {s:.0?}") }
    );

    let background = var(colors::BLACK);

    window! {
        background_color = background.easing(150.ms(), easing::linear);
        clear_color = rgba(0, 0, 0, 0);
        title;
        on_state_changed = hn!(|_, args: &WindowChangedArgs| {
            println!("state: {:?}", args.new_state().unwrap());
        });
        on_close_requested = confirm_close();
        child_align = Align::CENTER;
        child = h_stack! {
            spacing = 40;
            children = ui_list![
                v_stack! {
                    spacing = 20;
                    children = ui_list![
                        state_commands(window_id),
                        focus_control(),
                    ]
                },
                v_stack! {
                    spacing = 20;
                    children = ui_list![
                        state(window_vars),
                        visibility(window_vars),
                        chrome(window_vars),
                    ];
                },
                v_stack! {
                    spacing = 20;
                    children = ui_list![
                        icon(window_vars),
                        background_color(background),
                    ];
                },
                v_stack! {
                    spacing = 20;
                    children = ui_list![
                        screenshot(),
                        misc(window_id, window_vars),
                    ];
                },
            ];
        };
    }
}

fn background_color(color: impl Var<Rgba>) -> impl UiNode {
    fn color_btn(c: impl Var<Rgba>, select_on_init: bool) -> impl UiNode {
        toggle! {
            value::<Rgba> = c.clone();
            select_on_init;
            child = h_stack! {
                spacing = 4;
                children_align = Align::LEFT;
                children = ui_list![
                    blank! {
                        background_color = c.clone();
                        size = (16, 16);
                    },
                    text(c.map_to_text()),
                ];
            };
        }
    }
    fn primary_color(c: Rgba) -> impl UiNode {
        let c = c.desaturate(50.pct());
        let c = color_scheme_map(rgba(0, 0, 0, 20.pct()).mix_normal(c), rgba(255, 255, 255, 20.pct()).mix_normal(c));
        color_btn(c, false)
    }

    select(
        "Background Color",
        color,
        ui_list![
            color_btn(color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9)), true),
            primary_color(rgb(1.0, 0.0, 0.0)),
            primary_color(rgb(0.0, 0.8, 0.0)),
            primary_color(rgb(0.0, 0.0, 1.0)),
            primary_color(rgba(0, 0, 240, 20.pct())),
        ],
    )
}

fn screenshot() -> impl UiNode {
    fn of_window() -> impl UiNode {
        use std::time::Instant;
        let enabled = var(true);
        button! {
            child = text(enabled.map(|&enabled| {
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
                    Windows::req(ctx.services).frame_image(ctx.path.window_id()).get()
                });
                img.wait_done().await;
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

    fn of_headless_temp() -> impl UiNode {
        use zero_ui::core::window::{FrameCaptureMode, FrameImageReadyArgs};

        let enabled = var(true);
        button! {
            child = text(enabled.map(|&enabled| {
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
                Windows::req(ctx.services).open_headless(clone_move!(enabled, |_| window! {
                        size = (500, 400);
                        background_color = colors::DARK_GREEN;
                        font_size = 72;
                        child_align = Align::CENTER;
                        child = text("No Head!");

                        frame_capture_mode = FrameCaptureMode::Next;
                        on_frame_image_ready = async_hn_once!(|ctx, args: FrameImageReadyArgs| {
                            println!("saving screenshot..");
                            match args.frame_image.unwrap().save("screenshot.png").await {
                                Ok(_) => println!("saved"),
                                Err(e) => eprintln!("{e}")
                            }

                            let window_id = args.window_id;
                            ctx.with(|ctx| Windows::req(ctx.services).close(window_id).unwrap());

                            enabled.set(&ctx, true);
                        });
                    }),
                    true
                );
            });
        }
    }

    section("Screenshot", ui_list![of_window(), of_headless_temp(),])
}

fn icon(window_vars: &WindowVars) -> impl UiNode {
    let icon_btn = |label: &'static str, ico: WindowIcon| {
        toggle! {
            child = text(label);
            value = ico;
        }
    };
    select(
        "Icon",
        window_vars.icon().clone(),
        ui_list![
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
            icon_btn(
                "Render",
                WindowIcon::render(|_| text! {
                    size = (36, 36);
                    font_size = 28;
                    font_weight = FontWeight::BOLD;
                    text = "W";
                    drop_shadow = {
                        offset: (2, 2),
                        blur_radius: 5,
                        color: colors::BLACK,
                    };
                })
            )
        ],
    )
}

fn chrome(window_vars: &WindowVars) -> impl UiNode {
    let chrome_btn = |c: WindowChrome| {
        toggle! {
            child = text(formatx!("{c:?}"));
            value = c;
        }
    };
    select(
        "Chrome",
        window_vars.chrome().clone(),
        ui_list![
            chrome_btn(WindowChrome::Default),
            chrome_btn(WindowChrome::None),
            text("TODO custom"),
        ],
    )
}

fn state_commands(window_id: WindowId) -> impl UiNode {
    use zero_ui::widgets::window::commands::*;

    section(
        "Commands",
        ui_list![
            cmd_btn(MINIMIZE_CMD.scoped(window_id)),
            separator(),
            cmd_btn(RESTORE_CMD.scoped(window_id)),
            cmd_btn(MAXIMIZE_CMD.scoped(window_id)),
            separator(),
            cmd_btn(FULLSCREEN_CMD.scoped(window_id)),
            cmd_btn(EXCLUSIVE_FULLSCREEN_CMD.scoped(window_id)),
            separator(),
            cmd_btn(CLOSE_CMD.scoped(window_id)),
            cmd_btn(EXIT_CMD),
        ],
    )
}

fn focus_control() -> impl UiNode {
    let enabled = var(true);
    let focus_btn = button! {
        enabled = enabled.clone();
        child = text("Focus in 5s");
        on_click = async_hn!(enabled, |ctx, _| {
            enabled.set(&ctx, false);
            task::deadline(5.secs()).await;

            ctx.with(|ctx| {
                Windows::req(ctx.services).focus(ctx.path.window_id()).unwrap();
            });
            enabled.set(&ctx, true);
        });
    };

    let enabled = var(true);
    let critical_btn = button! {
        enabled = enabled.clone();
        child = text("Critical Alert in 5s");
        on_click = async_hn!(enabled, |ctx, _| {
            enabled.set(&ctx, false);
            task::deadline(5.secs()).await;

            ctx.with(|ctx| {
                WindowVars::req(ctx).focus_indicator().set(ctx.vars, Some(FocusIndicator::Critical));
            });
            enabled.set(&ctx, true);
        });
    };

    let enabled = var(true);
    let info_btn = button! {
        enabled = enabled.clone();
        child = text("Info Alert in 5s");
        on_click = async_hn!(enabled, |ctx, _| {
            enabled.set(&ctx, false);
            task::deadline(5.secs()).await;

            ctx.with(|ctx| {
                WindowVars::req(ctx).focus_indicator().set(ctx.vars, Some(FocusIndicator::Info));
            });
            enabled.set(&ctx, true);
        });
    };

    section("Focus", ui_list![focus_btn, critical_btn, info_btn,])
}

fn state(window_vars: &WindowVars) -> impl UiNode {
    let state_btn = |s: WindowState| {
        toggle! {
            child = text(formatx!("{s:?}"));
            value = s;
        }
    };

    select(
        "State",
        window_vars.state().clone(),
        ui_list![
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

fn visibility(window_vars: &WindowVars) -> impl UiNode {
    let visible = window_vars.visible();
    let btn = button! {
        enabled = visible.clone();
        child = text("Hide for 1s");
        on_click = async_hn!(visible, |ctx, _| {
            visible.set(&ctx, false);
            println!("visible=false");
            task::deadline(1.secs()).await;
            visible.set(&ctx, true);
            println!("visible=true");
        });
    };

    section("Visibility", ui_list![btn])
}

fn misc(window_id: WindowId, window_vars: &WindowVars) -> impl UiNode {
    let can_open_windows = window_vars.state().map(|&s| s != WindowState::Exclusive);
    section(
        "Misc.",
        ui_list![
            toggle! {
                child = text("Taskbar Visible");
                checked = window_vars.taskbar_visible().clone();
            },
            toggle! {
                child = text("Always on Top");
                checked = window_vars.always_on_top().clone();
            },
            separator(),
            cmd_btn(zero_ui::widgets::window::commands::INSPECT_CMD.scoped(window_id)),
            separator(),
            {
                let mut child_count = 0;
                button! {
                    child = text("Open Child Window");
                    enabled = can_open_windows.clone();
                    on_click = hn!(|ctx, _| {
                        child_count += 1;

                        let parent = ctx.path.window_id();
                        Windows::req(ctx.services).open(move |_| window! {
                            title = formatx!("Window Example - Child {child_count}");
                            size = (400, 300);
                            parent;
                            child_align = Align::CENTER;
                            start_position = StartPosition::CenterParent;
                            child = text! {
                                text = formatx!("Child {child_count}");
                                font_size = 20;
                            };
                        });
                    })
                }
            },
            {
                let mut other_count = 0;
                button! {
                    child = text("Open Other Window");
                    enabled = can_open_windows;
                    on_click = hn!(|ctx, _| {
                        other_count += 1;

                        Windows::req(ctx.services).open(move |_| window! {
                            title = formatx!("Window Example - Other {other_count}");
                            size = (400, 300);
                            child_align = Align::CENTER;
                            child = text! {
                                text = formatx!("Other {other_count}");
                                font_size = 20;
                            };
                        });
                    })
                }
            }
        ],
    )
}

#[derive(Debug, Clone, Copy)]
enum CloseState {
    Ask,
    Asking,
    Close,
}
fn confirm_close() -> impl WidgetHandler<WindowCloseRequestedArgs> {
    use zero_ui::widgets::window::*;

    let state = var(CloseState::Ask);
    hn!(|ctx, args: &WindowCloseRequestedArgs| {
        match state.get() {
            CloseState::Ask => {
                args.propagation().stop();
                state.set(ctx, CloseState::Asking);

                WindowLayers::insert(ctx, LayerIndex::TOP_MOST, close_dialog(args.windows.clone().into(), state.clone()))
            }
            CloseState::Asking => args.propagation().stop(),
            CloseState::Close => {}
        }
    })
}

fn close_dialog(windows: Vec<WindowId>, state: RcVar<CloseState>) -> impl UiNode {
    container! {
        id = "close-dialog";
        modal = true;
        background_color = color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        child_align = Align::CENTER;
        child = container! {
            background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 4;

            button::vis::extend_style = style_generator!(|_, _| {
                style! {
                    padding = 4;
                    corner_radius = unset!;
                }
            });

            child = v_stack! {
                children_align = Align::RIGHT;
                children = ui_list![
                    text! {
                        text = match windows.len() {
                            1 => "Close Confirmation\n\nClose 1 window?".to_text(),
                            n => formatx!("Close Confirmation\n\nClose {n} windows?")
                        };
                        margin = 15;
                    },
                    h_stack! {
                        spacing = 4;
                        children = ui_list![
                            button! {
                                child = strong("Close");
                                on_click = hn_once!(state, |ctx, _| {
                                    state.set(ctx, CloseState::Close);
                                    Windows::req(ctx.services).close_together(windows).unwrap();
                                })
                            },
                            button! {
                                child = text("Cancel");
                                on_click = hn!(state, |ctx, _| {
                                    state.set(ctx, CloseState::Ask);
                                    WindowLayers::remove(ctx, "close-dialog");
                                });
                            },
                        ]
                    }
                ]
            }
        }
    }
}

fn cmd_btn(cmd: Command) -> impl UiNode {
    button! {
        child = text(cmd.name_with_shortcut());
        enabled = cmd.is_enabled();
        visibility = cmd.has_handlers().map_into();
        on_click = hn!(|ctx, _| {
            cmd.notify(ctx);
        })
    }
}

fn section(header: &'static str, items: impl UiNodeList) -> impl UiNode {
    v_stack! {
        spacing = 5;
        children = ui_list![text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn select<T: VarValue + PartialEq>(header: &'static str, selection: impl Var<T>, items: impl UiNodeList) -> impl UiNode {
    v_stack! {
        spacing = 5;
        toggle::selection = toggle::Selector::single(selection);
        children = ui_list![text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn separator() -> impl UiNode {
    hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}
