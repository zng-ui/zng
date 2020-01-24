#![recursion_limit = "256"]

#[macro_use]
extern crate zero_ui;

use zero_ui::prelude::*;
use zero_ui::test;

fn main() {
    //normal_run();
    custom_run();
}

#[allow(unused)]
fn normal_run() {
    let data = Var::new(vec![0; 1024]);
    let bad = &*data;
    let data_ref = Var::clone(&data);

    app::run(rgba(0.1, 0.2, 0.3, 1.0), LayoutSize::new(800., 800.), move |_| {
        button! {
            background_color: rgb(100, 0, 0);
            font_family: "Arial";
            font_size: 28;
            text_color: rgb(255, 255, 255);

            on_click: enclose! { (data_ref) move |_, u| {
                println!("Changed data");
               u.change(&data_ref, |d|d[0] += 1);
            }};

            => text("Change Data")
        }
    });

    //println!("dangling: {:?}", bad[0]);
}

#[allow(unused)]
fn custom_run() {
    let data = Var::new(vec![0; 1024]);
    let bad = &*data;
    let data_ref = Var::clone(&data);

    let (_fake_renderer, mut ui) = test::test_ui_root(
        LayoutSize::new(800., 800.),
        96.,
        Box::new(move |_| {
            let ui = button! {
                background_color: rgb(100, 0, 0);
                font_family: "Arial";
                font_size: 28;
                text_color: rgb(255, 255, 255);

                on_click: enclose! { (data_ref) move |_, u| {
                    println!("Changed data");
                   u.change(&data_ref, |d|d[0] += 1);
                }};

                => text("Change Data")
            };
            ui.boxed()
        }),
    );

    ui.mouse_move(LayoutPoint::new(400., 400.), ModifiersState::default());
    ui.mouse_input(ElementState::Pressed, MouseButton::Left, ModifiersState::default());
    ui.mouse_input(ElementState::Released, MouseButton::Left, ModifiersState::default());

    println!("dangling: {:?}", bad[0]);
}
