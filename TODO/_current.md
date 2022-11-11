* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
    - Implemented, test.
    - This fails after one hover (text stops rendering).
```rust
use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            child_align = Align::CENTER;
            font_size = 48;

            child = text("Hello");

            when *#is_hovered {
                child = text("hovered!");
            }
        }
    });
}
```
    - Same for this:
```rust
use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            font_size = 48;
            
            child = v_stack! {
                children_align = Align::CENTER;
                children = ui_list![
                    text("normal"),
                    text("normal"),
                ];

                when *#is_hovered {
                    children = ui_list![
                        text("hovered!"),
                        text("hovered!"),
                    ];
                }
            };            
        }
    });
}
```

    - This works at least:
```rust
use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            font_size = 48;
            
            child_align = Align::CENTER;
            child = button! {
                child = text("Click ME!");

                on_click = hn!(|_, _| {
                    println!("normal click");
                });

                when *#is_hovered {
                    on_click = hn!(|_, _| {
                        println!("hovered click");
                    });
                }
            }      
        }
    });
}

```

* Implement all `todo!` code.