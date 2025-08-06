#![cfg(feature = "data_context")]

//! Data context service and properties.
//!
//! The [`data`](fn@data) property can be set on a widget to any type that can be used in variables ([`VarValue`]). The
//! [`DATA`] service can then be used on the widget or descendant widgets to retrieve the data and to set validation annotations
//! about the data.
//!
//! The example below demonstrates a simple [MVVM] implementation using the data context to share the view-model instance
//! with all widgets in the view. The example also uses the data annotations API to show data validation errors.
//!
//! [MVVM]: https://en.wikipedia.org/wiki/Model%E2%80%93view%E2%80%93viewmodel
//!
//! ```
//! # fn main() { }
//! mod view {
//!     use crate::view_model::*;
//!     use zng::{data_context, prelude::*, window::WindowRoot};
//!
//!     pub fn window() -> WindowRoot {
//!         Window! {
//!             // set data context for entire window, using `var` to be read-write.
//!             data = var(ViewModel::new(crate::model::connect()));
//!
//!             // bind title from data context.
//!             title = DATA.req::<ViewModel>().map(|vm| vm.title());
//!             child = content();
//!         }
//!     }
//!
//!     fn content() -> UiNode {
//!         // `req` panics if context is not set to the same type.
//!         let vm = DATA.req::<ViewModel>();
//!         Container! {
//!             child = TextInput! {
//!                 txt = vm.map_bidi_modify(|vm| vm.new_item(), |v, vm| vm.set_new_item(v.clone()));
//!
//!                 // FieldStyle shows data errors.
//!                 style_fn = style_fn!(|_| zng::text_input::FieldStyle!());
//!                 data_context::data_error = vm.map(|vm| vm.new_item_error());
//!             };
//!             child_bottom = Button! {
//!                 child = Text!("Submit");
//!                 widget::enabled = vm.map(|vm| !vm.new_item().is_empty());
//!                 on_click = hn!(|_| vm.modify(|vm| vm.submit()));
//!             }, 5;
//!             padding = 5;
//!         }
//!     }
//! }
//!
//! mod view_model {
//!     use crate::model::Model;
//!     use zng::text::*;
//!
//!     #[derive(Clone, Debug, PartialEq)]
//!     pub struct ViewModel {
//!         model: Model,
//!         new_item: Txt,
//!         new_item_error: Txt,
//!     }
//!     impl ViewModel {
//!         pub fn new(model: Model) -> Self {
//!             Self {
//!                 model,
//!                 new_item: Txt::from(""),
//!                 new_item_error: Txt::from(""),
//!             }
//!         }
//!
//!         pub fn title(&self) -> Txt {
//!             formatx!("App - {} entries", self.model.read().len())
//!         }
//!
//!         pub fn new_item(&self) -> Txt {
//!             self.new_item.clone()
//!         }
//!         pub fn set_new_item(&mut self, new_item: Txt) {
//!             self.new_item_error = "".into();
//!             self.new_item = new_item;
//!         }
//!
//!         pub fn new_item_error(&self) -> Txt {
//!             self.new_item_error.clone()
//!         }
//!
//!         pub fn submit(&mut self) {
//!             match self.new_item.parse::<u32>() {
//!                 Ok(item) => {
//!                     self.model.write().push(item);
//!                     self.set_new_item(Txt::from(""));
//!                 }
//!                 Err(e) => self.new_item_error = e.to_txt(),
//!             }
//!         }
//!     }
//! }
//!
//! mod model {
//!     use zng::{task::parking_lot::RwLock, var::ArcEq};
//!
//!     pub type Model = ArcEq<RwLock<Vec<u32>>>;
//!
//!     pub fn connect() -> ArcEq<RwLock<Vec<u32>>> {
//!         ArcEq::new(RwLock::new(vec![]))
//!     }
//! }
//! ```
//!
//! [`data`]: fn@data
//! [`VarValue`]: crate::var::VarValue
//!
//! # Full API
//!
//! See [`zng_wgt_data`] for the full API.

pub use zng_wgt_data::{
    DATA, DataNote, DataNoteHandle, DataNoteLevel, DataNoteValue, DataNotes, data, data_error, data_error_color, data_info,
    data_info_color, data_note, data_warn, data_warn_color, extend_data_note_colors, get_data_error, get_data_error_txt, get_data_info,
    get_data_info_txt, get_data_notes, get_data_notes_top, get_data_warn, get_data_warn_txt, has_data_error, has_data_info, has_data_notes,
    has_data_warn, replace_data_note_colors, with_data_note_color,
};
