//! Demonstrates the button and toggle widgets.

use zng::{
    color::filter::opacity,
    gesture::ClickArgs,
    layout::{align, margin},
    mouse::ClickMode,
    prelude::*,
    stack,
    var::ObservableVec,
    widget::{LineStyle, node::EditableUiNodeList},
};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Button Example";
            lang = lang!("en-US");
            child = Stack! {
                direction = StackDirection::left_to_right();
                spacing = 20;
                align = Align::CENTER;
                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 5;
                        layout::sticky_width = true;
                        children = ui_vec![
                            example(),
                            example(),
                            disabled(),
                            separator(),
                            image_button(),
                            repeat_button(),
                            split_button(),
                            separator(),
                            light_button(),
                        ];
                    },
                    toggle_buttons(),
                    dyn_buttons(),
                    dyn_buttons_from_data(),
                ]
            };
        }
    })
}

fn example() -> impl UiNode {
    let t = var_from("Click Me!");
    let mut count = 0;

    Button! {
        on_click = hn!(t, |_| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 { "s" } else { "" });
            t.set(new_txt);
        });
        gesture::on_double_click = hn!(|_| tracing::info!("double click!"));
        gesture::on_triple_click = hn!(|_| tracing::info!("triple click!"));
        gesture::on_context_click = hn!(|_| tracing::info!("context click!"));
        context_menu = ContextMenu!(ui_vec![
            Button! {
                child = Text!("Context Item 1");
                on_click = hn!(|_| tracing::info!("context item 1 click!"));
            },
            Button! {
                child = Text!("Context Item 2");
                on_click = hn!(|_| tracing::info!("context item 2 click!"));
            }
        ]);
        child = Text!(t);
    }
}
fn disabled() -> impl UiNode {
    Button! {
        on_click = hn!(|_| panic!("disabled button"));
        widget::enabled = false;
        child = Text!("Disabled");
        id = "disabled-btn";
        tip::disabled_tooltip = Tip!(Text!("disabled tooltip"));
    }
}
fn image_button() -> impl UiNode {
    Button! {
        id = "img-btn";
        tooltip = Tip!(Text!("image button"));
        on_click = hn!(|_| tracing::info!("Clicked image button"));
        child_start = {
            node: Image! {
                source = include_bytes!("../../window/res/icon-bytes.png");
                layout::size = 16;
                align = Align::CENTER;
            },
            spacing: 5,
        };
        child = Text!("Image!");
    }
}
fn light_button() -> impl UiNode {
    Stack! {
        direction = StackDirection::left_to_right();
        spacing = 5;
        children = ui_vec![
            Button! {
                tooltip = Tip!(Text!("light button, ideal for icons"));
                style_fn = zng::button::LightStyle!();
                child = ICONS.req("material/outlined/insert-emoticon");
            },
            Toggle! {
                style_fn = zng::toggle::LightStyle!();
                child = ICONS.req("material/outlined/lightbulb");
                checked = var(true);
                when *#is_checked {
                    child = ICONS.req("material/filled/lightbulb");
                }
            },
        ]
    }
}
fn repeat_button() -> impl UiNode {
    let t = var(Txt::from_static("Repeat Click!"));

    Button! {
        id = "repeat-btn";
        mouse::click_mode = ClickMode::repeat();
        on_click = hn!(t, |args: &ClickArgs| {
            let new_txt = formatx!("repeat: {}, count: {}", args.is_repeat, args.click_count);
            t.set(new_txt);
        });

        child = Text!(t);
        tooltip = Tip!(Text!("Repeat Click!"));
    }
}

fn split_button() -> impl UiNode {
    let button_count = var(0u32);
    let split_count = var(0u32);

    Toggle! {
        style_fn = toggle::ComboStyle!();

        on_click = hn!(split_count, |_| {
            tracing::info!("Clicked split part");
            split_count.set(split_count.get() + 1);
        });

        child = Button! {
            on_click = hn!(button_count, |args: &ClickArgs| {
                tracing::info!("Clicked button part");
                button_count.set(button_count.get() + 1);

                args.propagation().stop();
            });

            child = Text!(merge_var!(button_count, split_count, |&b, &s| {
                if b == 0 && s == 0 {
                    formatx!("Split!")
                } else {
                    formatx!("button {b}, split {s}")
                }
            }));
        };
    }
}

fn toggle_buttons() -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Toggle! {
                child = Text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s.unwrap())));
                checked = var(false);
            },
            Toggle! {
                child = Text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s)));
                checked_opt = var(None);
            },
            Toggle! {
                child = Text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s)));
                checked_opt = var(Some(false));
                tristate = true;
            },
            separator(),
            combo_box(),
            separator(),
            Toggle! {
                child = Text!("Switch");
                checked = var(false);
                style_fn = toggle::SwitchStyle!();
            },
            Toggle! {
                child = Text!("Checkbox");
                checked = var(false);
                style_fn = toggle::CheckStyle!();
            },
            Toggle! {
                child = Text!("Checkbox Tristate");
                checked_opt = var(Some(false));
                tristate = true;
                style_fn = toggle::CheckStyle!();
            },
            separator(),
            Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                toggle::selector = toggle::Selector::single(var("Paris"));
                children = ui_vec![
                    Toggle! {
                        child = Text!("Radio button (Tokyo)");
                        value::<&'static str> = "Tokyo";
                        style_fn = toggle::RadioStyle!();
                    },
                    Toggle! {
                        child = Text!("Radio button (Paris)");
                        value::<&'static str> = "Paris";
                        style_fn = toggle::RadioStyle!();
                    },
                    Toggle! {
                        child = Text!("Radio button (London)");
                        value::<&'static str> = "London";
                        style_fn = toggle::RadioStyle!();
                    },
                ];
            }
        ]
    }
}

fn combo_box() -> impl UiNode {
    let txt = var(Txt::from_static("Combo"));
    let options = ["Combo", "Congo", "Pombo"];
    Toggle! {
        id = "combo";
        child = TextInput! {
            txt = txt.clone();
            gesture::on_click = hn!(|a: &ClickArgs| a.propagation().stop());
        };
        style_fn = toggle::ComboStyle!();

        checked_popup = wgt_fn!(|_| popup::Popup! {
            id = "popup";
            child = Stack! {
                toggle::selector = toggle::Selector::single(txt.clone());
                direction = StackDirection::top_to_bottom();
                children = options
                    .into_iter()
                    .map(|o| {
                        Toggle! {
                            child = Text!(o);
                            value::<Txt> = o;
                        }
                    })
                    .collect::<UiVec>();
            };
        })
    }
}

// dynamic add and remove buttons created directly
fn dyn_buttons() -> impl UiNode {
    let dyn_children = EditableUiNodeList::new();
    let children_ref = dyn_children.reference();
    let mut btn = 'A';

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = dyn_children.chain(ui_vec![
            separator_not_first(),
            Button! {
                child = Text!("Add Button");
                tooltip = Tip!(Text!("Add `Button!` directly"));
                on_click = hn!(|_| {
                    children_ref.push(dyn_button(
                        btn,
                        clmv!(children_ref, || {
                            children_ref.remove(WIDGET.id());
                        }),
                    ));

                    if btn == 'Z' {
                        btn = 'A'
                    } else {
                        btn = std::char::from_u32(btn as u32 + 1).unwrap();
                    }
                })
            }
        ])
    }
}

// dynamic add and remove buttons created from a data source.
fn dyn_buttons_from_data() -> impl UiNode {
    let data_source = var(ObservableVec::<char>::new());
    let mut btn = 'A';

    let view = widget::node::list_presenter(
        data_source.clone(),
        wgt_fn!(data_source, |data: char| {
            dyn_button(
                data,
                clmv!(data_source, || {
                    data_source.modify(move |a| {
                        if let Some(i) = a.iter().position(|&c| c == data) {
                            a.to_mut().remove(i);
                        }
                    });
                }),
            )
        }),
    );

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = view.chain(ui_vec![
            separator_not_first(),
            Button! {
                child = Text!("Add Button");
                tooltip = Tip!(Text!("Add data that generates `Button!`"));
                on_click = hn!(|_| {
                    data_source.modify(move |a| {
                        a.to_mut().push(btn);
                    });

                    if btn == 'Z' {
                        btn = 'A'
                    } else {
                        btn = std::char::from_u32(btn as u32 + 1).unwrap();
                    }
                })
            }
        ])
    }
}

// button that animates in and out.
fn dyn_button(content: char, remove: impl Fn() + Send + Sync + 'static) -> impl UiNode {
    let remove = std::sync::Arc::new(remove);
    let removing = var(false);

    Button! {
        child = Text!("Remove {content}");

        #[easing(100.ms())]
        opacity = 0.pct();
        #[easing(100.ms())]
        margin = (0, 0, -10, 0);

        when *#widget::is_inited {
            opacity = 100.pct();
            margin = 0;
        }

        when *#{removing.clone()} {
            widget::interactive = false;
            opacity = 0.pct();
            margin = (0, 0, -30, 0);
        }

        on_click = async_hn!(remove, removing, |_| {
            FOCUS.focus_next();
            removing.set(true);
            task::deadline(100.ms()).await;
            remove();
        });
    }
}

fn separator() -> impl UiNode {
    Hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}

fn separator_not_first() -> impl UiNode {
    Hr! {
        when #stack::is_first {
            widget::visibility = false;
        }

        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}
