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

#[ui_widget]
/// Button widget.
/// # Arguments
/// * `on_click`: Button click event handler.
/// * `child`: Button content.
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
