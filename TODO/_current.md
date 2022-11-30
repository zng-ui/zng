* Implement inline layout.
    - Just flow LTR for now.
    - `text!`.
        - Use `TEXT_WRAP_VAR`, and line-break, word break, hyphens.
        - Implemented in `shape_text`?
    - Test code for text wrap:
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
                background_color = colors::GRAY;
            },
            text! {
                txt = "denique scriptorem";
                background_color = colors::RED;
            },
            text! {
                txt = " ius ei quem graece. Mei hinc iisque id, imperdiet pertinacia eum no. Ne eius porro exerci has. Eam laoreet deleniti adolescens ei, an pro meis vidisse menandri. Ei quas putent vel, eu vel placerat adipisci, et nam vide iriure nominavi.";
                background_color = colors::GRAY;
            },
        ]
    }
}

```




* Implement inline info in bounds info.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?