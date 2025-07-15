//! Demonstrates the CONFIG and SETTINGS services, live updating config between processes.

use zng::{layout::align, prelude::*};

use zng::config::{settings::SETTINGS, *};

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();
    app_main();
}

fn load_config() {
    // settings file for the app, keys with prefix "settings." are saved here.
    let user_settings = JsonConfig::sync(zng::env::config("example-config/settings.json"));
    // entries not found in `user_settings` bind to this file first before going to embedded fallback.
    let default_settings = ReadOnlyConfig::new(JsonConfig::sync(zng::env::res("default-settings.json")));

    let settings = FallbackConfig::new(user_settings, default_settings);
    // Clone a ref that can be used to reset specific entries.
    let settings_ref = settings.clone_boxed();

    // any other configs (Window::save_state for example)
    let other_cfg = JsonConfig::sync(zng::env::config("example-config/config.json"));

    // switch over configs, the prefix is striped from the inner configs.
    CONFIG.load(SwitchConfig::new().with_prefix("settings.", settings).with_prefix("", other_cfg));

    // register settings metadata
    SETTINGS.register_categories(|c| {
        c.entry("bool", |c| c.name("Booleans"))
            .entry("integers", |c| c.name("Integers"))
            .entry("floats", |c| c.name("Floats"))
            .entry("strings", |c| c.name("Strings"));
    });

    SETTINGS.register(move |s| {
        s.entry("settings.bool", "bool", |s| {
            s.name("bool")
                .description("Example *bool* value.")
                .value(false)
                .reset(settings_ref.clone_boxed(), "settings.")
        });

        macro_rules! examples {
            ($([$ty:tt, $cat:tt, $default:expr]),+ $(,)?) => {
                $(
                    s.entry(concat!("settings.", $ty), $cat, |s| {
                        s.name($ty)
                            .description(concat!("Example *", $ty, "* value."))
                            .value($default)
                            .reset(settings_ref.clone_boxed(), "settings.")
                    });
                )+
            };
        }
        examples! {
            ["u8", "integers", 0u8],
            ["u16", "integers", 0u16],
            ["u32", "integers", 0u32],
            ["u64", "integers", 0u64],
            ["u128", "integers", 0u128],
            ["i8", "integers", 0i8],
            ["i16", "integers", 0i16],
            ["i32", "integers", 0i32],
            ["i64", "integers", 0i64],
            ["i128", "integers", 0i128],

            ["f32", "floats", 0f32],
            ["f64", "floats", 0f64],

            ["Txt", "strings", Txt::from("")],
            ["String", "strings", String::new()],
            ["Char", "strings", 'c'],
        };
    });
}

fn app_main() {
    APP.defaults().run_window(async {
        load_config();

        WINDOW.id().set_name("main").unwrap(); // name used to save window state.
        Window! {
            title = if std::env::var("OTHER-PROCESS").is_err() { "Config Example" } else { "Config Example - Other Process" };
            size = (600, 500);
            // settings editor, usually not on the main window
            child = zng::config::settings::editor::SettingsEditor! {
                id = "settings";
            };
            child_bottom = Container! {
                child_out_top = Hr!(layout::margin = 0), 0;
                padding = 10;

                // status
                child_left = Text! {
                    txt = CONFIG.status().map_to_txt();
                    layout::margin = 10;
                    font_family = "monospace";
                    align = Align::TOP_LEFT;
                    font_weight = FontWeight::BOLD;

                    when *#{CONFIG.status().map(|s| s.is_err())} {
                        font_color = colors::RED;
                    }
                }, 0;

                // spawn another process to demonstrate the live update of configs
                child_right = Button! {
                    child = Text!("Open Another Process");
                    on_click = hn!(|_| {
                        let offset = layout::Dip::new(30);
                        let pos = WINDOW.vars().actual_position().get() + layout::DipVector::new(offset, offset);
                        let pos = pos.to_i32();
                        let r: Result<(), Box<dyn std::error::Error>> = (|| {
                            let exe = dunce::canonicalize(std::env::current_exe()?)?;
                            std::process::Command::new(exe).env("OTHER-PROCESS", format!("{},{}", pos.x, pos.y)).spawn()?;
                            Ok(())
                        })();
                        match r {
                            Ok(_) => tracing::info!("Opened another process"),
                            Err(e) => tracing::error!("Error opening another process, {e:?}"),
                        }
                    })
                }, 0;
            }, 0;
            on_load = hn_once!(|_| {
                // window position is saved, so we move the second window a bit
                if let Ok(pos) = std::env::var("OTHER-PROCESS")
                 && let Some((x, y)) = pos.split_once(',')
                 && let Ok(x) = x.parse() && let Ok(y) = y.parse() {
                        let pos = (layout::Dip::new(x), layout::Dip::new(y));
                        WINDOW.vars().position().set(pos);
                        WINDOWS.focus(WINDOW.id()).unwrap();
                }
            });
        }
    })
}
