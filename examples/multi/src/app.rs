use zng::prelude::*;

pub fn run() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            child = Text! {
                txt = formatx!("HELLO {}!", std::env::consts::OS.to_uppercase());
                txt_align = Align::TOP;
                layout::padding = 20;
                font_family = "fantasy";
                font_size = 2.em();
                font_weight = FontWeight::BOLD;
                font_style = FontStyle::Italic;
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
            };
        }
    });
}

#[allow(unused)]
pub fn run_headless() {
    zng::env::init!();

    let _app = APP.minimal().run_headless(false);

    tracing::info!("Debug tracing logs to console");
    tracing::warn!("Warn example!");
    tracing::error!("Error example!");

    let _web_time = INSTANT.epoch();
}
