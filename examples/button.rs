#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("button");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        Window! {
            title = "Button Example";
            child = Stack! {
                direction = StackDirection::left_to_right();
                spacing = 20;
                align = Align::CENTER;
                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 5;
                        sticky_width = true;
                        children = ui_vec![
                            example(),
                            example(),
                            disabled(),
                            image_button(),
                            repeat_button(),
                            separator(),
                            toggle_buttons(),
                            ];
                    },
                    dyn_buttons(),
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
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(new_txt);
        });
        on_double_click = hn!(|_| println!("double click!"));
        on_triple_click = hn!(|_| println!("triple click!"));
        on_context_click = hn!(|_| println!("context click!"));
        child = Text!(t);
    }
}

fn disabled() -> impl UiNode {
    Button! {
        on_click = hn!(|_| panic!("disabled button"));
        enabled = false;
        child = Text!("Disabled");
        id = "disabled-btn";
        disabled_tooltip = Tip!(Text!("disabled tooltip"));
    }
}

fn image_button() -> impl UiNode {
    Button! {
        id = "img-btn";
        tooltip = Tip!(Text!("image button"));
        on_click = hn!(|_| println!("Clicked image button"));
        child = Stack! {
            direction = StackDirection::left_to_right();
            children_align = Align::CENTER;
            children = ui_vec![
                Image! { source = "examples/res/window/icon-bytes.png"; size = (16, 16); },
                Text!("Click Me!")
            ];
            spacing = 5;
        };
    }
}

fn repeat_button() -> impl UiNode {
    Button! {
        id = "repeat-btn";
        click_mode = ClickMode::Repeat;
        on_click = hn!(|args: &ClickArgs| {
            println!("Clicked repeat button, is_repeat={}, click_count={}", args.is_repeat, args.click_count);
        });

        child = Text!("Repeat Click!");
    }
}

fn dyn_buttons() -> impl UiNode {
    let dyn_children = EditableUiNodeList::new();
    let children_ref = dyn_children.reference();
    let mut btn = 'A';

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = dyn_children.chain(ui_vec![
            Button! {
                child = Text!("Add Button");
                on_click = hn!(|_| {
                    children_ref.push(Button! {
                        child = Text!("Remove {}", btn);
                        on_click = hn!(children_ref, |_| {
                            children_ref.remove(WIDGET.id());
                        })
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

fn separator() -> impl UiNode {
    Hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
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
            Toggle! {
                child = TextInput! {
                    txt = var_from("Combo");
                    on_click = hn!(|a: &ClickArgs| a.propagation().stop());
                };
                style_fn = toggle::ComboStyle!();
            },
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
