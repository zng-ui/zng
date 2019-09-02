use super::{
    ElementState, Hits, KeyboardInput, LayoutPoint, ModifiersState, MouseButton, MouseInput, MouseMove, NextUpdate, Ui,
    UiContainer, VirtualKeyCode,
};
use std::fmt;

pub struct OnKeyDown<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> {
    child: T,
    handler: F,
}

impl<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> OnKeyDown<T, F> {
    pub fn new(child: T, handler: F) -> Self {
        OnKeyDown { child, handler }
    }
}

impl<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> UiContainer for OnKeyDown<T, F> {
    delegate_child!(child, T);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.child.keyboard_input(input, update);

        if let (ElementState::Pressed, Some(key)) = (input.state, input.virtual_keycode) {
            let input = KeyDown {
                key,
                modifiers: input.modifiers,
                repeat: input.repeat,
            };
            (self.handler)(input, update);
        }
    }
}

impl<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> Ui for OnKeyDown<T, F> {
    delegate_ui_methods!(UiContainer);
}

pub struct OnKeyUp<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> {
    child: T,
    handler: F,
}

impl<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> OnKeyUp<T, F> {
    pub fn new(child: T, handler: F) -> Self {
        OnKeyUp { child, handler }
    }
}

impl<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> UiContainer for OnKeyUp<T, F> {
    delegate_child!(child, T);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.child.keyboard_input(input, update);

        if let (ElementState::Released, Some(key)) = (input.state, input.virtual_keycode) {
            let input = KeyUp {
                key,
                modifiers: input.modifiers,
            };
            (self.handler)(input, update);
        }
    }
}

impl<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> Ui for OnKeyUp<T, F> {
    delegate_ui_methods!(UiContainer);
}

pub trait KeyboardEvents: Ui + Sized {
    fn on_key_down<F: FnMut(KeyDown, &mut NextUpdate)>(self, handler: F) -> OnKeyDown<Self, F> {
        OnKeyDown::new(self, handler)
    }

    fn on_key_up<F: FnMut(KeyUp, &mut NextUpdate)>(self, handler: F) -> OnKeyUp<Self, F> {
        OnKeyUp::new(self, handler)
    }
}
impl<T: Ui + Sized> KeyboardEvents for T {}

macro_rules! on_mouse {
    ($state: ident, $name: ident) => {
        #[derive(Clone)]
        pub struct $name<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> {
            child: T,
            handler: F,
        }

        impl<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> $name<T, F> {
            pub fn new(child: T, handler: F) -> Self {
                $name { child, handler }
            }
        }

        impl<T: Ui + 'static, F: FnMut(MouseButtonInput, &mut NextUpdate)> UiContainer for $name<T, F> {
            delegate_child!(child, T);

            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
                Ui::mouse_input(&mut self.child, input, hits, update);

                if let Some(position) = self.child.point_over(hits) {
                    if let ElementState::$state = input.state {
                        let input = MouseButtonInput {
                            button: input.button,
                            modifiers: input.modifiers,
                            position,
                        };
                        (self.handler)(input, update);
                    }
                }
            }
        }

        impl<T: Ui + 'static, F: FnMut(MouseButtonInput, &mut NextUpdate)> Ui for $name<T, F> {
            delegate_ui_methods!(UiContainer);
        }
    };
}

on_mouse!(Pressed, OnMouseDown);
on_mouse!(Released, OnMouseUp);

#[derive(Clone)]
pub struct OnMouseMove<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> {
    child: T,
    handler: F,
}

impl<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> OnMouseMove<T, F> {
    pub fn new(child: T, handler: F) -> Self {
        OnMouseMove { child, handler }
    }
}

impl<T: Ui + 'static, F: FnMut(MouseMove, &mut NextUpdate)> UiContainer for OnMouseMove<T, F> {
    delegate_child!(child, T);

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
        Ui::mouse_move(self, input, hits, update);
        if let Some(position) = self.child.point_over(hits) {
            (self.handler)(
                MouseMove {
                    position,
                    modifiers: input.modifiers,
                },
                update,
            )
        }
    }
}
impl<T: Ui + 'static, F: FnMut(MouseMove, &mut NextUpdate)> Ui for OnMouseMove<T, F> {
    delegate_ui_methods!(UiContainer);
}

macro_rules! on_mouse_enter_leave {
    ($Type: ident, $mouse_over: ident, $if_mouse_over: expr) => {
        pub struct $Type<T: Ui, F: FnMut(&mut NextUpdate)> {
            child: T,
            handler: F,
            mouse_over: bool,
        }

        impl<T: Ui, F: FnMut(&mut NextUpdate)> $Type<T, F> {
            pub fn new(child: T, handler: F) -> Self {
                $Type {
                    child,
                    handler,
                    mouse_over: false,
                }
            }

            fn set_mouse_over(&mut self, $mouse_over: bool, update: &mut NextUpdate) {
                if self.mouse_over != $mouse_over {
                    self.mouse_over = $mouse_over;
                    if $if_mouse_over {
                        (self.handler)(update);
                    }
                }
            }
        }

        impl<T: Ui + 'static, F: FnMut(&mut NextUpdate)> UiContainer for $Type<T, F> {
            delegate_child!(child, T);

            fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
                Ui::mouse_move(&mut self.child, input, hits, update);
                self.set_mouse_over(self.child.point_over(hits).is_some(), update);
            }

            fn mouse_left(&mut self, update: &mut NextUpdate) {
                Ui::mouse_left(&mut self.child, update);
                self.set_mouse_over(false, update);
            }
        }
        impl<T: Ui + 'static, F: FnMut(&mut NextUpdate)> Ui for $Type<T, F> {
            delegate_ui_methods!(UiContainer);
        }
    };
}
on_mouse_enter_leave!(OnMouseEnter, mouse_over, mouse_over);
on_mouse_enter_leave!(OnMouseLeave, mouse_over, !mouse_over);

pub trait MouseEvents: Ui + Sized {
    fn on_mouse_down<F: FnMut(MouseButtonInput, &mut NextUpdate)>(self, handler: F) -> OnMouseDown<Self, F> {
        OnMouseDown::new(self, handler)
    }

    fn on_mouse_up<F: FnMut(MouseButtonInput, &mut NextUpdate)>(self, handler: F) -> OnMouseUp<Self, F> {
        OnMouseUp::new(self, handler)
    }

    fn on_mouse_move<F: FnMut(MouseMove, &mut NextUpdate)>(self, handler: F) -> OnMouseMove<Self, F> {
        OnMouseMove::new(self, handler)
    }

    fn on_mouse_enter<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnMouseEnter<Self, F> {
        OnMouseEnter::new(self, handler)
    }

    fn on_mouse_leave<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnMouseLeave<Self, F> {
        OnMouseLeave::new(self, handler)
    }
}
impl<T: Ui + Sized> MouseEvents for T {}

#[derive(Debug)]
pub struct KeyDown {
    pub key: VirtualKeyCode,
    pub modifiers: ModifiersState,
    pub repeat: bool,
}

#[derive(Debug)]
pub struct KeyUp {
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

impl fmt::Display for KeyDown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.key {
            VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LShift
            | VirtualKeyCode::RShift
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt => write!(f, "{:?}", self.key)?,
            _ => {
                display_modifiers(&self.modifiers, f)?;
                write!(f, "{:?}", self.key)?;
            }
        }
        if self.repeat {
            write!(f, " (repeat)")?;
        }
        Ok(())
    }
}

impl fmt::Display for KeyUp {
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
    pub position: LayoutPoint,
}

impl fmt::Display for MouseButtonInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(&self.modifiers, f)?;
        write!(f, "{:?} {}", self.button, self.position)
    }
}
