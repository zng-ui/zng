* Implement baseline in widget layout (see CSS vertical-align).
* Glyphs are inserted for space, but not for new line.
   - make optional glyphs for spaces, new line and tab?
    - needs to be integrated with selection and line width

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 


# ShapedText investigation example
```Rust
use zero_ui::core::text::*;
use zero_ui::prelude::*;

fn main() {
    let mut app = App::default().run_headless(false);
    let font = app.ctx().services.fonts().get(
        &FontName::sans_serif(),
        FontStyle::Normal,
        FontWeight::NORMAL,
        FontStretch::NORMAL,
        &Default::default(),
    ).unwrap();
    let font = font.sized(Px(20), vec![]);
    let segmented_text = SegmentedText::new("t\n\tt");
    let shaped_text = font.shape_text(&segmented_text, &TextShapingArgs::default());
    println!("{shaped_text:#?}")
}
```