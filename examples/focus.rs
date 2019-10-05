//#![windows_subsystem = "windows"]

use zero_ui::{ui::*, *};

fn main() {
    //start_logger_for("log_target");
    app::run(rgbaf(0.1, 0.2, 0.3, 1.0), LayoutSize::new(800., 600.), widow);
}

fn widow(_: &mut NextUpdate) -> impl Ui {
    h_stack((0..4).map(item).collect::<Vec<_>>()).center()
}

fn item(i: usize) -> impl Ui {
    let bkg_color = Var::new(rgb(255, 255, 255));
    text("Ola")
        .font_family("Arial".to_owned())
        .font_size(90)
        .background_color(Var::clone(&bkg_color))
        .text_color(rgb(0, 150, 0))
        .focusable()
        .on_key_down(move |k, _| {
            println!("Key down @ text.{}: {}", i, k);
            //k.stop_propagation();
        })
        .center()
        .cursor(CursorIcon::Hand)
        .on_mouse_enter(enclose! {(bkg_color) move |u| {
            u.set(&bkg_color, rgb(100, 255, 255));
        }})
        .on_mouse_leave(move |u| {
            u.set(&bkg_color, rgb(255, 255, 255));
        })
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
        .focusable()
        .on_key_down(move |k, _| println!("Key down @ gradient.{}: {}", i, k))
        .width(200.)
        .margin(2.)
}
