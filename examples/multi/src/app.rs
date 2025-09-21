use zng::prelude::*;

#[allow(unused)]
pub fn run() {
    APP.defaults().run_window(async {
        CONFIG.load(zng::config::JsonConfig::sync(zng::env::config("config.json")));

        let res = zng::env::res("my-res.txt");
        match std::fs::read_to_string(&res) {
            Ok(res) => tracing::info!("resource read ok: {res}"),
            Err(e) => tracing::error!("resource read error, cannot read '{}', {e}", res.display()),
        }

        let count = CONFIG.get("count", 0u32);
        Window! {
            child = Button! {
                style_fn = zng::button::LightStyle!();
                child_align = Align::TOP;
                child = Text! {
                    txt = count.map(|&c| formatx!("HELLO {}!\n\n{c}", std::env::consts::OS.to_uppercase()));
                    txt_align = Align::CENTER;
                    layout::padding = 40;
                    font_family = "fantasy";
                    font_size = 22.pt();
                    font_weight = FontWeight::BOLD;
                    font_style = FontStyle::Italic;
                };
                on_click = hn!(count, |_| {
                    let c = count.get().wrapping_add(1);
                    tracing::info!("Clicked {c} times!");
                    count.set(c);
                });
                context_menu = ContextMenu!(ui_vec![Button! {
                    child = Text!("Reset");
                    on_click = hn!(|_| {
                        count.set(0u32);
                    });
                }]);
            };
            color_scheme = color::ColorScheme::Dark;
            widget::background_gradient = {
                axis: 180.deg(),
                stops: color::gradient::stops![
                    (hex!(#1F214D), 0.pct()),
                    (hex!(#50366F), 80.pct()),
                    (hex!(#BF3475), 90.pct()),
                    (hex!(#EE6C45), 99.pct()),
                    (hex!(#FFCE61), 100.pct()),
                ],
            };
        }
    });
}

#[allow(unused)]
pub fn run_headless() {
    let _app = APP.minimal().run_headless(false);

    tracing::info!("Debug tracing logs to console");
    tracing::warn!("Warn example!");
    tracing::error!("Error example!");

    let _web_time = INSTANT.epoch();
}
