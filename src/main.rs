pub mod app;
pub mod ui;
mod window;

use ui::*;
use webrender::api::{GradientStop, LayoutPoint};

fn main() {
    app::App::new() //
        .window("window1", rgbaf(0.1, 0.2, 0.3, 1.0), |c| {
            text(c, "çHello World! pç", rgb(0, 0, 0))
                .background_color(rgb(255, 255, 255))
                .center()
        })
        .window("window2", rgbaf(0.3, 0.2, 0.1, 1.0), |_| {
            center(h_stack(vec![
                fill_gradient(
                    LayoutPoint::new(0., 0.),
                    LayoutPoint::new(1., 1.),
                    vec![
                        GradientStop {
                            offset: 0.,
                            color: rgb(0, 200, 0),
                        },
                        GradientStop {
                            offset: 1.,
                            color: rgb(200, 0, 0),
                        }
                    ]
                )
                .width(200.)
                .margin(2.);
                4
            ]))
        })
        .run();
}
