//! Style mix-in and other types.
//!
//! A [`Style!`](struct@Style) is an special widget that represents a set of properties that are dynamically loaded onto
//! another styleable widget. Styleable widgets inherit from [`StyleMix<P>`](struct@StyleMix) and provide a contextual
//! `style_fn` property that sets the widget style.
//!
//! Styles extend the contextual style by default, only replacing the intersection of properties.
//! The special [`replace`](struct@Style#method.replace) property can be set on a style to fully replace the contextual style.
//!
//! The example below demonstrates multiple contexts setting style for buttons.
//!
//! ```
//! use zng::prelude::*;
//! # let _app = APP.defaults();
//!
//! # let _ =
//! Stack! {
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 5;
//!
//!     zng::button::style_fn = Style! {
//!         // override the default background_color for all buttons in the Stack.
//!         // note that this does not override the hovered/pressed background.
//!         widget::background_color = colors::BLUE;
//!     };
//!
//!     children = ui_vec![
//!         // these buttons have the default style with blue background.
//!         Button! {
//!             child = Text!("Default+BLUE");
//!         },
//!         Button! {
//!             child = Text!("Default+BLUE");
//!         },
//!         Stack! {
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 5;
//!
//!             zng::button::style_fn = Style! {
//!                 // override the default border for all buttons in the Stack.
//!                 widget::border = 2, colors::GREEN;
//!             };
//!
//!             children = ui_vec![
//!                 // these buttons have the default style, with blue background and green border.
//!                 Button! {
//!                     child = Text!("Default+BLUE+GREEN");
//!                 },
//!                 Button! {
//!                     child = Text!("Default+BLUE+GREEN");
//!                 },
//!                 Stack! {
//!                     direction = StackDirection::top_to_bottom();
//!                     spacing = 5;
//!
//!                     zng::button::style_fn = Style! {
//!                         // override the context style background_color in the Stack.
//!                         widget::background_color = colors::RED;
//!                     };
//!
//!                     children = ui_vec![
//!                         // these buttons have the default style, with green border and red background.
//!                         Button! {
//!                             child = Text!("Default+GREEN+RED");
//!                         },
//!                         Button! {
//!                             child = Text!("Default+GREEN+RED");
//!                         },
//!                         // this button ignores the contextual style by setting the `style_fn` to a style
//!                         // that is `replace=true`.
//!                         Button! {
//!                             child = Text!("Default");
//!                             style_fn = zng::button::DefaultStyle!();
//!                         },
//!                     ];
//!                 },
//!             ];
//!         },
//!         Stack! {
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 5;
//!
//!             zng::button::style_fn = Style! {
//!                 // replace the default style with this one.
//!                 replace = true;
//!                 widget::background_color = colors::RED;
//!             };
//!
//!             // these buttons only have the red background.
//!             children = ui_vec![
//!                 Button! {
//!                     child = Text!("RED");
//!                 },
//!                 Button! {
//!                     child = Text!("RED");
//!                 },
//!             ];
//!         }
//!     ];
//! }
//! # ;
//! ```
//!
//! # Shared Styles
//!
//! Style instances can be set directly on `style_fn` properties, but if the style is used by more then one widget property values
//! that can't be cloned will only appear on the last widget to use the style. The [`style_fn!`] macro can be used to declare a
//! closure that instantiates the style for each usage. The property values that can't be cloned are `impl IntoUiNode`.
//!
//! The example below demonstrates this issue:
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Stack!(
//!     top_to_bottom,
//!     20,
//!     ui_vec![
//!         Stack! {
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 5;
//!             widget::parallel = false; // init buttons sequentially
//!
//!             zng::button::style_fn = Style! {
//!                 // background is `impl IntoUiNode` that can't be cloned. Nodes
//!                 // are moved to the last place that requests it.
//!                 widget::background = zng::color::flood(colors::AZURE);
//!             };
//!             children = ui_vec![
//!                 Button! { child = Text!("Default") },
//!                 // the last button to init takes the background node.
//!                 Button! { child = Text!("Default+AZURE") },
//!             ]
//!         },
//!         Stack! {
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 5;
//!
//!             // Sets the style to a closure that will be called for each button.
//!             zng::button::style_fn = style_fn!(|_| Style! {
//!                 widget::background = zng::color::flood(colors::AZURE);
//!             });
//!             children = ui_vec![
//!                 // each button gets its own background node.
//!                 Button! {
//!                     child = Text!("Default+AZURE");
//!                 },
//!                 Button! {
//!                     child = Text!("Default+AZURE");
//!                 },
//!             ];
//!         }
//!     ]
//! )
//! # ; }
//! ```

pub use zng_wgt_style::{Style, StyleArgs, StyleBuilder, StyleFn, StyleMix, impl_style_fn, style_fn};
