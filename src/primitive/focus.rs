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
        if self.focused {
            Some(FocusStatus::Focused)
        } else {
            match self.child.focus_status() {
                None => None,
                _ => Some(FocusStatus::FocusWithin),
            }
        }
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
    key: FocusKey,
    navigation: KeyNavigation,
    capture: bool,
    #[new(default)]
    logical_focus: Option<FocusKey>,
}

impl<C: Ui> FocusScope<C> {
    pub(crate) fn key(&self) -> FocusKey {
        self.key
    }
}

#[impl_ui_crate(child)]
impl<C: Ui> Ui for FocusScope<C> {
    fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.focus_changed(change, values, update);

        if change.new_focus == Some(self.key) {
            if let (true, Some(logical_focus)) = (self.capture, self.logical_focus) {
                update.focus(FocusRequest::Direct(logical_focus));
            } else {
                update.focus(FocusRequest::Next);
            }
        } else if self.child.focus_status().is_some() {
            self.logical_focus = change.new_focus;
        }
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_focus_scope(
            self.key,
            &LayoutRect::from_size(f.final_size()),
            self.navigation,
            self.capture,
            &self.child,
        );
    }
}

pub trait FocusScopeExt: Ui + Sized {
    fn focus_scope(self, navigation: KeyNavigation, capture: bool) -> FocusScope<Self> {
        FocusScope::new(self, FocusKey::new_unique(), navigation, capture)
    }
}
impl<T: Ui> FocusScopeExt for T {}
