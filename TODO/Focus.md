# Focus TODO

* Restore focus from `modal` focus scope to button that opened it. 
* Restore focus to nearest sibling when focused is removed.
* Support more then one ALT scopes?
* Test directional navigation.
   * Including layout transformed widgets.
* Mnemonics.

## Icon Example

* Focus moving to scrollable when using left and right arrows in the middle row
* Focus moving twice when cycling from one of the icons
* The priority of keyboard focus should be high when highlighting and low when not?
* `move_focus` changes highlight to true when scrolling with arrow keys

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::prelude::*;
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-_test.json.gz", &[("example", &"_test")], |_| true);

    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            content_align = Align::CENTER;
            // zero_ui::widgets::inspector::show_center_points = true;
            content = v_stack! {
                focusable = true;
                background_color = colors::DARK_RED;
                when self.is_focused {
                    background_color = colors::GREEN;
                }

                padding = 10;
                // padding = (20, 10, 10, 10);

                spacing = 10;
                items = widgets![
                    focusable_item(),
                    focusable_item(),
                    // focusable_item(),
                    focusable_item(),
                    focusable_item(),
                ]
            };
        }
    })
}

fn focusable_item() -> impl Widget {
    text! {
        focusable = true;
        padding = 10;
        font_size = 20;
        background_color = colors::DARK_RED;
        when self.is_focused {
            background_color = colors::GREEN;
        }

        text = "Focusable Item";
    }
}
```