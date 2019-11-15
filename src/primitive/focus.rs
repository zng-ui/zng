use super::event::STOP_KEY_DOWN;
use crate::core::*;

#[derive(new)]
pub struct FocusOnInit<C: Ui> {
    child: Focusable<C>,
    request_focus: bool,
}

#[impl_ui_crate(child)]
impl<C: Ui> Ui for FocusOnInit<C> {
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.init(values, update);
        if self.request_focus {
            update.focus(FocusRequest::Direct(self.child.key));
        }
    }
}

#[derive(new)]
pub struct Focusable<C: Ui> {
    child: C,
    key: FocusKey,
    #[new(default)]
    focused: bool,
}

fn focusable_status(focused: bool, child: &impl Ui) -> Option<FocusStatus> {
    if focused {
        Some(FocusStatus::Focused)
    } else {
        match child.focus_status() {
            None => None,
            _ => Some(FocusStatus::FocusWithin),
        }
    }
}

#[impl_ui_crate(child)]
impl<C: Ui> Focusable<C> {
    pub fn focused(self, request_focus: bool) -> FocusOnInit<C> {
        FocusOnInit::new(self, request_focus)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_focusable(self.key, &LayoutRect::from_size(f.final_size()));
        self.child.render(f);
    }

    #[Ui]
    fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.focus_changed(change, values, update);

        self.focused = Some(self.key) == change.new_focus;
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        if input.state == ElementState::Pressed && self.child.point_over(hits).is_some() {
            update.focus(FocusRequest::Direct(self.key));
        }

        self.child.mouse_input(input, hits, values, update);
    }

    #[Ui]
    fn focus_status(&self) -> Option<FocusStatus> {
        focusable_status(self.focused, &self.child)
    }
}

pub trait FocusableExt: Ui + Sized {
    fn focusable(self) -> Focusable<Self> {
        Focusable::new(self, FocusKey::new_unique())
    }

    fn focusable_with_key(self, key: FocusKey) -> Focusable<Self> {
        Focusable::new(self, key)
    }
}
impl<T: Ui> FocusableExt for T {}

#[derive(new)]
pub struct FocusScope<C: Ui> {
    child: C,

    #[new(default)]
    focused: bool,

    key: FocusKey,
    #[new(default)]
    skip: bool,
    #[new(value = "Some(TabNav::Continue)")]
    tab: Option<TabNav>,
    #[new(default)]
    directional: Option<DirectionalNav>,

    #[new(default)]
    alt: bool,
    #[new(default)]
    return_focus: Option<FocusKey>,

    #[new(default)]
    remember_focus: bool,
    #[new(default)]
    remembered_focus: Option<FocusKey>,
}

impl<C: Ui> FocusScope<C> {
    pub(crate) fn key(&self) -> FocusKey {
        self.key
    }

    // Optionally navigation does not move into this scope automatically, but automatic navigation within it still works.
    pub fn with_skip(mut self, skip: bool) -> Self {
        self.skip = skip;
        self
    }

    /// Optional automatic tab navigation inside this scope.
    pub fn with_tab_nav(mut self, tab: Option<TabNav>) -> Self {
        self.tab = tab;
        self
    }

    /// Optional automatic arrow keys navigation inside this scope.
    pub fn with_directional_nav(mut self, directional: Option<DirectionalNav>) -> Self {
        self.directional = directional;
        self
    }

    /// Optionally automatically focus in this scope when alt is pressed in window, returning focus on previous location when esc is pressed.
    pub fn with_alt_nav(mut self, alt: bool) -> Self {
        self.alt = alt;
        self
    }

    ///Optionally remember the last focused location inside this scope and restore it when the scope is refocused again.
    pub fn with_remember(mut self, remember: bool) -> Self {
        self.remember_focus = remember;
        self
    }
}

#[impl_ui_crate(child)]
impl<C: Ui> Ui for FocusScope<C> {
    fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
        let was_focused = self.focus_status().is_some();

        self.child.focus_changed(change, values, update);

        let is_focused = if change.new_focus == Some(self.key) {
            update.focus(if self.remember_focus {
                self.remembered_focus
                    .map(FocusRequest::Direct)
                    .unwrap_or(FocusRequest::Next)
            } else {
                FocusRequest::Next
            });

            self.focused = true;
            true
        } else {
            self.focused = false;
            self.focus_status().is_some()
        };

        if self.alt && was_focused != is_focused {
            if is_focused {
                self.return_focus = change.old_focus;
            } else {
                self.return_focus = None;
            }
        }
        if self.remember_focus && is_focused {
            self.remembered_focus = change.new_focus;
        }
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.keyboard_input(input, values, update);

        if !self.alt && values.child(*STOP_KEY_DOWN).is_some() {
            return;
        }

        if let (ElementState::Pressed, Some(key)) = (input.state, input.virtual_keycode) {
            match key {
                VirtualKeyCode::LAlt => {
                    if self.focus_status().is_none() {
                        update.focus(FocusRequest::Direct(self.key));
                    }
                }
                VirtualKeyCode::Escape => {
                    if let (Some(return_focus), Some(_)) = (self.return_focus, self.focus_status()) {
                        update.focus(FocusRequest::Direct(return_focus))
                    }
                }
                _ => {}
            }
        }
    }

    fn focus_status(&self) -> Option<FocusStatus> {
        focusable_status(self.focused, &self.child)
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_focus_scope(
            self.key,
            &LayoutRect::from_size(f.final_size()),
            self.skip,
            self.tab,
            self.directional,
            &self.child,
        );
    }
}

pub trait FocusScopeExt: Ui + Sized {
    ///Creates a default FocusScope
    fn focus_scope(self) -> FocusScope<Self> {
        FocusScope::new(self, FocusKey::new_unique())
    }

    ///Creates a default FocusScope with a specific FocusKey
    fn focus_scope_with_key(self, key: FocusKey) -> FocusScope<Self> {
        FocusScope::new(self, key)
    }
}
impl<T: Ui> FocusScopeExt for T {}
