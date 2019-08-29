use super::{
    ElementState, KeyboardInput, ModifiersState, MouseButton, MouseInput, MouseMove, NextUpdate, Ui, UiContainer,
    VirtualKeyCode,
};
use std::fmt;

macro_rules! on_key {
    ($state: ident, $name: ident, $ext_name: ident, $ext_fn: ident) => {
        pub struct $name<T: Ui, F: FnMut(KeyInput, &mut NextUpdate)> {
            child: T,
            handler: F,
        }

        impl<T: Ui, F: FnMut(KeyInput, &mut NextUpdate)> $name<T, F> {
            pub fn new(child: T, handler: F) -> Self {
                $name { child, handler }
            }
        }

        impl<T: Ui, F: FnMut(KeyInput, &mut NextUpdate)> UiContainer for $name<T, F> {
            delegate_child!(child, T);

            fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
                self.child.keyboard_input(input, update);

                if let (ElementState::$state, Some(key)) = (input.state, input.virtual_keycode) {
                    let input = KeyInput {
                        key,
                        modifiers: input.modifiers,
                    };
                    (self.handler)(input, update);
                }
            }
        }

        impl<T: Ui, F: FnMut(KeyInput, &mut NextUpdate)> Ui for $name<T, F> {
            delegate_ui_methods!(UiContainer);
        }

        pub trait $ext_name: Ui + Sized {
            fn $ext_fn<F: FnMut(KeyInput, &mut NextUpdate)>(self, handler: F) -> $name<Self, F> {
                $name::new(self, handler)
            }
        }

        impl<T: Ui + Sized> $ext_name for T {}
    };
}

on_key!(Pressed, OnKeyDown, OnKeyDownExt, on_key_down);
on_key!(Released, OnKeyUp, OnKeyUpExt, on_key_up);

macro_rules! on_mouse {
    ($state: ident, $name: ident, $ext_name: ident, $ext_fn: ident) => {
        pub struct $name<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> {
            child: T,
            handler: F,
        }

        impl<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> $name<T, F> {
            pub fn new(child: T, handler: F) -> Self {
                $name { child, handler }
            }
        }

        impl<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> UiContainer for $name<T, F> {
            delegate_child!(child, T);

            fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
                self.child.mouse_input(input, update);

                if let ElementState::$state = input.state {
                    let input = MouseButtonInput {
                        button: input.button,
                        modifiers: input.modifiers,
                    };
                    (self.handler)(input, update);
                }
            }
        }

        impl<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> Ui for $name<T, F> {
            delegate_ui_methods!(UiContainer);
        }

        pub trait $ext_name: Ui + Sized {
            fn $ext_fn<F: FnMut(MouseButtonInput, &mut NextUpdate)>(self, handler: F) -> $name<Self, F> {
                $name::new(self, handler)
            }
        }

        impl<T: Ui + Sized> $ext_name for T {}
    };
}

on_mouse!(Pressed, OnMouseDown, OnMouseDownExt, on_mouse_down);
on_mouse!(Released, OnMouseUp, OnMouseUpExt, on_mouse_up);

pub struct OnMouseMove<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> {
    child: T,
    handler: F,
}

impl<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> OnMouseMove<T, F> {
    pub fn new(child: T, handler: F) -> Self {
        OnMouseMove { child, handler }
    }
}

impl<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> UiContainer for OnMouseMove<T, F> {
    delegate_child!(child, T);

    fn mouse_move(&mut self, input: &MouseMove, update: &mut NextUpdate) {
        (self.handler)(
            MouseMove {
                position: input.position,
                modifiers: input.modifiers,
            },
            update,
        )
    }
}

impl<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> Ui for OnMouseMove<T, F> {
    delegate_ui_methods!(UiContainer);
}

pub trait OnMouseMoveExt: Ui + Sized {
    fn on_mouse_move<F: FnMut(MouseMove, &mut NextUpdate)>(self, handler: F) -> OnMouseMove<Self, F> {
        OnMouseMove::new(self, handler)
    }
}

impl<T: Ui + Sized> OnMouseMoveExt for T {}

#[derive(Debug)]
pub struct KeyInput {
    pub key: VirtualKeyCode,
    pub modifiers: ModifiersState,
}

fn display_modifiers(m: &ModifiersState, f: &mut fmt::Formatter) -> fmt::Result {
    if m.ctrl {
        write!(f, "Ctrl + ")?;
    }
    if m.alt {
        write!(f, "Alt + ")?;
    }
    if m.shift {
        write!(f, "Shift + ")?;
    }
    if m.logo {
        write!(f, "Logo + ")?;
    }

    Ok(())
}

impl fmt::Display for KeyInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.key {
            VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LShift
            | VirtualKeyCode::RShift
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt => write!(f, "{:?}", self.key),
            _ => {
                display_modifiers(&self.modifiers, f)?;
                write!(f, "{:?}", self.key)
            }
        }
    }
}

#[derive(Debug)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub modifiers: ModifiersState,
}

impl fmt::Display for MouseButtonInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(&self.modifiers, f)?;
        write!(f, "{:?}", self.button)
    }
}
