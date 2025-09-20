//! Accessibility service, events and properties.
//!
//! The accessibility API helps external tools to query the state of a widget and issue programmatic commands to it.
//! This API is mainly used by accessibility assistants like [`NVDA`] to narrate and operate the current screen, but
//! usage is not limited to accessibility, the access provided to widgets also enables external automation tools and
//! internal operations such as programmatically clicking a button.
//!
//! [`NVDA`]: https://en.wikipedia.org/wiki/NonVisual_Desktop_Access
//!
//! # Metadata
//!
//! Metadata is collected on demand during info build, there is a small performance impact to this so the access
//! builder is only available after accessibility was requested at least once for the window.
//!
//! ```
//! use zng::prelude_wgt::*;
//!
//! # let _ =
//! match_node_leaf(|op| match op {
//!     UiNodeOp::Info { info } => {
//!         if let Some(mut a) = info.access() {
//!             // accessibility requested for this window
//!             a.set_label("label");
//!         }
//!     }
//!     _ => {}
//! })
//! # ;
//! ```
//!
//! You can also enables access info programmatically using [`WINDOW.enable_access()`], if the view-process did not
//! request accessibility the window still skips sending the access tree, so the performance impact is minimal.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let mut app = APP.defaults().run_headless(false);
//! # app.doc_test_window(async {
//! WINDOW.enable_access();
//!
//! Window! {
//!     child = Button! {
//!         id = "btn-1";
//!         child = Text!("Button 1");
//!     };
//!
//!     widget::on_info_init = hn!(|_| {
//!         let btn_info = WINDOW.info().get("btn-1").unwrap().access().unwrap();
//!         let txt_info = btn_info.info().children().next().unwrap().access().unwrap();
//!
//!         assert_eq!(None, btn_info.label());
//!         assert!(btn_info.labelled_by_child());
//!         assert_eq!(Some(Txt::from("Button 1")), txt_info.label());
//!         # WINDOW.close();
//!     });
//! }
//! # });
//! ```
//!
//! When accessibility info is build you it can be accessed using [`WidgetInfo::access`]. Note that this is a low level
//! access info, provided as it was set by the widgets, in the example above the *label* value is only found on the text widget,
//! accessibility tools will use the text label for the button.
//!
//! [`WINDOW.enable_access()`]: crate::window::WINDOW_Ext::enable_access
//! [`WidgetInfo::access`]: crate::widget::info::WidgetInfo::access
//!
//! ## Properties
//!
//! Properties of this module only define metadata that indicate that the widget implements a certain UI pattern, by
//! setting a property you must make sure that the widget actually implements said pattern, for this reason most
//! of the accessibility definitions are provided by the widget implementations.
//!
//! In the example below a `TextInput!` widget instance changes its role to [`AccessRole::SearchBox`], the default
//! role is set by the widget itself to [`AccessRole::TextInput`], this usage of the widget has a more specific role
//! so it can be changed, in this case it is up to the app developer to actually implement the search.
//!
//! ```
//! use zng::access::{AccessRole, access_role};
//! use zng::prelude::*;
//!
//! # fn example() {
//! let search_txt = var(Txt::from(""));
//! # let _ =
//! TextInput! {
//!     access_role = AccessRole::SearchBox;
//!     placeholder_txt = "search";
//!     txt = search_txt;
//! }
//! # ; }
//! ```
//!
//! # Service & Events
//!
//! The [`ACCESS`] service provides methods that control widgets by notifying accessibility events. Access events
//! are handled by widgets even when accessibility is disabled.
//!
//! In the example below the button shows and hides the tooltip of a different widget using [`ACCESS.show_tooltip`]
//! and [`ACCESS.hide_tooltip`].
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! let mut show_tooltip = false;
//! Window! {
//!     child_align = Align::CENTER;
//!     child = Stack!(
//!         top_to_bottom,
//!         50,
//!         ui_vec![
//!             Button! {
//!                 on_click = hn!(|_| {
//!                     use zng::access::ACCESS;
//!
//!                     show_tooltip = !show_tooltip;
//!                     if show_tooltip {
//!                         ACCESS.show_tooltip(WINDOW.id(), "tooltip-anchor");
//!                     } else {
//!                         ACCESS.hide_tooltip(WINDOW.id(), "tooltip-anchor");
//!                     }
//!                 });
//!                 child = Text!("Toggle Tooltip");
//!             },
//!             Text! {
//!                 id = "tooltip-anchor";
//!                 txt = "tooltip anchor";
//!                 tooltip = Tip!(Text!("Tooltip"));
//!             }
//!         ]
//!     );
//! }
//! # ; }
//! ```
//!
//! [`ACCESS.show_tooltip`]: ACCESS::show_tooltip
//! [`ACCESS.hide_tooltip`]: ACCESS::hide_tooltip
//!
//! # Full API
//!
//! See [`zng_app::access`] and [`zng_wgt_access`] for the full API.

pub use zng_app::access::{
    ACCESS, ACCESS_CLICK_EVENT, ACCESS_EXPANDER_EVENT, ACCESS_INCREMENT_EVENT, ACCESS_INITED_EVENT, ACCESS_NUMBER_EVENT,
    ACCESS_SCROLL_EVENT, ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT, ACCESS_TOOLTIP_EVENT, AccessClickArgs, AccessExpanderArgs,
    AccessIncrementArgs, AccessInitedArgs, AccessNumberArgs, AccessScrollArgs, AccessSelectionArgs, AccessTextArgs, AccessToolTipArgs,
    ScrollCmd,
};
pub use zng_wgt_access::{
    AccessCmdName, AccessRole, AutoComplete, CurrentKind, Invalid, LiveIndicator, Orientation, Popup, SortDirection, access_commands,
    access_role, accessible, active_descendant, auto_complete, checked, col_count, col_index, col_span, controls, current, described_by,
    details, error_message, expanded, flows_to, invalid, item_count, item_index, label, labelled_by, labelled_by_child, level, live, modal,
    multi_selectable, on_access_click, on_access_expander, on_access_increment, on_access_number, on_access_scroll, on_access_selection,
    on_access_text, on_access_tooltip, on_pre_access_click, on_pre_access_expander, on_pre_access_increment, on_pre_access_number,
    on_pre_access_scroll, on_pre_access_selection, on_pre_access_text, on_pre_access_tooltip, orientation, owns, placeholder, popup,
    read_only, required, row_count, row_index, row_span, scroll_horizontal, scroll_vertical, selected, sort, value, value_max, value_min,
};
