#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use enclose::enclose;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let shortcut_text = var(Text::empty());
        let keypress_text = var(Text::empty());

        window! {
            title: "Shortcuts Example";
            auto_size: true;
            margin: 50;
            on_shortcut: enclose! { (shortcut_text) move |ctx, args| {
                shortcut_text.set(ctx.vars, args.shortcut.to_text())
            }};
            on_key_down: enclose! { (keypress_text, shortcut_text) move |ctx, args| {
                shortcut_text.set(ctx.vars, "".into());
                keypress_text.set(ctx.vars,
                    if let Some(key) = args.key {
                        formatx!{"{:?}", key}
                    } else {
                        formatx!{"Scan Code: {:?}", args.scan_code}
                    }
                )
            }};
            content: v_stack! {
                items: ui_vec! [
                    text!{
                        align: Alignment::CENTER;
                        font_size: 18.pt();
                        text: "Press a shortcut:";
                    },
                    text! {
                        align: Alignment::CENTER;
                        margin: (10, 0);
                        font_size: 28.pt();
                        text: shortcut_text;
                    },
                    text! {
                        align: Alignment::CENTER;
                        font_size: 22.pt();
                        font_family: FontName::monospace();
                        color: web_colors::LIGHT_SLATE_GRAY;
                        text: keypress_text;
                    }
                ];
            };
        }
    })
}
