//#![windows_subsystem = "windows"]

pub mod app;
pub mod ui;
mod window;

use ui::*;
use webrender::api::{GradientStop, LayoutPoint};

fn main() {
    //start_logger_for("log_target");

    fn main_window(next_update: &mut NextUpdate) -> impl Ui {
        let string = r".....
    ÃÂÃÂÃÂÃÂÃÂ
    Economize tempo no Word com novos botões que são mostrados
     no local em que você precisa deles.
     Para alterar a maneira como uma imagem se ajusta ao seu documento,
    clique nela e um botão de opções de layout será exibido ao lado.";
        v_stack(
            string
                .split("\n")
                .map(|l| text(next_update, l, rgb(0, 0, 0), "Arial", 14).background_color(rgb(255, 255, 255)))
                .collect::<Vec<_>>(),
        )
        .on_click(|m, next_update| {
            println!("on_click: {}", m);
            next_update.create_window(rgbaf(0.3, 0.2, 0.1, 1.0), LayoutSize::new(1000., 800.), other_widow);
        })
    }

    fn other_widow(next_update: &mut NextUpdate) -> impl Ui {
        center(h_stack(
            (0..4)
                .map(|i| {
                    text(next_update, "Ola", rgb(0, 0, 0), "Arial", 90)
                        .background_color(rgb(255, 255, 255))
                        .center()
                        .cursor(CursorIcon::Hand)
                        .on_mouse_down(move |m, _| println!("'Text#{}'.on_mouse_down: {}", i, m))
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
    }

    app::App::new().run(rgbaf(0.1, 0.2, 0.3, 1.0), LayoutSize::new(800., 600.), main_window);
}
