//! Style mix-in and other types.
//!
//! A [`Style!`](struct@Style) is an special widget that represents a set of properties that are dynamically applied to
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
//! # Named Styles
//!
//! Some widgets provide alternate styles that are also contextual. The example below demonstrates a button using
//! a named style being affected by properties set for that particular style name in context.
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
//!     zng::button::light_style_fn = Style! {
//!         // override the background color of only buttons in the Stack using the `LightStyle!`.
//!         widget::background_color = colors::BLUE;
//!     };
//!     zng::button::style_fn = Style! {
//!         // override the default border for all buttons in the Stack.
//!         widget::border = 2, colors::GREEN;
//!     };
//!
//!     children = ui_vec![
//!         // This button is affected by the contextual light and default styles.
//!         Button! {
//!             child = Text!("BLUE+GREEN");
//!             style_fn = zng::button::LightStyle!();
//!         },
//!         This button is only affected by the contextual default style.
//!         Button! {
//!             child = Text!("Normal+GREEN");
//!         },
//!     ];
//! }
//! ```
//!
//! Named styles accumulate context the same way the widget default style does. When applied the widget properties are
//! set/replaced in this order:
//!
//! 1 - The default properties set on the widget declaration.
//! 1 - The `base_style_fn`, if set directly on the widget.
//! 2 - The default style and any extension/replacement style set for the default style using `style_fn` in a parent widget.
//! 3 - The named style set using `style_fn` on the widget and any extension/replacement style set for the named style property in a parent widget.
//! 4 - The properties set on the widget instance.
//!
//! Note that on the target widget instance only the `style_fn` property is used, the named style property is standalone and for use in parent widgets.
//!
//! Named styles are ideal for cases where an widget can have a distinct appearance without changing its behavior. For example instead of a
//! *CheckBox!* widget the `Toggle!` widget provides a `toggle::{CheckStyle, check_style_fn}`. Theme implementers can restyle *check-boxes* just
//! the same, and widget users don't need to change the widget type just to use a different appearance.
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
//! Stack! {
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 5;
//!     widget::parallel = false; // init buttons sequentially
//!
//!     zng::button::style_fn = Style! {
//!         // background is `impl IntoUiNode` that can't be cloned. Nodes
//!         // are moved to the last place that requests it.
//!         widget::background = zng::color::flood(colors::AZURE);
//!     };
//!     children = ui_vec![
//!         Button! { child = Text!("Default") },
//!         // the last button to init takes the background node.
//!         Button! { child = Text!("Default+AZURE") },
//!     ];
//! }
//! # ; }
//! ```
//!
//! Using a closure to set fixes the issue:
//!
//! ```
//! # use zng::prelude::*;
//! # fn example() {
//! #
//! # let _ =
//! # Stack! {
//! zng::button::style_fn = style_fn!(|_| Style! {
//!     widget::background = zng::color::flood(colors::AZURE);
//! });
//! # }
//! # ; }

pub use zng_wgt_style::{Style, StyleArgs, StyleBuilder, StyleFn, StyleMix, impl_style_fn, style_fn};
