# Inline Bidi

* Can't combine levels?
  - levels of a single text run are different from the same text in parts in a wrap panel.
  - Only space segments for now, but this highlights a real issue, an entire widget can be quoted by
    special bidi chars before and after it.
  - We need to compute levels on the `wrap!`.
  - No need to store levels in the measure info?
* Wrap panels need to do something about blocks.
  - Treat then like an isolated insert?
* Wrap panels need to shape the "row" for each widget in its row to cover all reordered segments.
  - Change horizontal positioning all to resort algorithm.
  - But still track wrap the old way?

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]

use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();

    zero_ui_view::init();

    // let rec = examples_util::record_profile("_expand");

    app_main();

    // rec.finish();

    // zero_ui_view::run_same_process(app_main);
}

fn app_main() {
    App::default().run_window(|_| {
        let other_is_hovered = var(false);
        window! {
            // zero_ui::properties::inspector::show_bounds = true;
            child_align = Align::CENTER;
            child = markdown_rtl();
        }
    })
}

fn markdown_rtl() -> impl UiNode {
    stack! {
        lang = lang!("ar");
        direction = StackDirection::top_to_bottom();
        font_size = 16.pt();
        // spacing = 60;
        children = ui_vec![
            wrap! {
                children = ui_vec![
                    text! {
                        txt = "النص ثنائي الاتجاه (بالإنجليزية: Bi ";
                        background_color = colors::BROWN;
                    },
                    text! {
                        txt = "directional";
                    },
                    text! {
                        txt = " text)‏ هو نص يحتوي على نص في كل من";
                        background_color = colors::GREEN;
                    },
                ]
            },
            text!("النص ثنائي الاتجاه (بالإنجليزية: Bi directional text)‏ هو نص يحتوي على نص في كل من"),
        ]
    }
}
```

```html
<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="X-UA-Compatible" content="ie=edge">
  <title>udhr_arb</title>
  <style>
    * {
      margin: 0;
      padding: 0;
    }

    body {
      background-color: rgb(25, 25, 25);
    }

    p {
      font-family: 'Segoe UI';
      font-size: 16pt;
      color: #FFF;
      white-space: pre-wrap;
    }

    span.a {
      background-color: brown;
    }

    span.b {
      background-color: green;
    }
  </style>
</head>

<body lang="ar" dir="rtl">
  <p><span class="a">النص ثنائي الاتجاه (بالإنجليزية: Bi </span>directional<span class="b"> text)‏ هو نص يحتوي على نص في كل من</span></p>
  <p>النص ثنائي الاتجاه (بالإنجليزية: Bi directional text)‏ هو نص يحتوي على نص في كل من</p>
</body>

</html>
```

# Other

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.