#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

// use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Build Time";
            child = v_stack! {
                spacing = 5;
                children = ui_list![
                    example(),
                    example(),
                    disabled(),
                    image_button(),
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
        on_click = hn!(t, |ctx, _| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(ctx, new_txt);
        });
        on_double_click = hn!(|_, _| println!("double click!"));
        on_triple_click = hn!(|_, _| println!("triple click!"));
        on_context_click = hn!(|_, _| println!("context click!"));
        child = text(t);
    }
}

fn disabled() -> impl UiNode {
    button! {
        on_click = hn!(|_, _| panic!("disabled button"));
        enabled = false;
        child = text("Disabled");
        id = "disabled-btn"
    }
}

fn image_button() -> impl UiNode {
    button! {
        id = "img-btn";
        on_click = hn!(|_, _| println!("Clicked image button"));
        child = h_stack! {
            children = ui_list![
                image! { source = "examples/res/window/icon-bytes.png"; size = (16, 16); },
                text("Click Me!")
            ];
            spacing = 5;
        };
    }
}

fn dyn_buttons() -> impl UiNode {
    let dyn_items = widget_vec![separator()];
    let items_ref = dyn_items.reference();
    let mut btn = 'A';

    v_stack! {
        spacing = 5;
        items = dyn_items.chain(ui_list![
            button! {
                child = text("Add Button");
                on_click = hn!(|ctx, _| {
                    items_ref.push(ctx, button! {
                        child = text(formatx!("Remove {}", btn));
                        on_click = hn!(items_ref, |ctx, _| {
                            items_ref.remove(ctx.updates, ctx.path.widget_id());
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
        style_gen = LineStyle::Dashed;
    }
}
