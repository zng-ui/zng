use super::{ElementState, KeyboardInput, ModifiersState, NextUpdate, Ui, VirtualKeyCode};
use std::fmt;

#[derive(Debug)]
pub struct KeyInput {
    pub key: VirtualKeyCode,
    pub modifiers: ModifiersState,
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
                if self.modifiers.ctrl {
                    write!(f, "Ctrl + ")?;
                }
                if self.modifiers.alt {
                    write!(f, "Alt + ")?;
                }
                if self.modifiers.shift {
                    write!(f, "Shift + ")?;
                }
                if self.modifiers.logo {
                    write!(f, "Logo + ")?;
                }
                write!(f, "{:?}", self.key)
            }
        }
    }
}

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

        impl<T: Ui, F: FnMut(KeyInput, &mut NextUpdate)> Ui for $name<T, F> {
            type Child = T;

            fn for_each_child(&mut self, mut action: impl FnMut(&mut Self::Child)) {
                action(&mut self.child)
            }

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

        pub trait $ext_name: Ui + Sized {
            fn $ext_fn<F: FnMut(KeyInput, &mut NextUpdate)>(self, handler: F) -> $name<Self, F> {
                $name::new(self, handler)
            }
        }

        impl<T: Ui + Sized> $ext_name for T {}
    };
}

on_key!(Pressed, OnKeyDown, OnKeyDownExt, on_keydown);
on_key!(Released, OnKeyUp, OnKeyUpExt, on_keyup);
