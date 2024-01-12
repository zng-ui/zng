//! Data context service and properties.
//!
//! The [`data`](fn@data) property can be set on a widget to any type that can be used in variables ([`VarValue`]). The
//! [`DATA`] service can then be used on the widget or descendant widgets to retrieve the data and to set validation annotations
//! about the data.
//! 
//! The example below TODO!
//! 
//! ```
//! mod view {
//!     use crate::view_model::*;
//!     use zero_ui::{data_context, prelude::*, text_input, window::WindowRoot};
//! 
//!     pub fn window() -> WindowRoot {
//!         Window! {
//!             // set data context for entire window.
//!             data = ViewModel::default();
//! 
//!             // bind title from data context.
//!             title = DATA.req::<ViewModel>().map(|vm| vm.title());
//!             child = content();
//!         }
//!     }
//! 
//!     fn content() -> impl UiNode {
//!         Container! {
//!             child = TextInput! {
//!                 // bind data context, `req` panics if context is not set to the same type.
//!                 txt = DATA.req::<ViewModel>().map_ref_bidi(|vm| vm.typing(), |vm| vm.typing_mut());
//!             };
//!             child_bottom = Button! {
//!                 child = Text!("Submit");
//!                 widget::enabled = DATA.req::<ViewModel>().map(|vm| !vm.typing().is_empty());
//!                 on_click = hn!(|_| {
//!                     // use data context directly.
//!                     DATA.req::<ViewModel>().modify(|vm| vm.to_mut().submit()).unwrap()
//!                 });
//!             }, 5;
//! 
//!             // set data error for all widgets in this container.
//!             data_context::data_error = DATA.req::<ViewModel>().map_ref(|vm| vm.last_error());
//!             // data_context::FieldStyle displays data errors.
//!             text_input::replace_style = style_fn!(|_| text_input::FieldStyle!());
//!         }
//!     }
//! }
//! 
//! mod view_model {
//!     use zero_ui::text::*;
//! 
//!     #[derive(Clone, Debug, PartialEq, Default)]
//!     pub struct ViewModel {
//!         items: Vec<u32>,
//!         typing: Txt,
//!         last_error: Txt,
//!     }
//!     impl ViewModel {
//!         pub fn title(&self) -> Txt {
//!             formatx!("App - {} items", self.items.len())
//!         }
//! 
//!         pub fn typing(&self) -> &Txt {
//!             &self.typing
//!         }
//!         pub fn typing_mut(&mut self) -> &mut Txt {
//!             &mut self.typing
//!         }
//! 
//!         pub fn last_error(&self) -> &Txt {
//!             &self.last_error
//!         }
//! 
//!         pub fn submit(&mut self) {
//!             match self.typing.parse::<u32>() {
//!                 Ok(item) => {
//!                     self.items.push(item);
//!                     self.last_error = Txt::from("");
//!                     self.typing = Txt::from("");
//!                 }
//!                 Err(e) => self.last_error = e.to_txt(),
//!             }
//!         }
//!     }
//! }
//! ```
//! 
//! [`data`]: fn@data
//! [`VarValue`]: crate::var::VarValue
//! 
//! # Full API
//!
//! See [`zero_ui_wgt_data`] for the full API.

pub use zero_ui_wgt_data::{
    data, data_error, data_error_color, data_info, data_info_color, data_note, data_warn, data_warn_color, extend_data_note_colors,
    get_data_error, get_data_error_txt, get_data_info, get_data_info_txt, get_data_notes, get_data_notes_top, get_data_warn,
    get_data_warn_txt, has_data_error, has_data_info, has_data_notes, has_data_warn, replace_data_note_colors, with_data_note_color,
    DataNote, DataNoteHandle, DataNoteLevel, DataNoteValue, DataNotes, DATA, DATA_NOTE_COLORS_VAR,
};
