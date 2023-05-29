#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui::core::config::*;

use zero_ui::prelude::new_widget::{Dip, DipPoint, DipVector};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("button");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        CONFIG.load(JsonConfig::sync("target/tmp/example.config.json"));

        let checked = CONFIG.get("main.checked", || false);
        let count = CONFIG.get("main.count", || 0);
        let txt = CONFIG.get("main.txt", || "Save this".to_text());

        Window! {
            title = if std::env::var("MOVE-TO").is_err() { "Config Example" } else { "Config Example - Other Process" };
            background = Text! {
                txt = CONFIG.status().map_to_text();
                margin = 10;
                font_family = "monospace";
                align = Align::TOP_LEFT;
                font_weight = FontWeight::BOLD;

                when *#{CONFIG.status().map(|s| s.is_err())} {
                    txt_color = colors::RED;
                }
            };
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                align = Align::CENTER;
                spacing = 5;
                children = ui_vec![
                    Toggle! {
                        child = Text!(checked.map(|c| formatx!("Checked: {c:?}")));
                        checked = checked.clone();
                    },
                    Button! {
                        child = Text!(count.map(|c| formatx!("Count: {c:?}")));
                        on_click = hn!(count, |_| {
                            count.modify(|c| *c.to_mut() += 1).unwrap();
                        })
                    },
                    separator(),
                    TextInput! {
                        txt = txt.clone();
                        min_width = 100;
                    },
                    separator(),
                    Button! {
                        child = Text!("Reset");
                        on_click = hn!(|_| {
                            checked.set_ne(false).unwrap();
                            count.set_ne(0).unwrap();
                            txt.set_ne("Save this").unwrap();
                        })
                    },
                    Button! {
                        child = Text!("Open Another Process");
                        on_click = hn!(|_| {
                            let offset = Dip::new(30);
                            let pos = WINDOW_CTRL.vars().actual_position().get() + DipVector::new(offset, offset);
                            let pos = pos.to_i32();
                            let r: Result<(), Box<dyn std::error::Error>> = (|| {
                                let exe = std::env::current_exe()?;
                                std::process::Command::new(exe).env("MOVE-TO", format!("{},{}", pos.x, pos.y)).spawn()?;
                                Ok(())
                            })();
                            match r {
                                Ok(_) => println!("Opened another process"),
                                Err(e) => eprintln!("Error opening another process, {e:?}"),
                            }
                        })
                    }
                ];
            };
            on_load = hn_once!(|_| {
                if let Ok(pos) = std::env::var("MOVE-TO") {
                    if let Some((x, y)) = pos.split_once(',') {
                        if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                            let pos = DipPoint::new(Dip::new(x), Dip::new(y));
                            WINDOW_CTRL.vars().position().set(pos);
                            WINDOWS.focus(WINDOW.id()).unwrap();
                        }
                    }
                }
            });
        }
    })
}

fn separator() -> impl UiNode {
    Hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}
