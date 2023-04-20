//! UI nodes used for building a window widget.

use crate::core::window::WINDOW_CTRL;
use crate::prelude::new_property::*;

use std::time::Duration;

/// Defines if a widget load affects the parent window load.
///
/// Widgets that support this behavior have a `block_window_load` property.
#[derive(Clone, Copy, Debug)]
pub enum BlockWindowLoad {
    /// Widget requests a [`WindowLoadingHandle`] and retains it until the widget is loaded.
    ///
    /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
    Enabled {
        /// Handle expiration deadline, if the widget takes longer than this deadline the window loads anyway.
        deadline: Deadline,
    },
    /// Widget does not hold back window load.
    Disabled,
}
impl BlockWindowLoad {
    /// Enabled value.
    pub fn enabled(deadline: impl Into<Deadline>) -> BlockWindowLoad {
        BlockWindowLoad::Enabled { deadline: deadline.into() }
    }

    /// Returns `true` if is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// Returns `true` if is disabled.
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Returns the block deadline if is enabled and the deadline has not expired.
    pub fn deadline(self) -> Option<Deadline> {
        match self {
            BlockWindowLoad::Enabled { deadline } => {
                if deadline.has_elapsed() {
                    None
                } else {
                    Some(deadline)
                }
            }
            BlockWindowLoad::Disabled => None,
        }
    }
}
impl_from_and_into_var! {
    /// Converts `true` to `BlockWindowLoad::enabled(1.secs())` and `false` to `BlockWindowLoad::Disabled`.
    fn from(enabled: bool) -> BlockWindowLoad {
        if enabled {
            BlockWindowLoad::enabled(1.secs())
        } else {
            BlockWindowLoad::Disabled
        }
    }

    /// Converts to enabled with the duration timeout.
    fn from(enabled_timeout: Duration) -> BlockWindowLoad {
        BlockWindowLoad::enabled(enabled_timeout)
    }
}

/// Node that binds the [`COLOR_SCHEME_VAR`] to the [`actual_color_scheme`].
///
/// [`actual_color_scheme`]: crate::core::window::WindowVars::actual_color_scheme
pub fn color_scheme(child: impl UiNode) -> impl UiNode {
    with_context_var_init(child, COLOR_SCHEME_VAR, || WINDOW_CTRL.vars().actual_color_scheme().boxed())
}

/// Wrap around the window outer-most event node to create the layers.
///
/// This node is included in the [`NestGroup::EVENT`] group.
///
/// [`NestGroup::EVENT`]: crate::core::widget_builder::NestGroup::EVENT
pub fn layers(child: impl UiNode) -> impl UiNode {
    super::layers::node(child)
}
