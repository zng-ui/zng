//! Menu widgets, properties and types.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_menu`] for the full widget API.

pub use zero_ui_wgt_menu::{
    icon, icon_fn, panel_fn, shortcut_spacing, shortcut_txt, style_fn, ButtonStyle, DefaultStyle, Menu, ToggleStyle, TouchButtonStyle,
};

/// Context menu widget and properties.
///
/// See [`zero_ui_wgt_menu::context`] for the full widget API.
pub mod context {
    pub use zero_ui_wgt_menu::context::{
        context_menu, context_menu_anchor, context_menu_fn, disabled_context_menu, disabled_context_menu_fn, panel_fn, style_fn,
        ContextMenu, ContextMenuArgs, DefaultStyle, TouchStyle,
    };
}

/// Sub-menu popup widget and properties.
///
/// See [`zero_ui_wgt_menu::popup`] for the full widget API.
pub mod popup {
    pub use zero_ui_wgt_menu::popup::{panel_fn, style_fn, DefaultStyle, SubMenuPopup};
}
