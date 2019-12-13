//#![windows_subsystem = "windows"]
#![recursion_limit = "512"]

use zero_ui::{core::*, primitive::*, widget::*, *};

fn main() {
    //start_logger_for("test");
    app::run(rgba(0.1, 0.2, 0.3, 1.0), LayoutSize::new(800., 800.), window);
}

fn window(u: &mut NextUpdate) -> impl Ui {
    let menu_fkey = FocusKey::new_unique();
    u.focus(FocusRequest::Direct(menu_fkey));
    v_stack(
        (
            // menu
            ui! {
                focus_scope: move |s| s.menu().key(menu_fkey);
                => line(100., "menu")
            },
            // grid
            v_stack((0..4).map(|_| line(200., "Olá")).collect()),
        )
            .into(),
    )
}

fn line(h: f32, text: &'static str) -> impl Ui {
    ui! {
        height: h;
        => h_stack((0..4).map(|i| item(i, text)).collect())
    }
}

fn item(_: usize, txt: &'static str) -> impl Ui {
    let box_border = Var::new(rgba(0, 0, 0, 0.0));
    let text_border = Var::new(rgba(0, 0, 0, 0.0));

    let text_id = UiItemId::new_unique();

    let text = ui! {
        id: text_id;
        font_family: "Arial";
        font_size: 60;
        text_color: rgb(0, 0, 0);
        background_color: rgba(1., 1., 1., 0.5);
        border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
        focusable: default;
        cursor: CursorIcon::Hand;

        on_focus: enclose! {(text_border) move |u| {
            u.set(&text_border, rgb(145, 218, 255));
        }};
        on_blur: move |u| {
            u.set(&text_border, rgba(0, 0, 0, 0.0));
        };

        => button(text(txt), |bi, u|{
            println!("button click {:?}", bi)
        })
    };

    ui! {
        background_gradient: {
            start: (0., 0.),
            end: (1., 1.),
            stops: vec![rgb(0, 200, 0), rgb(200, 0, 0)]
        };
        border: 4., (Var::clone(&box_border), BorderStyle::Dashed);
        focusable: default;
        margin: 2.0;
        width: 200.0;

        on_focus: enclose! {(box_border)move |u| {
            u.set(&box_border, rgb(145, 218, 255));
        }};
        on_blur: move |u| {
            u.set(&box_border, rgba(0, 0, 0, 0.0));
        };

        => center(text)
    }
}
