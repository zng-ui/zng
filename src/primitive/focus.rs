use super::event::STOP_KEY_DOWN;
use crate::core::*;

#[derive(new)]
pub struct Focusable<C: Ui> {
    child: C,
    tab_index: u32,
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
    pub fn from_config(child: C, config: FocusableConfig) -> Self {
        Focusable {
            child,
            tab_index: config.tab_index,
            key: config.key.unwrap_or_else(FocusKey::new_unique),
            focused: false,
        }
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_focusable(self.tab_index, self.key, &LayoutRect::from_size(f.final_size()));
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
    fn focusable(self, config: impl FnOnce(FocusableConfig) -> FocusableConfig) -> Focusable<Self> {
        let c = config(FocusableConfig::new());
        Focusable::from_config(self, c)
    }
}
impl<T: Ui> FocusableExt for T {}

#[derive(new)]
pub struct FocusableConfig {
    #[new(value = "u32::max_value()")]
    tab_index: u32,
    #[new(default)]
    key: Option<FocusKey>,
}

impl UiConfig for FocusableConfig {}

impl FocusableConfig {
    /// Optionally set a custom tab navigation order inside the parent scope. The smallest index is navigated to first,
    /// equal indexes are ordered by their order of creation. The default value for the tab_index is u32::max_value().
    pub fn tab_index(mut self, tab_index: u32) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn key(mut self, key: FocusKey) -> Self {
        self.key = Some(key);
        self
    }
}

pub struct FocusScope<C: Ui> {
    child: C,

    focused: bool,

    tab_index: u32,
    key: FocusKey,
    skip: bool,
    tab: Option<TabNav>,
    directional: Option<DirectionalNav>,

    alt: bool,
    return_focus: Option<FocusKey>,

    remember_focus: bool,
    remembered_focus: Option<FocusKey>,
}

impl<C: Ui> FocusScope<C> {
    pub fn new(child: C, config: FocusScopeConfig) -> Self {
        let FocusScopeConfig {
            tab_index,
            key,
            skip,
            tab,
            directional,
            alt,
            remember_focus,
        } = config;

        FocusScope {
            child,

            focused: false,

            tab_index,
            key: key.unwrap_or_else(FocusKey::new_unique),
            skip,
            tab,
            directional,

            alt,
            return_focus: None,

            remember_focus,
            remembered_focus: None,
        }
    }
}

#[impl_ui_crate(child)]
impl<C: Ui> FocusScope<C> {
    #[Ui]
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

    #[Ui]
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
                    self.return_focus(values, update);
                }
                _ => {}
            }
        }
    }

    #[Ui]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.window_focused(focused, values, update);

        if !focused {
            self.return_focus(values, update);
        }
    }

    fn return_focus(&self, values: &mut UiValues, update: &mut NextUpdate) {
        if self.alt && self.focus_status().is_some() {
            update.focus(FocusRequest::Direct(
                self.return_focus.unwrap_or_else(|| values.window_focus_key()),
            ));
        }
    }

    #[Ui]
    fn focus_status(&self) -> Option<FocusStatus> {
        focusable_status(self.focused, &self.child)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_focus_scope(
            self.tab_index,
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
    fn focus_scope(self, config: impl FnOnce(FocusScopeConfig) -> FocusScopeConfig) -> FocusScope<Self> {
        let c = config(FocusScopeConfig::new());
        FocusScope::new(self, c)
    }
}
impl<T: Ui> FocusScopeExt for T {}

#[derive(new)]
pub struct FocusScopeConfig {
    #[new(value = "u32::max_value()")]
    tab_index: u32,
    #[new(default)]
    key: Option<FocusKey>,
    #[new(default)]
    skip: bool,

    #[new(value = "Some(TabNav::Continue)")]
    tab: Option<TabNav>,
    #[new(default)]
    directional: Option<DirectionalNav>,

    #[new(default)]
    alt: bool,
    #[new(default)]
    remember_focus: bool,
}

impl UiConfig for FocusScopeConfig {}

impl FocusScopeConfig {
    /// Optionally set a custom tab navigation order inside the parent scope. The smallest index is navigated to first,
    /// equal indexes are ordered by their order of creation. The default value for the tab_index is u32::max_value().
    pub fn tab_index(mut self, tab_index: u32) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn key(mut self, key: FocusKey) -> Self {
        self.key = Some(key);
        self
    }

    /// Optionally navigation does not move into this scope automatically, but automatic navigation within it still works.
    pub fn skip(mut self, skip: bool) -> Self {
        self.skip = skip;
        self
    }

    /// Optional automatic tab navigation inside this scope.
    pub fn tab_nav(mut self, tab: Option<TabNav>) -> Self {
        self.tab = tab;
        self
    }

    /// Optional automatic arrow keys navigation inside this scope.
    pub fn directional_nav(mut self, directional: Option<DirectionalNav>) -> Self {
        self.directional = directional;
        self
    }

    /// Optionally automatically focus in this scope when alt is pressed in
    ///  window, returning focus on previous location when esc is pressed.
    pub fn alt_nav(mut self, alt: bool) -> Self {
        self.alt = alt;
        self
    }

    ///Optionally remember the last focused location inside this scope and
    /// restore it when the scope is refocused again.
    pub fn remember_focus(mut self, remember_focus: bool) -> Self {
        self.remember_focus = remember_focus;
        self
    }

    pub fn tab_nav_cycle(self) -> Self {
        self.tab_nav(Some(TabNav::Cycle))
    }

    pub fn tab_nav_contained(self) -> Self {
        self.tab_nav(Some(TabNav::Contained))
    }

    pub fn tab_nav_continue(self) -> Self {
        self.tab_nav(Some(TabNav::Continue))
    }

    pub fn tab_nav_once(self) -> Self {
        self.tab_nav(Some(TabNav::Once))
    }

    pub fn directional_nav_cycle(self) -> Self {
        self.directional_nav(Some(DirectionalNav::Cycle))
    }

    pub fn directional_nav_contained(self) -> Self {
        self.directional_nav(Some(DirectionalNav::Contained))
    }

    pub fn directional_nav_continue(self) -> Self {
        self.directional_nav(Some(DirectionalNav::Continue))
    }

    pub fn no_tab_nav(self) -> Self {
        self.tab_nav(None)
    }

    pub fn menu(self) -> Self {
        self.skip(true).alt_nav(true).tab_nav_cycle().directional_nav_cycle()
    }
}
