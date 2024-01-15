//! Toggle button widget and styles for check box, combo box, radio button and switch button.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_toggle`] for the full widget API.

pub use zero_ui_wgt_toggle::{
    check_spacing, combo_spacing, deselect_on_deinit, deselect_on_new, is_checked, radio_spacing, scroll_on_select, select_on_init,
    select_on_new, selector, style_fn, switch_spacing, tristate, CheckStyle, ComboStyle, DefaultStyle, RadioStyle, Selector, SelectorError,
    SelectorImpl, SwitchStyle, Toggle, IS_CHECKED_VAR,
};

/// Toggle commands.
pub mod cmd {
    pub use zero_ui_wgt_toggle::cmd::{SelectOp, SELECT_CMD, TOGGLE_CMD};
}
