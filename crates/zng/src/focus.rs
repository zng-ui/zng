//! Focus service, properties, events and other types.
//!
//! # Keyboard Focus
//!
//! In an app instance only a single widget can receive keyboard input at a time, this widget has the *keyboard focus*.
//! The operating system defines what window has keyboard focus and the app-process defines what widget has focus, these
//! two systems work in conjunction to define the keyboard focus.
//!
//! You can track the focused widget by listening to the [`FOCUS_CHANGED_EVENT`] event or the [`FOCUS.focused`](FOCUS::focused)
//! variable. The focus state of a widget can be tracked using the [`is_focused`](fn@is_focused), [`is_focus_within`](fn@is_focus_within),
//! [`on_focus_changed`](fn@on_focus_changed) and other properties on this module.
//!
//! ```
//! use zng::{focus, prelude::*};
//!
//! # fn example() {
//!
//! focus::FOCUS_CHANGED_EVENT
//!     .on_pre_event(app_hn!(|args: &focus::FocusChangedArgs, _| {
//!         println!("new_focus: {:?}", args.new_focus);
//!     }))
//!     .perm();
//!
//! # let _ =
//! Stack!(
//!     top_to_bottom,
//!     5,
//!     ui_vec![
//!         Wgt! {
//!             id = "subject";
//!             focus::focusable = true;
//!
//!             layout::size = (100, 30);
//!             widget::background_color = colors::RED;
//!             when *#focus::is_focused {
//!                 widget::background_color = colors::GREEN;
//!             }
//!
//!             focus::on_focus = hn!(|_| {
//!                 println!("subject on_focus");
//!             });
//!             focus::on_blur = hn!(|_| {
//!                 println!("subject on_blur");
//!             });
//!         },
//!         Button! {
//!             child = Text!("Focus subject");
//!             on_click = hn!(|_| {
//!                 FOCUS.focus_widget("subject", /*highlight: */ false);
//!             });
//!         },
//!         Text! {
//!             txt = FOCUS.focused().map(|f| formatx!("focused {f:?}"));
//!         }
//!     ]
//! )
//! # ; }
//! ```
//!
//! # Navigation
//!
//! The keyboard focus can be moved from one widget to the next using the keyboard or the [`FOCUS`] service methods.
//! There are two styles of movement: [tabbing](#tab-navigation) that follows the logical order and [directional](#directional-navigation)
//! that follows the visual order.
//!
//! Keyboard navigation behaves different depending on what region of the screen the current focused widget is in, these regions
//! are called [focus scopes](#focus-scopes). Every window is a focus scope that can be subdivided further.
//!
//! ## Tab Navigation
//!
//! Tab navigation follows a logical order, the position of the widget in the [widget tree](FocusInfoTree),
//! optionally overridden using [`tab_index`](fn@tab_index).
//!
//! Focus is moved forward by pressing `TAB` or calling [`FOCUS.focus_next`](FOCUS::focus_next) and backward by pressing `SHIFT+TAB` or
//! calling [`FOCUS.focus_prev`](FOCUS::focus_prev).
//!
//! ## Directional Navigation
//!
//! Directional navigation follows the visual position of the widget on the screen.
//!
//! Focus is moved by pressing the **arrow keys** or calling the focus direction methods in the [`FOCUS`](FOCUS::focus_up) service.
//!
//! ## Focus Scopes
//!
//! Focus scopes are widgets that configure how focus navigation happens inside then. They control what happens
//! when the scope widget is focused, how the navigation flows inside their screen region and even if the navigation
//! can naturally mode out of their region.
//!
//! You can use the [`focus_scope`](fn@focus_scope) property on a widget to turn it into a focus scope and use
//! the [`tab_nav`](fn@tab_nav), [`directional_nav`](fn@directional_nav) and other properties on this module to
//! configure the focus scope.
//!
//! ### Alt Scopes
//!
//! Alt scopes are specially marked focus scopes that receive focus when the `ALT`
//! key is pressed or [`FOCUS.focus_alt`](FOCUS::focus_alt) is called. The alt scope of a widget
//! is selected by [`WidgetFocusInfo::alt_scope`].
//!
//! Alt scopes remember the previously focused widget as a [return focus](#return-focus). The focus returns ALT is pressed again,
//! or [`FOCUS.focus_exit`](FOCUS::focus_exit) is called and the parent is the focus scope.
//!
//! ### Return Focus
//!
//! Focus scopes can be configured to remember the last focused widget inside then, the focus than **returns** to
//! this widget when the scope receives focus. Alt scopes also remember the widget from which the *alt* focus happened
//! and can also return focus back to that widget.
//!
//! You can track the return focus by listening to the [`RETURN_FOCUS_CHANGED_EVENT`] event or
//! [`FOCUS.return_focused`](FOCUS::return_focused) variable. Usually the window root scope remembers
//! return focus and some widgets, like text fields visually indicate that they will be focused when the window
//! is focused.
//!
//! You can use the [`focus_scope_behavior`](fn@focus_scope_behavior) property to configure a custom focus scope
//! to remember the return focus.
//!
//! # Configuring Widgets
//!
//! Focusable configuration is set as info metadata using the [`FocusInfoBuilder`]. You can use this type to make a widget
//! focusable or a focus scope and customize how the focus manager interacts with the widget.
//!
//! Note that the main crate already provides properties for configuring focus in widgets, you only need to
//! set the [`FocusInfoBuilder`] directly if you are developing your own focus defining properties.
//!
//! # Querying
//!
//! Focus information exists as metadata associated with a window widget tree. This metadata can be manually queried by
//! creating a [`FocusInfoTree`] or directly from a widget info by using the [`WidgetInfoFocusExt`] extension methods.
//!
//! # Full API
//!
//! See [`zng_ext_input::focus`] and [`zng_wgt_input::focus`] for the full focus API.

pub use zng_ext_input::focus::{
    DirectionalNav, FOCUS, FOCUS_CHANGED_EVENT, FocusChangedArgs, FocusChangedCause, FocusInfo, FocusInfoBuilder, FocusInfoTree,
    FocusNavAction, FocusRequest, FocusScopeOnFocus, FocusTarget, RETURN_FOCUS_CHANGED_EVENT, ReturnFocusChangedArgs, TabIndex, TabNav,
    WidgetFocusInfo, WidgetInfoFocusExt, cmd, iter,
};
pub use zng_wgt_input::focus::{
    FocusClickBehavior, FocusableMix, alt_focus_scope, directional_nav, focus_click_behavior, focus_highlight, focus_on_init, focus_scope,
    focus_scope_behavior, focus_shortcut, focusable, is_focus_within, is_focus_within_hgl, is_focused, is_focused_hgl, is_return_focus,
    is_return_focus_within, on_blur, on_focus, on_focus_changed, on_focus_enter, on_focus_leave, on_pre_blur, on_pre_focus,
    on_pre_focus_changed, on_pre_focus_enter, on_pre_focus_leave, return_focus_on_deinit, skip_directional, tab_index, tab_nav,
};
