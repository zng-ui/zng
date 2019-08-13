pub mod app;
//pub mod button;
pub mod ui;
pub mod window;

use ui::*;
use webrender::api::*;

fn main() {
    let r_color = ColorF::new(0.2, 0.4, 0.1, 1.);
    let r_size = LayoutSize::new(554., 50.);
    app::App::new()
        .window(
            "window1",
            ColorF::new(0.1, 0.2, 0.3, 1.0),
            center(v_list(vec![
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin_ltrb(20., 2., 20., 2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
            ])),
        )
        .window(
            "window2",
            ColorF::new(0.3, 0.2, 0.1, 1.0),
            center(v_list(vec![
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
                Box::new(Rect::new(r_color).size(r_size).margin(2.)),
            ])),
        )
        .run();
}
