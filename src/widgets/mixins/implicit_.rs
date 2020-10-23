use crate::core::{widget_mixin, WidgetId};
use crate::properties::capture_only::widget_id;

widget_mixin! {
    /// Mix-in inherited implicitly by all [widgets](../../../zero_ui/macro.widget.html).
    pub implicit_mixin;

    default {
        /// Unique identifier of the widget.
        /// Set to [`WidgetId::new_unique()`](WidgetId::new_unique()) by default.
        id -> widget_id: WidgetId::new_unique();
    }
}
