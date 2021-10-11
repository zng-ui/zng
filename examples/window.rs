#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::{
    units::{DipPoint, DipSize},
    window::WindowStateChangedArgs,
};
use zero_ui::prelude::*;

fn main() {
    zero_ui_view::init();
    App::default().run_window(|ctx| {
        let vars = ctx.window_state.req(WindowVarsKey);
        //ctx.services.windows().shutdown_on_last_close = false;

        let title = merge_var!(vars.actual_position(), vars.actual_size(), |p: &DipPoint, s: &DipSize| {
            formatx!("Window Example - position: {:.0?}, size: {:.0?}", p, s)
        });
        let background_color = var(rgb(0.1, 0.1, 0.1));

        let position = var(Point::default());
        let size = var_from((900, 600));

        let icon = var(WindowIcon::Default);
        let chrome = var(WindowChrome::Default);
        let state = var(WindowState::Normal);

        window! {
            position = position.clone();
            size = size.clone();
            icon = icon.clone();
            chrome = chrome.clone();
            background_color = background_color.clone();
            state = state.clone();
            title;
            on_state_changed = hn!(|_, args: &WindowStateChangedArgs| {
                println!("state: {:?}", args.new_state);
            });
            content = h_stack! {
                spacing = 40;
                items = widgets![
                    v_stack! {
                        spacing = 20;
                        items = widgets![
                            property_stack("position", widgets![
                                set_position(0.0, 0.0, &position),
                                set_position(490.0, 290.0, &position),
                                set_position(500.0, 300.0, &position),
                            ]),
                            property_stack("miscellaneous", widgets![
                                screenshot(),
                                inspect(*ctx.window_id),
                                headless(),
                                always_on_top(),
                                taskbar_visible(),
                                close_window()
                            ]),
                        ];
                    },
                    v_stack! {
                        spacing = 20;
                        items = widgets![
                            property_stack("size", widgets![
                                set_size(1000.0, 900.0, &size),
                                set_size(500.0, 1000.0, &size),
                                set_size(800.0, 600.0, &size),
                            ]),
                            property_stack("state", widgets![
                                set_state(WindowState::Normal, &state),
                                set_state(WindowState::Minimized, &state),
                                set_state(WindowState::Maximized, &state),
                                set_state(WindowState::Fullscreen, &state),
                                set_state(WindowState::Exclusive, &state),
                            ]),
                        ]
                    },
                    property_stack("icon", widgets![
                        set_icon("Default", WindowIcon::Default, &icon),
                        set_icon("Png File", "examples/res/window/icon-file.png", &icon),
                        set_icon("Png Bytes", include_bytes!("res/window/icon-bytes.png"), &icon),
                        set_icon("Raw BGRA", {
                            let translucent_red = [0, 0, 255, 255 / 2];
                            let bgra = translucent_red.iter().copied().cycle().take(32 * 32 * 4).collect::<Vec<u8>>();
                            (bgra, zero_ui::core::image::ImageDataFormat::from(PxSize::new(Px(32), Px(32))))
                        }, &icon),
                        set_icon("Render", WindowIcon::render(|_| {
                            container! {
                                content = text("W");
                                background_color = colors::DARK_BLUE;
                            }
                        }), &icon)
                    ]),
                    property_stack("chrome", widgets![
                        set_chrome("Default", true, &chrome),
                        set_chrome("None", false, &chrome),

                    ]),
                    property_stack("background_color", widgets![
                        set_background(rgb(0.1, 0.1, 0.1), "default", &background_color),
                        set_background(rgb(0.3, 0.0, 0.0), "red", &background_color),
                        set_background(rgb(0.0, 0.3, 0.0), "green", &background_color),
                        set_background(rgb(0.0, 0.0, 0.3), "blue", &background_color),
                    ])
                ];
            };
        }
    });

    #[cfg(feature = "app_profiler")]
    zero_ui::core::profiler::write_profile("window-profile.json", false);
}

fn property_stack(header: &'static str, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items = widgets![text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn set_position(x: f32, y: f32, window_position: &RcVar<Point>) -> impl Widget {
    set_var_btn(window_position, (x, y).into(), formatx!("move to {}x{}", x, y))
}

fn set_size(width: f32, height: f32, window_size: &RcVar<Size>) -> impl Widget {
    set_var_btn(window_size, (width, height).into(), formatx!("resize to {}x{}", width, height))
}

fn set_state(state: WindowState, state_var: &RcVar<WindowState>) -> impl Widget {
    set_var_btn(state_var, state, formatx!("{:?}", state))
}

fn set_background(color: Rgba, color_name: &str, background_color: &RcVar<Rgba>) -> impl Widget {
    set_var_btn(background_color, color, formatx!("{} background", color_name))
}

fn set_var_btn<T: zero_ui::core::var::VarValue>(var: &RcVar<T>, new_value: T, content_txt: Text) -> impl Widget {
    let var = var.clone();
    button! {
        content = text(content_txt);
        on_click = hn!(|ctx, _| {
            var.set(ctx,  new_value.clone());
        });
    }
}

fn screenshot() -> impl Widget {
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
            println!("taken in {:?}, saving..", t.elapsed());

            let t = Instant::now();

            match img.save("screenshot.png").await {
                Ok(_) => {
                    println!("saved in {:?}", t.elapsed());
                }
                Err(e) => {
                    eprintln!("error {}", e)
                }
            }


            enabled.set(&ctx, true);
        });
        enabled;
    }
}

fn inspect(scope: WindowId) -> impl Widget {
    let cmd = zero_ui::widgets::window::commands::InspectCommand.scoped(scope);
    button! {
        content = text(cmd.name());
        enabled = cmd.enabled();
        visibility = cmd.has_handlers().map_into();
        on_click = hn!(|ctx, _| {
            cmd.notify(ctx, None);
        })
    }
}

fn headless() -> impl Widget {
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
                        enabled.set(&ctx, false);

                        println!("saving screenshot..");
                        match args.frame_image.unwrap().save("screenshot.png").await {
                            Ok(_) => println!("saved"),
                            Err(e) => eprintln!("{}", e)
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

fn always_on_top() -> impl Widget {
    button! {
        content = text("always_on_top");
        on_click = hn!(|ctx, _| {
            ctx.services.windows().open(|_| {
                let always_on_top = var(true);
                window! {
                    title = always_on_top.map(|b| formatx!{"always_on_top = {}", b});
                    content = button!{
                        content = text("toggle always_on_top");
                        on_click = hn!(always_on_top, |ctx, _| {
                            always_on_top.modify(ctx, |b| **b = !**b)
                        })
                    };
                    size = (400, 300);
                    always_on_top;
                }
            });
        })
    }
}

fn taskbar_visible() -> impl Widget {
    button! {
        content = text("taskbar_visible");
        on_click = hn!(|ctx, _| {
            ctx.services.windows().open(|_| {
                let taskbar_visible = var(false);
                window! {
                    title = taskbar_visible.map(|b| formatx!{"taskbar_visible = {}", b});
                    content = button!{
                        content = text("toggle taskbar_visible");
                        on_click = hn!(taskbar_visible, |ctx, _| {
                            taskbar_visible.modify(ctx, |b| **b = !**b)
                        })
                    };
                    size = (400, 300);
                    taskbar_visible;
                }
            });
        })
    }
}

fn close_window() -> impl Widget {
    use zero_ui::widgets::window::commands::CloseCommand;
    button! {
        content = text(CloseCommand.name());
        on_click = hn!(|ctx, _| {
            CloseCommand.notify(ctx, None);
        })
    }
}

fn set_chrome(label: impl IntoVar<Text> + 'static, chrome: impl Into<WindowChrome>, var: &RcVar<WindowChrome>) -> impl Widget {
    let var = var.clone();
    let chrome = chrome.into();
    button! {
        content = text(label);
        on_click = hn!(|ctx, _| {
            var.set_ne(ctx, chrome.clone());
        });
    }
}

fn set_icon(label: impl IntoVar<Text> + 'static, icon: impl Into<WindowIcon>, var: &RcVar<WindowIcon>) -> impl Widget {
    let var = var.clone();
    let icon = icon.into();
    button! {
        content = text(label);
        on_click = hn!(|ctx, _| {
            var.set_ne(ctx, icon.clone());
        });
    }
}
