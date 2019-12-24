//#![windows_subsystem = "windows"]

#[macro_use]
extern crate zero_ui;

use zero_ui::prelude::*;

fn main() {
    //start_logger_for("gradient_text");
    app::run(rgb(0.1, 0.2, 0.3), LayoutSize::new(800., 600.), main_window);
}

fn main_window(_: &mut NextUpdate) -> impl Ui {
    let string = r".....
    ÃÂÃÂÃÂÃÂÃÂ
    Economize tempo no Word com novos botões que são mostrados
     no local em que você precisa deles.
     Para alterar a maneira como uma imagem se ajusta ao seu documento,
    clique nela e um botão de opções de layout será exibido ao lado.";

    let paragraph = v_stack(
        string
            .split('\n')
            .map(|l| {
                ui! {
                    background_color: rgb(255, 255, 255);
                    cursor: CursorIcon::Hand;

                    on_click: |_c, _| {
                        //c.stop_propagation();
                    };
                    => text(l)
                }
            })
            .collect(),
    );

    ui! {
        font_family: "Arial";
        font_size: 14;
        on_click: |_, u| {
            u.create_window(rgb(0.3, 0.2, 0.1), LayoutSize::new(1000., 800.), other_widow);
        };
        => paragraph
    }
}

fn other_widow(_: &mut NextUpdate) -> impl Ui {
    center(h_stack((0..4).map(item).collect()))
}

fn item(i: usize) -> impl Ui {
    let box_border = Var::new(rgba(0, 0, 0, 0.0));
    let text_border = Var::new(rgba(0, 0, 0, 0.0));

    let text = ui! {
        font_family: "Arial";
        font_size: 60;
        text_color: rgb(0, 0, 0);
        background_color: rgba(1., 1., 1., 0.5);
        border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
        cursor: CursorIcon::Hand;

        => text(format!("Item {}", i  + 1))
    };

    ui! {
        background_gradient: (0., 0.), (1., 1.), vec![rgb(0, 200, 0), rgb(200, 0, 0)];
        border: 4., (Var::clone(&box_border), BorderStyle::Dashed);
        focusable: default;
        margin: 2.0;
        width: 200.0;

        => center(text)
    }
}
