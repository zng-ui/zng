#![recursion_limit = "256"]

use enclose::enclose;

use zero_ui::prelude::*;

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        let size = var((800., 600.));
        let title = size.map(|s: &LayoutSize| formatx!("Button Example - {}x{}", s.width.ceil(), s.height.ceil()));
        window! {
            size: size;
            title: title;
            => {
                v_stack((example(), example()).into())
            }
        }
    })
}

fn example() -> impl UiNode {
    let t = var("Click Me!");

    button! {
        on_click: enclose!{ (t) move |a| {
            let ctx = a.ctx();
            ctx.updates.push_set(&t, "Clicked!".into(), ctx.vars).unwrap();
        }};
        align: Alignment::CENTER;
        font_size: 20;

        when self.is_hovered {
            font_size: 30;
        }

        => {
            text(t)
        }
    }
}

#[allow(unused)]
macro_rules! TODO {
    (new) => {
        button! {
            on_click: |_|println!("Button clicked!");
            // remove `=>`?
            content: {
                container! {
                    content: "Click Me!"
                }
            };

            margin: 10.0;
            size: (300.0, 200.0);
            align: Alignment::CENTER;
            font_size: 28;

            // when only at the end?
            when self.is_hovered {
                font_size: 30;
            }
        }
    };

    (widget) => {
        widget! {
            // using properties?
            content -> child!;
            // OR
            content: child!;//like required! but is the child
            // OR
            content -> child!: required!;//optional child, if not required we use a placeholder?.
            // where to place it? default and default_child does not fit?

            // using fns?
            // OR
            fn child(content!) -> impl UiNode {
                child// content is an UiNode and is a required "property" in the widget.
            }

            // OR
            fn child(text) -> impl UiNode {
                text(text) // generate child node, text is a captured property?
            }
            // `new_child` still exists?

            // how to support multiple children?
            fn children(?) -> ? {
                ?
            }

            // OR
            default_child {
                content -> ui_node_property: required!;
                padding -> margin;//?
            }

            fn new_child(content) -> impl UiNode {
                content.unwrap()
            }

            // default:
            fn new_child() -> impl UiNode {
                NilUiNode// UiNode that does nothing.
            }
        }
    };

    (what_to_support) => {
        text! {
            text: "Hello!";
            color: rgb(0, 0, 0);
        }

        text_box! {
            text: var("Hello!");

        }

        grid! {
            columns: vec![
                (),
                (),
            ];
            rows: vec![
                (),
                (),
            ];
            items: ui_vec![
                text! { column: 0; row: 1; column_span: 2; text: "" }
            ]
        }

        browser! {
            url: "";

        }

        spreadsheet! {
            data: var
        }

        list! {
            items: [
                list_item! {

                }
            ]

            items_source: var;
            item_template: |item| {
                list_item! {
                    todo!()
                }
            }

            border: todo!();

            // in widget!
            fn new_child(items) -> impl UiNode {
                todo!()
            }

            fn new_child(items_source, item_template) -> impl UiNode {
                todo!()
            }
        }

        data_list! {
            items_source: var;
            item_template: |item| {
                list_item! {
                    todo!()
                }
            }

            border: todo!();
        }
    };
}
