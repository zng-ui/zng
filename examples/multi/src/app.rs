use zng::prelude::*;

#[allow(unused)]
pub fn run() {
    APP.defaults().run_window(async {
        let count = var(0u32);
        Window! {
            child = Button! {
                style_fn = zng::button::LightStyle!();
                child_align = Align::TOP;
                child = Text! {
                    txt = count.map(|&c| {
                        let mut txt = formatx!("HELLO {}!", std::env::consts::OS.to_uppercase());
                        if c > 0 {
                            use std::fmt::Write;
                            write!(&mut txt, "\n{c}");
                        }
                        txt
                    });
                    txt_align = Align::CENTER;
                    layout::padding = 20;
                    font_family = "fantasy";
                    font_size = 2.em();
                    font_weight = FontWeight::BOLD;
                    font_style = FontStyle::Italic;

                };
                on_click = hn!(|_| {
                    let c = count.get() + 1;
                    tracing::info!("Clicked {c} times!");
                    count.set(c);
                });
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
