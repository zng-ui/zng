//#![windows_subsystem = "windows"]

pub mod app;
pub mod ui;
mod window;

use ui::*;
use webrender::api::{GradientStop, LayoutPoint};

fn main() {
    let string = r".....
    ÃÂÃÂÃÂÃÂÃÂ
    Economize tempo no Word com novos botões que são mostrados
     no local em que você precisa deles.
     Para alterar a maneira como uma imagem se ajusta ao seu documento,
    clique nela e um botão de opções de layout será exibido ao lado.";

    app::App::new() //
        .window("window1", rgbaf(0.1, 0.2, 0.3, 1.0), |c| {
            v_stack(
                string
                    .split("\n")
                    .map(|l| text(c, l, rgb(0, 0, 0), "Arial", 14).background_color(rgb(255, 255, 255)))
                    .collect::<Vec<_>>(),
            )
            .on_keydown(|k, _| println!("on_keydown: {}", k))
            .on_keyup(|k, _| println!("on_keyup: {}", k))
            .on_mousedown(|m, _| println!("on_mousedown: {}", m))
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
