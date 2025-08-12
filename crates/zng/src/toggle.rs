#![cfg(feature = "toggle")]

//! Toggle button widget and styles for check box, combo box, radio button and switch button.
//!
//! The [`Toggle!`](struct@Toggle) widget has three states, `Some(true)`, `Some(false)` and `None`. How
//! the widget toggles between this values is defined by what property is used to bind the state.
//!
//! The [`checked`](struct@Toggle#checked) property binds to a `bool` variable and toggles between `true` and `false` only.
//! The example below makes use of the property.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let checked = var(false);
//! # let _ =
//! Toggle! {
//!     child = Text!(checked.map(|b| formatx!("checked = {b}")));
//!     checked;
//! }
//! # ;
//! ```
//!
//! The [`checked_opt`](struct@Toggle#method.checked_opt) and [`tristate`](struct@Toggle#method.tristate) properties can be used to toggle
//! between `Some(true)` and `Some(false)` and accept the `None` value, or with tristate enabled to include `None` in the toggle cycle.
//! Note that even if tristate is not enabled the variable can be set to `None` from another source and the widget will display the
//! `None` appearance.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let checked = var(Some(false));
//! # let _ =
//! Toggle! {
//!     child = Text!(checked.map(|b| formatx!("checked = {b:?}")));
//!     tristate = true;
//!     checked_opt = checked;
//! }
//! # ;
//! ```
//!
//! The [`selector`](fn@selector) and [`value`](struct@Toggle#method.value) properties can be used to have the toggle insert and
//! remove a value from a contextual selection of values. The example below declares a stack with 10 toggle buttons each
//! representing a value, the stack is also setup as a selector context for these toggle buttons, when each toggle button
//! is clicked it replaces the selected value.
//!
//! Note that the toggle does not know what the selection actually is, the [`Selector`] type abstracts over multiple
//! selection kinds, including custom implementations of [`SelectorImpl`].
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let selected_item = var(1_i32);
//! # let _ =
//! Stack! {
//!     toggle::selector = toggle::Selector::single(selected_item.clone());
//!
//!     spacing = 5;
//!     children = (1..=10_i32).map(|i| {
//!         Toggle! {
//!             child = Text!("Item {i}");
//!             value::<i32> = i;
//!         }
//!     }).collect::<Vec<_>>();
//! }
//! # ;
//! ```
//!
//! Regardless of how the checked state of a toggle is defined the [`IS_CHECKED_VAR`] variable and [`is_checked`](fn@is_checked) property
//! can be used to track the checked state of the widget. The example below defines a toggle that changes background color to green
//! when it is in the `Some(true)` state.
//!
//! ```
//! # use zng::prelude::*;
//! # let _scope = APP.defaults();
//! # let _ =
//! Toggle! {
//!     checked = var(false);
//!     // checked_opt = var(Some(false));
//!     // value<i32> = 42;
//!
//!     widget::background_color = colors::RED;
//!     when *#is_checked {
//!         widget::background_color = colors::GREEN;
//!     }
//! }
//! # ;
//! ```
//!
//! # Styles
//!
//! Toggle is a versatile widget, it can be styled to represent check boxes, switches, radio buttons and combo boxes.
//!
//! ## Check & Switch
//!
//! The [`CheckStyle!`](struct@CheckStyle) changes the toggle into a check box. The [`SwitchStyle!`](struct@SwitchStyle)
//! changes the toggle into an on/off switch.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Toggle! {
//!     child = Text!(toggle::IS_CHECKED_VAR.map(|&s| match s {
//!         Some(true) => Txt::from("checked text"),
//!         Some(false) => Txt::from("unchecked text"),
//!         None => Txt::from(""),
//!     }));
//!     checked = var(false);
//!     style_fn = toggle::SwitchStyle!();
//! }
//! # ;
//! ```
//!
//! The example above declares a toggle switch that changes the text depending on the state.
//!
//! ## Radio
//!
//! The [`RadioStyle!`](struct@RadioStyle) can be used in `value` toggle areas. The example below
//! declares a stack that is a selector context and sets the toggle style for all toggle buttons inside.
//!
//! ```
//! # use zng::prelude::*;
//! # let _scope = APP.defaults();
//! let selected_item = var(1_i32);
//! # let _ =
//! Stack! {
//!     toggle::style_fn = style_fn!(|_| toggle::RadioStyle!());
//!     toggle::selector = toggle::Selector::single(selected_item.clone());
//!     // ..
//! }
//! # ;
//! ```
//!
//! ## Combo
//!
//! The [`ComboStyle!`](struct@ComboStyle) together with the [`checked_popup`](struct@Toggle#method.checked_popup) property can be used
//! to declare a combo box, that is a toggle for a drop down that contains another toggle selector context that selects a value.
//!
//! Note that the `checked_popup` setups the `checked` state, you cannot set one of the other checked properties in the same
//! widget.
//!
//! The example below declares a combo box with a `TextInput!` content, users can type a custom option or open the popup and pick
//! an option. Note that the `ComboStyle!` also restyles `Toggle!` inside the popup to look like a menu item.
//!
//! ```
//! use zng::prelude::*;
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
//!             });
//!         };
//!     })
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_toggle`] for the full widget API.

pub use zng_wgt_toggle::{
    CheckStyle, ComboStyle, DefaultStyle, IS_CHECKED_VAR, LightStyle, RadioStyle, Selector, SelectorError, SelectorImpl, SwitchStyle,
    Toggle, check_spacing, combo_spacing, deselect_on_deinit, deselect_on_new, is_checked, radio_spacing, scroll_on_select, select_on_init,
    select_on_new, selector, style_fn, switch_spacing, tristate,
};

/// Toggle commands.
pub mod cmd {
    pub use zng_wgt_toggle::cmd::{SELECT_CMD, SelectOp, TOGGLE_CMD};
}
