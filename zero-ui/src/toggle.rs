//! Toggle button widget and styles for check box, combo box, radio button and switch button.
//!
//! # Combo
//!
//! !!: TODO
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let txt = var(Txt::from_static("Combo"));
//! let options = ["Combo", "Congo", "Pombo"];
//! # let _ =
//! Toggle! {
//!     child = TextInput! {
//!         txt = txt.clone();
//!         gesture::on_click = hn!(|a: &gesture::ClickArgs| a.propagation().stop());
//!     };
//!     style_fn = toggle::ComboStyle!();
//!
//!     checked_popup = wgt_fn!(|_| popup::Popup! {
//!         id = "popup";
//!         child = Stack! {
//!             toggle::selector = toggle::Selector::single(txt.clone());
//!             direction = StackDirection::top_to_bottom();
//!             children = options.into_iter().map(|o| Toggle! {
//!                 child = Text!(o);
//!                 value::<Txt> = o;
//!             })
//!             .collect::<UiNodeVec>();
//!         };
//!     })
//! }
//! # ;
//! ```
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
