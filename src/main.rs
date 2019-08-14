pub mod app;
pub mod ui;
pub mod window;

use ui::*;
use webrender::api::*;

fn main() {
    let r_color = ColorF::new(0.2, 0.4, 0.1, 1.);
    app::App::new()
        .window(
            "window1",
            ColorF::new(0.1, 0.2, 0.3, 1.0),
            center(v_list(
                std::iter::repeat_with(|| Rect::new(r_color).height(150.).margin(2.).into_box())
                    .take(8)
                    .collect(),
            )),
        )
        .window(
            "window2",
            ColorF::new(0.3, 0.2, 0.1, 1.0),
            center(h_list(
                std::iter::repeat_with(|| Rect::new(r_color).width(200.).margin(2.).into_box())
                    .take(8)
                    .collect(),
            )),
        )
        .run();
}
