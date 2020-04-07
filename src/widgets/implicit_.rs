use crate::prelude::WidgetId;
use crate::widget_mixin;

#[doc(hidden)]
pub use crate::properties::id;

widget_mixin! {
    /// Mix-in inherited implicitly by all [widgets](../../../zero_ui/macro.widget.html).
    pub implicit_mixin;

    default {
        /// Unique identifier of the widget.
        /// Set to [`WidgetId::new_unique()`](WidgetId::new_unique()) by default.
        id: WidgetId::new_unique();
    }
}
