use super::{
    ElementState, Hits, KeyboardInput, LayoutPoint, ModifiersState, MouseButton, MouseInput, MouseMove, NextUpdate, Ui,
    UiContainer, VirtualKeyCode,
};
use std::fmt;
use std::time::{Duration, Instant};

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

pub struct OnClick<T: Ui, F: FnMut(ClickInput, &mut NextUpdate)> {
    child: T,
    handler: F,
    click_count: u8,
    last_pressed: Instant,
}

impl<T: Ui, F: FnMut(ClickInput, &mut NextUpdate)> OnClick<T, F> {
    pub fn new(child: T, handler: F) -> Self {
        OnClick {
            child,
            handler,
            click_count: 0,
            last_pressed: Instant::now() - Duration::from_secs(30),
        }
    }

    fn call_handler(&mut self, input: &MouseInput, position: LayoutPoint, update: &mut NextUpdate) {
        let input = ClickInput {
            button: input.button,
            modifiers: input.modifiers,
            position,
            click_count: self.click_count,
        };
        (self.handler)(input, update);
    }

    fn interaction_outside(&mut self) {
        self.click_count = 0;
        self.last_pressed -= Duration::from_secs(30);
    }
}

#[cfg(target_os = "windows")]
fn multi_click_time_ms() -> Duration {
    Duration::from_millis(unsafe { winapi::um::winuser::GetDoubleClickTime() } as u64)
}

#[cfg(not(target_os = "windows"))]
fn multi_click_time_ms() -> u32 {
    // https://stackoverflow.com/questions/50868129/how-to-get-double-click-time-interval-value-programmatically-on-linux
    // https://developer.apple.com/documentation/appkit/nsevent/1532495-mouseevent
    Duration::from_millis(500)
}

impl<T: Ui + 'static, F: FnMut(ClickInput, &mut NextUpdate)> UiContainer for OnClick<T, F> {
    delegate_child!(child, T);

    fn focused(&mut self, _: bool, _: &mut NextUpdate) {
        self.interaction_outside();
    }

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
        Ui::mouse_input(&mut self.child, input, hits, update);

        match input.state {
            ElementState::Pressed => {
                if let Some(position) = self.child.point_over(hits) {
                    self.click_count = self.click_count.saturating_add(1);

                    let now = Instant::now();

                    if self.click_count > 1 {
                        if (now - self.last_pressed) < multi_click_time_ms() {
                            self.call_handler(input, position, update);
                        } else {
                            self.click_count = 1;
                        }
                    }
                    self.last_pressed = now;
                } else {
                    self.interaction_outside();
                }
            }
            ElementState::Released => {
                if self.click_count > 0 {
                    if let Some(position) = self.child.point_over(hits) {
                        if self.click_count == 1 {
                            self.call_handler(input, position, update);
                        }
                    } else {
                        self.interaction_outside();
                    }
                }
            }
        }
    }
}

impl<T: Ui + 'static, F: FnMut(ClickInput, &mut NextUpdate)> Ui for OnClick<T, F> {
    delegate_ui_methods!(UiContainer);
}

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

    fn on_click<F: FnMut(ClickInput, &mut NextUpdate)>(self, handler: F) -> OnClick<Self, F> {
        OnClick::new(self, handler)
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

#[derive(Debug)]
pub struct ClickInput {
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
    pub click_count: u8,
}

impl fmt::Display for MouseButtonInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(&self.modifiers, f)?;
        write!(f, "{:?} {}", self.button, self.position)
    }
}

impl fmt::Display for ClickInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(&self.modifiers, f)?;
        write!(f, "{:?} {}", self.button, self.position)?;
        match self.click_count {
            0..=1 => {}
            2 => write!(f, " double-click")?,
            3 => write!(f, " triple-click")?,
            n => write!(f, " click_count={}", n)?,
        }
        Ok(())
    }
}
