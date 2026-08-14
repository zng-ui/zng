#![cfg(feature = "wizard")]

//! Wizard widget and related types.
//!
//! By default the [`Wizard!`] widget presents the traditional multi-step dialog/form experience,
//! but it is highly customizable. The widget is composed of four panels: header, footer, side and content,
//! each panel can be customized using contextual properties.
//!
//! The [`Page`] type defines widget builders for the content of each panel when the page is open. It also
//! defines variables that control how navigation works to and from the page.
//!  
//! ```
//! use zng::prelude::*;
//! use zng::wizard;
//! # let _scope = zng::APP.defaults();
//!
//! let wizard_id = WidgetId::new_unique();
//!
//! // basic page with defaults depending on page index
//! let basic_page = wizard::Page::new(
//!     "Header",
//!     "Description of what this pages is asking or doing",
//!     wgt_fn!(|_| {
//!         Markdown! {
//!             txt = "Custom page content.";
//!         }
//!     }),
//! );
//!
//! // more customized page, with wizard commands control
//! let can_cancel = var(true);
//! let can_finish = var(true);
//! let mut page = wizard::Page::new(
//!     "Commands",
//!     "What wizard commands are enabled?",
//!     wgt_fn!(can_cancel, can_finish, |args: wizard::PageArgs| {
//!         Stack! {
//!             // on enter page
//!             widget::on_init = hn!(can_cancel, |_| {
//!                 can_cancel.set(false);
//!             });
//!             // on exit page
//!             widget::on_deinit = hn!(can_cancel, |_| {
//!                 can_cancel.set(true);
//!             });
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 5;
//!             toggle::style_fn = style_fn!(|_| toggle::CheckStyle!());
//!             children = ui_vec![
//!                 Text!("Select what wizard commands are enabled:"),
//!                 Toggle! {
//!                     checked = can_cancel.clone();
//!                     child = Text!("CANCEL_CMD");
//!                 },
//!                 Toggle! {
//!                     checked = can_finish.clone();
//!                     child = Text!("FINISH_CMD");
//!                 },
//!                 Text!("Also wizard navigation commands:"),
//!                 Toggle! {
//!                     checked = args.can_back;
//!                     child = Text!("BACK_CMD");
//!                 },
//!             ];
//!         }
//!     }),
//! );
//! // by default the last page only has BACK and FINISH buttons
//! page.footer = wgt_fn!(|_| {
//!     ui_vec![
//!         Button! {
//!             cmd = wizard::BACK_CMD.scoped(wizard_id);
//!             tab_index = TabIndex::FIRST - 1;
//!         },
//!         Button! {
//!             cmd = wizard::FINISH_CMD.scoped(wizard_id);
//!             tab_index = TabIndex::FIRST;
//!             style_fn = style_fn!(|_| zng::button::PrimaryStyle!());
//!         },
//!         Button! {
//!             cmd = wizard::CANCEL_CMD.scoped(wizard_id);
//!             tab_index = TabIndex::FIRST - 2;
//!         },
//!     ]
//!     .into_node()
//! });
//!
//! Wizard! {
//!     id = wizard_id;
//!     // side_background_fn = wgt_fn!(|_| flood(colors::RED));
//!     // header_background_fn = wgt_fn!(|_| flood(colors::BLUE));
//!     pages = vec![basic_page, page];
//!     can_cancel;
//!     on_cancel = hn!(|a| {
//!         println!("Cancel!");
//!         WINDOW.close();
//!     });
//!     finish_cmd_name = "Apply";
//!     can_finish;
//!     on_finish = hn!(|a| {
//!         println!("Finish!");
//!         WINDOW.close();
//!     });
//! }
//! ```

pub use zng_wgt_wizard::{
    BACK_CMD, CANCEL_CMD, ContentFnArgs, FINISH_CMD, FooterFnArgs, HeaderFnArgs, NEXT_CMD, Page, PageArgs, PanelFnArgs, SideFnArgs, Wizard,
    content_fn, footer_fn, header_background_fn, header_fn, panel_fn, side_background_fn, side_fn,
};
