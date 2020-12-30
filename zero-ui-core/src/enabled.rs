use crate::context::{state_key, LazyStateMap};
use crate::render::WidgetInfo;
use crate::var::{context_var, Vars};
use crate::Widget;

state_key! {
    struct EnabledState: bool;
}

context_var! {
    /// Don't use this directly unless you read all the enabled related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct IsEnabledVar: bool = return &true;
}

/// Extension method for accessing the [`enabled`] state of widgets.
pub trait WidgetEnabledExt {
    /// Gets the widget enabled state.
    ///
    /// The implementation for [`LazyStateMap`] and [`Widget`] only get the state configured
    /// in the widget, if a parent widget is disabled that does not show here. Use [`IsEnabled`]
    /// to get the inherited state from inside a widget.
    ///
    /// The implementation for [`WidgetInfo`] gets if the widget and all ancestors are enabled.
    fn enabled(&self) -> bool;
}
impl WidgetEnabledExt for LazyStateMap {
    fn enabled(&self) -> bool {
        self.get(EnabledState).copied().unwrap_or(true)
    }
}
impl<W: Widget> WidgetEnabledExt for W {
    fn enabled(&self) -> bool {
        self.state().enabled()
    }
}
impl<'a> WidgetEnabledExt for WidgetInfo<'a> {
    fn enabled(&self) -> bool {
        self.meta().enabled() && self.parent().map(|p| p.enabled()).unwrap_or(true)
    }
}

/// Contextual [`enabled`] accessor.
pub struct IsEnabled;
impl IsEnabled {
    /// Gets the enabled state in the current `vars` context.
    pub fn get(vars: &Vars) -> bool {
        *IsEnabledVar::var().get(vars)
    }
}
