#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::focus::{FocusRequest, FocusTarget, FOCUS_CHANGED_EVENT};
use zero_ui::prelude::*;
use zero_ui::widgets::window::{LayerIndex, WindowLayers};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("focus");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        ctx.window_id.set_name("main").unwrap();

        trace_focus();
        let window_enabled = var(true);
        window! {
            title = "Focus Example";
            enabled = window_enabled.clone();
            background = commands();
            child = v_stack! {
                children = ui_list![
                    alt_scope(),
                    h_stack! {
                        margin = (50, 0, 0, 0);
                        align = Align::CENTER;
                        spacing = 10;
                        children = ui_list![
                            tab_index(),
                            functions(window_enabled),
                            delayed_focus(),
                        ]
                    }
                ];
            };
            // zero_ui::properties::inspector::show_center_points = true;
            // zero_ui::properties::inspector::show_bounds = true;
            // zero_ui::properties::inspector::show_hit_test = true;
            // zero_ui::properties::inspector::show_directional_query = Some(zero_ui::core::units::Orientation2D::Below);
        }
    })
}

fn alt_scope() -> impl UiNode {
    h_stack! {
        alt_focus_scope = true;
        button::vis::extend_style = style_generator!(|_, _| style! {
            border = unset!;
            corner_radius = unset!;
        });
        children = ui_list![
            button("alt", TabIndex::AUTO),
            button("scope", TabIndex::AUTO),
        ];
    }
}

fn tab_index() -> impl UiNode {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(T);
        children = ui_list![
            title("TabIndex (T)"),
            button("Button A5", 5),
            button("Button A4", 3),
            button("Button A3", 2),
            button("Button A1", 0),
            button("Button A2", 0),
        ];
    }
}

fn functions(window_enabled: RcVar<bool>) -> impl UiNode {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(F);
        children = ui_list![
            title("Functions (F)"),
            // New Window
            button! {
                child = text("New Window");
                on_click = hn!(|ctx, _| {
                    Windows::req(ctx.services).open(|ctx| {
                        let _ = ctx.window_id.set_name("other");
                        window! {
                            title = "Other Window";
                            focus_shortcut = shortcut!(W);
                            child = v_stack! {
                                align = Align::CENTER;
                                spacing = 5;
                                children = ui_list![
                                    title("Other Window (W)"),
                                    button("Button B5", 5),
                                    button("Button B4", 3),
                                    button("Button B3", 2),
                                    button("Button B1", 0),
                                    button("Button B2", 0),
                                ]
                            };
                        }
                    });
                });
            },
            // Detach Button
            {
                let detach_focused = RcNode::new_cyclic(|wk| {
                    let btn = button! {
                        child = text("Detach Button");
                        // focus_on_init = true;
                        on_click = hn!(|ctx, _| {
                            let wwk = wk.clone();
                            Windows::req(ctx.services).open(move |_| {
                                window! {
                                    title = "Detached Button";
                                    child_align = Align::CENTER;
                                    child = wwk.upgrade().unwrap().take_when(true);
                                }
                            });
                        });
                    };
                    btn.boxed()
                });
                detach_focused.take_when(true).into_widget()
            },
            // Disable Window
            disable_window(window_enabled.clone()),
            // Overlay Scope
            button! {
                child = text("Overlay Scope");
                on_click = hn!(|ctx, _| {
                    WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay(window_enabled.clone()));
                });
            },
            nested_focusables(),
        ]
    }
}
fn disable_window(window_enabled: RcVar<bool>) -> impl UiNode {
    button! {
        child = text(window_enabled.map(|&e| if e { "Disable Window" } else { "Enabling in 1s..." }.into()));
        min_width = 140;
        on_click = async_hn!(window_enabled, |ctx, _| {
            window_enabled.set(&ctx, false);
            task::deadline(1.secs()).await;
            window_enabled.set(&ctx, true);
        });
    }
}
fn overlay(window_enabled: RcVar<bool>) -> impl UiNode {
    container! {
        id = "overlay";
        modal = true;
        child_align = Align::CENTER;
        child = container! {
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 2;
            child = v_stack! {
                children_align = Align::RIGHT;
                children = ui_list![
                    text! {
                        txt = "Window scope is disabled so the overlay scope is the root scope.";
                        margin = 15;
                    },
                    h_stack! {
                        spacing = 2;
                        children = ui_list![
                        disable_window(window_enabled),
                        button! {
                                child = text("Close");
                                on_click = hn!(|ctx, _| {
                                    WindowLayers::remove(ctx, "overlay");
                                })
                            }
                        ]
                    }
                ]
            }
        }
    }
}

fn delayed_focus() -> impl UiNode {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(D);
        children = ui_list![
            title("Delayed 4s (D)"),

            delayed_btn("Force Focus", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: true,
                    window_indicator: None,
                });
            }),
            delayed_btn("Info Indicator", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Info),
                });
            }),
            delayed_btn("Critical Indicator", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Critical),
                });
            }),

            text! {
                id = "target";
                padding = (7, 15);
                corner_radius = 4;
                txt = "delayed target";
                font_style = FontStyle::Italic;
                txt_align = TextAlign::CENTER_MIDDLE;
                background_color = color_scheme_map(rgb(30, 30, 30), rgb(225, 225, 225));

                focusable = true;
                when *#is_focused {
                    txt = "focused";
                    background_color = color_scheme_map(colors::DARK_GREEN, colors::LIGHT_GREEN);
                }
            },
        ]
    }
}
fn delayed_btn(content: impl Into<Text>, on_timeout: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
    let on_timeout = std::rc::Rc::new(std::cell::RefCell::new(Box::new(on_timeout)));
    let enabled = var(true);
    button! {
        child = text(content.into());
        on_click = async_hn!(enabled, on_timeout, |ctx, _| {
            enabled.set(&ctx, false);
            task::deadline(4.secs()).await;
            ctx.with(|ctx| {
                let mut on_timeout = on_timeout.borrow_mut();
                on_timeout(ctx);
            });
            enabled.set(&ctx, true);
        });
        enabled;
    }
}

fn title(txt: impl IntoVar<Text>) -> impl UiNode {
    text! { txt; font_weight = FontWeight::BOLD; align = Align::CENTER; }
}

fn button(content: impl Into<Text>, tab_index: impl Into<TabIndex>) -> impl UiNode {
    let content = content.into();
    let tab_index = tab_index.into();
    button! {
        child = text(content.clone());
        tab_index;
        on_click = hn!(|_, _| {
            println!("Clicked {content} {tab_index:?}")
        });
    }
}

fn commands() -> impl UiNode {
    use zero_ui::core::focus::commands::*;

    let cmds = [
        FOCUS_NEXT_CMD,
        FOCUS_PREV_CMD,
        FOCUS_UP_CMD,
        FOCUS_RIGHT_CMD,
        FOCUS_DOWN_CMD,
        FOCUS_LEFT_CMD,
        FOCUS_ALT_CMD,
        FOCUS_ENTER_CMD,
        FOCUS_EXIT_CMD,
    ];

    v_stack! {
        align = Align::BOTTOM_RIGHT;
        padding = 10;
        spacing = 8;
        children_align = Align::RIGHT;

        font_family = FontName::monospace();
        txt_color = colors::GRAY;

        children = cmds.into_iter().map(|cmd| {
            text! {
                txt = cmd.name_with_shortcut();

                when *#{cmd.is_enabled()} {
                    txt_color = color_scheme_map(colors::WHITE, colors::BLACK);
                }
            }.boxed()
        }).collect::<Vec<_>>();
    }
}

fn trace_focus() {
    FOCUS_CHANGED_EVENT
        .on_pre_event(app_hn!(|ctx, args: &FocusChangedArgs, _| {
            if args.is_hightlight_changed() {
                println!("highlight: {}", args.highlight);
            } else if args.is_widget_move() {
                println!("focused {:?} moved", args.new_focus.as_ref().unwrap());
            } else if args.is_enabled_change() {
                println!("focused {:?} enabled/disabled", args.new_focus.as_ref().unwrap());
            } else {
                println!(
                    "{} -> {}",
                    inspect::focus(ctx.services, &args.prev_focus),
                    inspect::focus(ctx.services, &args.new_focus)
                );
            }
        }))
        .perm();
}

fn nested_focusables() -> impl UiNode {
    button! {
        child = text("Nested Focusables");

        on_click = hn!(|ctx, args: &ClickArgs| {
            args.propagation().stop();
            Windows::req(ctx.services).focus_or_open("nested-focusables", |_| {
                window! {
                    title = "Focus Example - Nested Focusables";

                    color_scheme = ColorScheme::Dark;
                    background_color = colors::DIM_GRAY;

                    // zero_ui::properties::inspector::show_center_points = true;
                    child_align = Align::CENTER;
                    child = v_stack! {
                        spacing = 10;
                        children = ui_list![
                            nested_focusables_group('a'),
                            nested_focusables_group('b'),
                        ];
                    }
                }
            });
        })
    }
}
fn nested_focusables_group(g: char) -> impl UiNode {
    h_stack! {
        align = Align::TOP;
        spacing = 10;
        children = (0..5).map(|n| nested_focusable(g, n, 0).boxed()).collect::<Vec<_>>()
    }
}
fn nested_focusable(g: char, column: u8, row: u8) -> impl UiNode {
    let nested = text! {
        txt = format!("Focusable {column}, {row}");
        margin = 5;
    };
    v_stack! {
        id = formatx!("focusable-{g}-{column}-{row}");
        padding = 2;

        children = if row == 5 {
            ui_list![nested]
        } else {
            ui_list![
                nested,
                nested_focusable(g, column, row + 1),
            ]
        };

        focusable = true;

        corner_radius = 5;
        border = 1, colors::RED.with_alpha(30.pct());
        background_color = colors::RED.with_alpha(20.pct());
        when *#is_focused {
            background_color = colors::GREEN;
        }
        when *#is_return_focus {
            border = 1, colors::LIME_GREEN;
        }
    }
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;
    use zero_ui::core::focus::WidgetInfoFocusExt;
    use zero_ui::core::inspector::WidgetInfoInspectorExt;

    pub fn focus(services: &mut Services, path: &Option<InteractionPath>) -> String {
        path.as_ref()
            .map(|p| {
                let frame = if let Ok(w) = Windows::req(services).widget_tree(p.window_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let widget = if let Some(w) = frame.get(p.widget_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let wgt_mod = if let Some(b) = widget.inspector_info() {
                    b.builder.widget_mod()
                } else {
                    return format!("<{p}>");
                };
                if wgt_mod.path.ends_with("button") {
                    let txt = widget
                        .inspect_descendant("text")
                        .expect("expected text in button")
                        .inspect_property("txt")
                        .expect("expected txt property in text")
                        .live_debug(0)
                        .get();

                    format!("button({txt})")
                } else if wgt_mod.path.ends_with("window") {
                    let title = widget.inspect_property("title").map(|p| p.live_debug(0).get()).unwrap_or_default();

                    format!("window({title})")
                } else {
                    let focus_info = widget.as_focus_info(true, true);
                    if focus_info.is_alt_scope() {
                        format!("{}(is_alt_scope)", wgt_mod.name())
                    } else if focus_info.is_scope() {
                        format!("{}(is_scope)", wgt_mod.name())
                    } else {
                        format!("{}({})", wgt_mod.name(), p.widget_id())
                    }
                }
            })
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

#[cfg(not(debug_assertions))]
mod inspect {
    use super::*;

    pub fn focus(path: &Option<InteractionPath>, _: &mut Services) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}
