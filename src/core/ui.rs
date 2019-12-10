use super::{
    impl_ui_crate, FocusChange, FocusStatus, Hits, KeyboardInput, LayoutPoint, LayoutSize, MouseInput, NextFrame,
    NextUpdate, UiMouseMove, UiValues,
};

/// An UI component.
///
/// # Implementers
/// This is usually not implemented directly, consider using [impl_ui](attr.impl_ui.html) first.
pub trait Ui: 'static {
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    fn arrange(&mut self, final_size: LayoutSize);

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate);

    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate);

    fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn focus_status(&self) -> Option<FocusStatus>;

    /// Gets the point over this UI element using a hit test result.
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint>;

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Box this component, unless it is already `Box<dyn Ui>`.
    fn into_box(self) -> Box<dyn Ui>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

#[impl_ui_crate(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl Ui for Box<dyn Ui> {
    fn into_box(self) -> Box<dyn Ui> {
        self
    }
}

/// Marker trait for a Ui configuration. Enables [default] for
/// Ui optional configuration pattern.
pub trait UiConfig {}

/// Uses the default configuration.
pub fn default<C: UiConfig>(config: C) -> C {
    config
}

impl<U: Ui> Ui for Option<U> {
    #[inline]
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.init(values, update);
        }
    }

    #[inline]
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        match self.as_mut() {
            Some(inner) => inner.measure(available_size),
            None => LayoutSize::zero(),
        }
    }

    #[inline]
    fn arrange(&mut self, final_size: LayoutSize) {
        if let Some(inner) = self.as_mut() {
            inner.arrange(final_size);
        }
    }

    #[inline]
    fn render(&self, f: &mut NextFrame) {
        if let Some(inner) = self.as_ref() {
            inner.render(f);
        }
    }

    #[inline]
    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.keyboard_input(input, values, update);
        }
    }

    #[inline]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.window_focused(focused, values, update);
        }
    }

    #[inline]
    fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.focus_changed(change, values, update);
        }
    }

    #[inline]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.mouse_input(input, hits, values, update);
        }
    }

    #[inline]
    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.mouse_move(input, hits, values, update);
        }
    }

    #[inline]
    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.mouse_entered(values, update);
        }
    }

    #[inline]
    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.mouse_left(values, update);
        }
    }

    #[inline]
    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.close_request(values, update);
        }
    }

    #[inline]
    fn focus_status(&self) -> Option<FocusStatus> {
        match self.as_ref() {
            Some(inner) => inner.focus_status(),
            None => None,
        }
    }

    #[inline]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        match self.as_ref() {
            Some(inner) => inner.point_over(hits),
            None => None,
        }
    }

    #[inline]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.value_changed(values, update);
        }
    }

    #[inline]
    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if let Some(inner) = self.as_mut() {
            inner.parent_value_changed(values, update);
        }
    }
}
