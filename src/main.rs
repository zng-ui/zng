//#![windows_subsystem = "windows"]

pub mod app;
pub mod ui;
mod window;

use ui::*;
use webrender::api::{GradientStop, LayoutPoint};

fn main() {
    //start_logger_for("log_target");

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
            //.log_layout("log_target")
            .on_key_down(|k, _| println!("on_key_down: {}", k))
            .on_key_up(|k, _| println!("on_key_up: {}", k))
            .on_mouse_down(|m, _| println!("on_mouse_down: {}", m))
            .on_mouse_up(|m, _| println!("on_mouse_up: {}", m))
            //.on_mouse_move(|m, _| println!("on_mouse_move: {}", m.position))
        })
        .window("window2", rgbaf(0.3, 0.2, 0.1, 1.0), |c| {
            center(h_stack(
                (0..4)
                    .map(|i| {
                        text(c, "Ola", rgb(0, 0, 0), "Arial", 90)
                            .background_color(rgb(255, 255, 255))
                            .cursor(CursorIcon::Hand)
                            .on_mouse_down(move |m, _| println!("'Text#{}'.on_mouse_down: {}", i, m))
                            .center()
                            .background_gradient(
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
                                    },
                                ],
                            )
                            .width(200.)
                            .on_mouse_enter(move |_| println!("'Gradient#{}'.on_mouse_enter", i))
                            .on_mouse_leave(move |_| println!("'Gradient#{}'.on_mouse_leave", i))
                            .margin(2.)
                    })
                    .collect::<Vec<_>>(),
            ))
        })
        .run();
}
