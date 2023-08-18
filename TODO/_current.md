```rust
    use zero_ui_material_icons as icons;
    App::default().extend(icons::MaterialFonts).run_window(async {
        Window! {
            //background_color = colors::YELLOW;
            child_align = Align::CENTER;
            child = Icon!{
                ico = icons::outlined::N3G_MOBILEDATA;
                //size = 500;
                background_color = colors::RED;
                border = 10, colors::GREEN.with_alpha(20.pct());
            };
        }
    })
```

# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Text

* Implement text clip.
    - Ellipses, fade-out.
    - Very visible in icon example.

# Tooltip

* Tooltips stop showing upon interaction (click/tab/enter/etc) in HTML.
    - Ours doesn't.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement OpenGL example.
    - Overlay and texture image.
* Implement automation/screen reader APIs.

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.
