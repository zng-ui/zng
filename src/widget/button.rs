use crate::core::*;
use crate::primitive::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Arguments for [button] click event. A button click
/// can be initiated by the mouse, keyboard or touch screen.
#[derive(Debug)]
pub enum ButtonInput {
    /// Mouse left button click.
    Mouse(ClickInput),
    /// Keyboard enter or space key tap.
    Keyboard(KeyTap),
    // TODO Touch(TouchInput)
}

impl ButtonInput {
    /// Returns keyboard modifiers state.
    pub fn modifiers(&self) -> ModifiersState {
        match self {
            ButtonInput::Mouse(ci) => ci.modifiers,
            ButtonInput::Keyboard(kt) => kt.modifiers,
        }
    }

    pub fn stop_propagation(&self) {
        match self {
            ButtonInput::Mouse(ci) => ci.stop_propagation(),
            ButtonInput::Keyboard(kt) => kt.stop_propagation(),
        }
    }
}

// Declares a button! {} macro.
ui_widget! {
    // Properties applied to child before calling widget fn.
    child_properties {
        // Property declaration without default value, if not set does not apply.
        // If set applies margin to child.
        padding -> margin;
        // Same with default value.
        content_align -> align: CENTER ;
        // Default value of background_color property that is applied to child.
        background_color: rgb(0, 0, 0);
    }


    // Properties applied to return of widget fn. Same sintax as
    // child_properties.
    self_properties {
        border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
    }

    // widget signature, must name the parameters after child,
    // they behave like required properties in the declared button! macro.

    /// Button widget.
    /// # Arguments
    /// * `on_click`: Required button click event handler.
    /// * `padding`: Margin around the button content.
    /// * `background_color`:
    /// * `border`:
    pub fn button(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static) -> impl Ui {
        let on_click = Rc::new(RefCell::new(on_click));
        ui! {
            focusable: default;
            on_click: enclose! ((on_click) move |ci, n|{
                if ci.button == MouseButton::Left {
                    (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
                }
            });
            on_key_tap: move |kt, n|{
                if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
                    (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
                }
            };
            => child
        }
    }
}

/*
//
// /// Button widget.
// /// # Arguments
// /// * `on_click`: Required button click event handler.
// /// * `padding`: Margin around the button content.
// /// * `background_color`:
// /// * `border`:
// #[allow(unused)]
// macro_rules! button {
//     ($($tt:tt)*) => {
//         custom_ui! {
//             child_properties {
//                 padding -> margin;
//                 content_align: CENTER -> align;
//                 background_color: rgb(0, 0, 0);
//             }
//
//             self_properties {
//                 border: border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
//             }
//
//             args {
//                 $($tt)*
//             }
//
//             fn button(on_click);
//         }
//     };
// }
//
// fn button_callsite() -> impl Ui {
//     button! {
//         padding: 5.;
//         on_click: |_|{};
//         => text("Click Me!")
//     };
//
//     let child = text("Click Me!");
//     let child = margin::build(child, 5.);
//     let child = align(child, CENTER);
//     let child = background_color(rgb(0, 0, 0));
//
//     let child = button(child, |_|{});
//
//     let child = border(child,  4., (Var::clone(&text_border), BorderStyle::Dashed));
//
//     ui_item(child)
// }
//
// /// See [macro definition](button!)
// pub fn button(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static) -> impl Ui {
//     let on_click = Rc::new(RefCell::new(on_click));
//     ui! {
//         focusable: default;
//         on_click: enclose! ((on_click) move |ci, n|{
//             if ci.button == MouseButton::Left {
//                 (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
//             }
//         });
//         on_key_tap: move |kt, n|{
//             if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
//                 (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
//             }
//         };
//         => child
//     }
// }

ui_widget! {
    //! Button widget.
    //! # Arguments
    //! * `on_click`: Required button click event handler.
    //! * `padding`: Margin around the button content.
    //! * `background_color`:
    //! * `border`:

    use $crate::primitive::*;

     // Properties applied to child before calling widget fn.
     properties(child) {
         // Property declaration without default value, if not set does not apply.
         // If set applies margin to child.
         padding -> margin;
         // Same with default value.
         content_align: CENTER -> align;
         // Default value of background_color property that is applied to child.
         background_color: rgb(0, 0, 0);
     },


     // Properties applied to return of widget fn. Same sintax as
     // child_properties.
     properties(self) {
         border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
     },

     on(hover) {
        background_color: rgb(10, 10, 10);
        border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
     }

     on(pressed) {

     }

     on(disabled) {

     }

     // widget signature, must name the parameters after child,
     // they behave like required properties in the declared button! macro.

     pub fn button(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static, state: State) -> impl Ui {
         let on_click = Rc::new(RefCell::new(on_click));
         ui! {
             focusable: default;
             on_mouse_enter: |_, _| {
                state.hover(true);
             };
             on_mouse_leave: |_, _| {
                state.hover(false);
             };
             on_mouse_down: |_, _| {
                state.pressed(true);
             };
             on_mouse_up: |_, _| {
                state.pressed(false);
             };
             on_click: enclose! ((on_click) move |ci, n|{
                 if ci.button == MouseButton::Left {
                     (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
                 }
             });
             on_key_tap: move |kt, n|{
                 if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
                     (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
                 }
             };
             => child
         }
     }
}

// OR

/// Button widget.
/// # Arguments
/// * `on_click`: Required button click event handler.
/// * `padding`: Margin around the button content.
/// * `background_color`:
/// * `border`:
#[ui_widget]
#[child(padding -> margin)]
#[child(content_align: CENTER -> align)]
#[child(background_color: rgb(0, 0, 0))]
#[self(border: 4., (Var::clone(&text_border), BorderStyle::Dashed))]
pub fn button(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static) -> impl Ui {
    let on_click = Rc::new(RefCell::new(on_click));
    ui! {
        focusable: default;
        on_click: enclose! ((on_click) move |ci, n|{
            if ci.button == MouseButton::Left {
                (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
            }
        });
        on_key_tap: move |kt, n|{
            if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
                (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
            }
        };
        => child
    }
}

// OR
#[ui_widget]
#[child_properties {
    padding -> margin;
    content_align: CENTER -> align;
    background_color: rgb(0, 0, 0);
}]
#[self_properties {
    border: border: 4., (Var::clone(&text_border), BorderStyle::Dashed)
}]
pub fn button2(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static) -> impl Ui {
    let on_click = Rc::new(RefCell::new(on_click));
    ui! {
        focusable: default;
        on_click: enclose! ((on_click) move |ci, n|{
            if ci.button == MouseButton::Left {
                (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
            }
        });
        on_key_tap: move |kt, n|{
            if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
                (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
            }
        };
        => child
    }
}
*/
