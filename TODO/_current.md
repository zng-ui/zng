# Nested Animation Changes

* New handle for each animation.
* Shared importance for nested animations.

* Can't scroll with wheel in inspector window after it is focused by parent.

* `wrap!` bugs:
    - Need to track row height?
    - Need to track all rows in the `InlineLayout`?
    - Does not grow to fit children when possible.

* Implement `markdown!`.
* Implement inline info in bounds info.
* Implement `TextAlign` across multiple inlined texts.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.

```rust
use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            padding = 20;
            child = text_wrap();
        }
    });
}

fn text_wrap() -> impl UiNode {
    wrap! {
        children = ui_list![
            text! {
                txt = "Lorem ipsum dolor sit amet, ne duo fugit atomorum maiestatis, vim harum ridens nusquam ei. Sit suas ";
                background_color = colors::LIGHT_SALMON;
            },
            text! {
                txt = "denique scriptorem";
                background_color = colors::LIGHT_PINK;
            },
        ]
    }
}
```

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?