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
    App::default().run_window(|_| {
        window! {
            title = "Button Example";
            child = stack! {
                direction = StackDirection::top_to_bottom();
                align = Align::CENTER;
                spacing = 5;
                sticky_width = true;
                children = ui_vec![
                    example(),
                    example(),
                    disabled(),
                    image_button(),
                    separator(),
                    toggle_buttons(),
                    dyn_buttons(),
                ];
            };
        }
    })
}

fn example() -> impl UiNode {
    let t = var_from("Click Me!");
    let mut count = 0;

    button! {
        on_click = hn!(t, |_, _| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(new_txt);
        });
        on_double_click = hn!(|_, _| println!("double click!"));
        on_triple_click = hn!(|_, _| println!("triple click!"));
        on_context_click = hn!(|_, _| println!("context click!"));
        child = text!(t);
    }
}

fn disabled() -> impl UiNode {
    button! {
        on_click = hn!(|_, _| panic!("disabled button"));
        enabled = false;
        child = text!("Disabled");
        id = "disabled-btn"
    }
}

fn image_button() -> impl UiNode {
    button! {
        id = "img-btn";
        on_click = hn!(|_, _| println!("Clicked image button"));
        child = stack! {
            direction = StackDirection::left_to_right();
            children_align = Align::CENTER;
            children = ui_vec![
                image! { source = "examples/res/window/icon-bytes.png"; size = (16, 16); },
                text!("Click Me!")
            ];
            spacing = 5;
        };
    }
}

fn dyn_buttons() -> impl UiNode {
    let dyn_children = EditableUiNodeList::from_vec(ui_vec![separator()]);
    let children_ref = dyn_children.reference();
    let mut btn = 'A';

    stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = dyn_children.chain(ui_vec![
            button! {
                child = text!("Add Button");
                on_click = hn!(|ctx, _| {
                    children_ref.push(button! {
                        child = text!("Remove {}", btn);
                        on_click = hn!(children_ref, |ctx, _| {
                            children_ref.remove(ctx.path.widget_id());
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
    hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}

fn toggle_buttons() -> impl UiNode {
    stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            toggle! {
                child = text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s.unwrap())));
                checked = var(false);
            },
            toggle! {
                child = text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s)));
                checked_opt = var(None);
            },
            toggle! {
                child = text!(toggle::IS_CHECKED_VAR.map(|s| formatx!("Toggle: {:?}", s)));
                checked_opt = var(Some(false));
                tristate = true;
            },
            toggle! {
                child = text!("Switch");
                checked = var(false);
                style_gen = style_gen!(|_, _| toggle::vis::switch_style!());
            },
            toggle! {
                child = text!("Checkbox");
                checked = var(false);
                style_gen = style_gen!(|_, _| toggle::vis::check_style!());
            },
            toggle! {
                child = text!("Checkbox Tristate");
                checked_opt = var(Some(false));
                tristate = true;
                style_gen = style_gen!(|_, _| toggle::vis::check_style!());
            },
            stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                toggle::selector = toggle::Selector::single(var("Paris"));
                children = ui_vec![
                    toggle! {
                        child = text!("Radio button (Tokyo)");
                        value::<&'static str> = "Tokyo";
                        style_gen = style_gen!(|_, _| toggle::vis::radio_style!());
                    },
                    toggle! {
                        child = text!("Radio button (Paris)");
                        value::<&'static str> = "Paris";
                        style_gen = style_gen!(|_, _| toggle::vis::radio_style!());
                    },
                    toggle! {
                        child = text!("Radio button (London)");
                        value::<&'static str> = "London";
                        style_gen = style_gen!(|_, _| toggle::vis::radio_style!());
                    },
                ];
            }
        ]
    }
}
