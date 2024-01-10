//! Menu widgets, properties and types.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_menu`] for the full widget API.

pub use zero_ui_wgt_menu::{
    extend_style, icon, icon_fn, panel_fn, replace_style, shortcut_spacing, shortcut_txt, ButtonStyle, CmdButton, DefaultStyle, Menu,
    ToggleStyle, TouchCmdButton,
};

/// Context menu widget and properties.
///
/// See [`zero_ui_wgt_menu::context`] for the full widget API.
pub mod context {
    pub use zero_ui_wgt_menu::context::{
        context_menu, context_menu_anchor, context_menu_fn, disabled_context_menu, disabled_context_menu_fn, extend_style, panel_fn,
        replace_style, ContextMenu, ContextMenuArgs, DefaultStyle, TouchStyle,
    };
}

/// Sub-menu popup widget and properties.
///
/// See [`zero_ui_wgt_menu::popup`] for the full widget API.
pub mod popup {
    pub use zero_ui_wgt_menu::popup::{extend_style, panel_fn, replace_style, DefaultStyle, SubMenuPopup};
}
