use crate::core::*;

#[derive(new)]
pub struct Focusable<C: Ui> {
    child: C,
    focused: bool,
}
#[impl_ui_crate(child)]
impl<C: Ui> Focusable<C> {
    #[Ui]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.window_focused(focused, values, update);
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.mouse_input(input, hits, values, update);

        if input.state == ElementState::Pressed {
            self.focused = self.child.focus_status().is_none() && self.point_over(hits).is_some();
        }
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
        Focusable::new(self, false)
    }
}
impl<T: Ui> FocusableExt for T {}

#[derive(new)]
pub struct FocusScope<C: Ui> {
    child: C,
    key: FocusKey,
    navigation: KeyNavigation,
    capture: bool,
}
#[impl_ui_crate(child)]
impl<C: Ui> Ui for FocusScope<C> {
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
        FocusScope::new(self, FocusKey::new(), navigation, capture)
    }
}
impl<T: Ui> FocusScopeExt for T {}
