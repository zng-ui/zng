# Bug

* Priority of when conditions that extend an inherited when condition is not right.

```rust
    use zero_ui::prelude::new_widget::*;

    #[widget($crate::when_test::base_widget)]
    pub mod base_widget {
        use super::*;

        properties! {
            background_color = colors::RED;

            when self.is_hovered {
                background_color = colors::YELLOW;
            }
        }
    }

    #[widget($crate::when_test::final_widget)]
    pub mod final_widget {
        use super::*;

        inherit!(base_widget);

        properties! {
            when self.is_pressed {
                background_color = colors::GREEN;
            }
        }
    }
```

# Text

* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline
