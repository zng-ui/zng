#![allow(clippy::needless_doctest_main)]
#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]

//! Zng is a cross-platform GUI framework, it provides ready made highly customizable widgets, responsive layout,
//! live data binding, easy localization, automatic focus navigation and accessibility, async and multi-threaded tasks, robust
//! multi-process architecture and more.
//!
//! Zng is pronounced "zing", or as an initialism: ZNG (Z Nesting Graphics).
//!
//! Every component of the framework can be extended, you can create new widgets or add properties to existing ones,
//! at a lower level you can introduce new events and services, seamless integrating custom hardware.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zng = { version = "0.6.2", features = ["view_prebuilt"] }
//! ```
//!
//! Then create your first app:
//!
//! ```no_run
//! use zng::prelude::*;
//!
//! fn main() {
//!     zng::env::init!();
//!     app();
//! }
//!
//! fn app() {
//!     APP.defaults().run_window(async {
//!         Window! {
//!             child_align = Align::CENTER;
//!             child = {
//!                 let size = var(28i32);
//!                 Button! {
//!                     child = Text! {
//!                         txt = "Hello World!";
//!
//!                         #[easing(200.ms())]
//!                         font_size = size.map_into();
//!                     };
//!                     on_click = hn!(|_| {
//!                         let next = size.get() + 10;
//!                         size.set(if next > 80 { 28 } else { next });
//!                     });
//!                 }
//!             };
//!         }
//!     })
//! }
//! ```
//!
//! You can also use a [prebuild view](app#prebuild) and run in the [same process](app#same-process), see [`app`] for more details.
//!
//! # Widgets & Properties
//!
//! The high-level building blocks of UI.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _scope = APP.defaults();
//! # let _ =
//! Button! {
//!     child = Text!("Green?");
//!     widget::background_color = colors::GREEN;
//!     on_click = hn!(|_| println!("SUPER GREEN!"));
//! }
//! # ;
//! ```
//!
//! In the example above [`Button!`] and [`Text!`] are widgets and `child`, [`background_color`] and [`on_click`] are properties.
//! Widgets are mostly an aggregation of properties that define an specific function and presentation, most properties are standalone
//! implementations of an specific behavior or appearance, in the example only `child` is implemented by the button widget, the
//! other two properties can be set in any widget.
//!
//! Each widget is a dual macro and `struct` of the same name, in the documentation only the `struct` is visible, when
//! an struct represents a widget it is tagged with <strong><code>W</code></strong>. Each properties is declared as a function,
//! in the documentation property functions are tagged with <strong><code>P</code></strong>.
//!
//! Widget instances can be of any type, usually they are an opaque [`impl UiNode`], some special widgets have an instance type,
//! the [`Window!`] widget for example has the instance type [`WindowRoot`]. Property instances are always of type `impl UiNode`,
//! each property function takes an `impl UiNode` input plus one or more value inputs and returns an `impl UiNode` output that
//! wraps the input node adding the property behavior, the widgets take care of this node chaining nesting each property
//! instance in the proper order, internally every widget instance is a tree of nested node instances.
//!
//! Widgets and properties are very versatile, each widget documentation page will promote the properties that the widget implementer
//! explicitly associated with the widget, but that is only a starting point.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _app = APP.minimal();
//! # let _ =
//! Wgt! {
//!     layout::align = layout::Align::CENTER;
//!     layout::size = 50;
//!     
//!     #[easing(200.ms())]
//!     widget::background_color = colors::RED;
//!     
//!     when *#gesture::is_hovered {
//!         widget::background_color = colors::GREEN;
//!     }
//! }
//! # ;
//! ```
//!
//! In the example above an [`Wgt!`] is completely defined by stand-alone properties, [`align`] and [`size`] define
//! the bounds of the widget, [`background_color`] fills the bounds with color and [`is_hovered`] reacts to pointer interaction.
//!
//! The example also introduces [`when`] blocks, [state properties] and the [`easing`] property attribute. State properties
//! compute an state from the widget, this state can be used to change the value of other properties. When blocks are a powerful
//! feature of widgets, they declare conditional property values. The easing attribute can be set in any property with transitionable
//! values to smoothly animate between changes.
//!
//! The [`widget`](mod@widget) module documentation provides an in-depth explanation of how widgets and properties work.
//!
//! [`Button!`]: struct@button::Button
//! [`Window!`]: struct@window::Window
//! [`Text!`]: struct@text::Text
//! [`Wgt!`]: struct@widget::Wgt
//! [`background_color`]: fn@widget::background_color
//! [`on_click`]: fn@gesture::on_click
//! [`is_hovered`]: fn@gesture::is_hovered
//! [`align`]: fn@layout::align
//! [`size`]: fn@layout::size
//! [`when`]: widget#when
//! [state properties]: widget#state-properties
//! [`easing`]: widget::easing
//! [`impl UiNode`]: widget::node::UiNode
//! [`WindowRoot`]: window::WindowRoot
//!
//! # Variables
//!
//! Observable values that glue most of the UI together.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _app = APP.minimal();
//! let btn_pressed = var(false);
//!
//! # let _ =
//! Stack! {
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 10;
//!     children = ui_vec![
//!         Button! {
//!             child = Text! { txt = "Press Me!"; };
//!             gesture::is_pressed = btn_pressed.clone();   
//!         },
//!         Text! {
//!             txt = btn_pressed.map(|&b| {
//!                 if b {
//!                     "Button is pressed!"
//!                 } else {
//!                     "Button is not pressed."
//!                 }.into()
//!             });
//!         }
//!     ]
//! }
//! # ;
//! ```
//!
//! The example above binds the pressed state of a widget with the text content of another using a [`var`]. Variables
//! are the most common property input kind, in the example `direction`, `spacing`, `is_pressed` and `txt` all accept
//! an [`IntoVar<T>`] input that gets converted into a [`Var<T>`] when the property is instantiated.
//!
//! There are multiple variable types, they can be a simple static value, a shared observable and modifiable value or a
//! contextual value. Variables can also depend on other variables automatically updating when input variables update.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _app = APP.minimal();
//! fn ui(txt: impl IntoVar<Txt>) -> impl UiNode {
//!     Text!(txt)
//! }
//!
//! ui("static value");
//!
//! let txt = var(Txt::from("dynamic value"));
//! ui(txt.clone());
//! txt.set("change applied next update");
//!
//! let show_txt = var(true);
//! ui(expr_var!(if *#{show_txt} { #{txt}.clone() } else { Txt::from("") }));
//!
//! ui(text::FONT_COLOR_VAR.map(|s| formatx!("font color is {s}")));
//! ```
//!
//! In the example a [`var`] clone is shared with the UI and a new value is scheduled for the next app update. Variable
//! updates are batched, during each app update pass every property can observe the current value and schedule modifications to
//! the value, the modifications are only applied after, potentially causing a new update pass if any value actually changed, see
//! [var updates] in the [var module] documentation for more details.
//!
//! The example also demonstrates the [`expr_var!`], a read-only observable variable that interpolates other variables, the
//! value of this variable automatically update when any of the interpolated variables update.
//!
//! And finally the example demonstrates a context var, `FONT_COLOR_VAR`. Context variables get their value from the
//! *environment* where they are used, the UI in the example can show different a different text depending on where it is placed.
//! Context variables are usually encapsulated by properties strongly associated with a widget, most of [`Text!`] properties just
//! set a context var that affects all text instances in the widget they are placed and descendant widgets.
//!
//! There are other useful variable types, see the [var module] module documentation for more details.
//!
//! [`var`]: var::var
//! [`expr_var!`]: var::expr_var
//! [var module]: crate::var
//! [`IntoVar<T>`]: var::IntoVar
//! [`Var<T>`]: var::Var
//!
//! # Context
//!
//! Context or *ambient* values set on parent widgets affecting descendant widgets.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _app = APP.minimal();
//! # let _ =
//! Stack! {
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 10;
//!     
//!     text::font_color = colors::RED;
//!
//!     children = ui_vec![
//!         Button! { child = Text!("Text 1"); },
//!         Button! { child = Text!("Text 2"); },
//!         Button! {
//!             child = Text!("Text 3");
//!             text::font_color = colors::GREEN;
//!         },
//!     ];
//! }
//! # ;
//! ```
//!
//! In the example above "Text 1" and "Text 2" are rendered in red and "Text 3" is rendered in green. The context
//! of a widget is important, `text::font_color` sets text color in the `Stack!` widget and all descendant widgets,
//! the color is overridden in the third `Button!` for the context of that button and descendants, the `Text!`
//! widget has a different appearance just by being in a different context.
//!
//! Note that the text widget can also set the color directly, in the following example the "Text 4" is blue, this
//! value is still contextual, but texts are usually leaf widgets so only the text is affected.
//!
//! ```
//! # use zng::prelude::*;
//! # let _app = APP.minimal();
//! # let _ =
//! Text! {
//!     txt = "Text 4";
//!     font_color = colors::BLUE;
//! }
//! # ;
//! ```
//!
//! In the example above a context variable defines the text color, but not just variables are contextual, layout
//! units and widget services are also contextual, widget implementers may declare custom contextual values too,
//! see [context local] in the app module documentation for more details.
//!
//! [context local]: app#context-local
//!  
//! # Services
//!
//! App or contextual value and function providers.
//!
//! ```
//! use zng::prelude::*;
//! use zng::clipboard::CLIPBOARD;
//!
//! # let _app = APP.minimal();
//! # let _ =
//! Stack! {
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 10;
//!
//!     children = {
//!         let txt = var(Txt::from(""));
//!         let txt_is_err = var(false);
//!         ui_vec![
//!             Button! {
//!                 child = Text!("Paste");
//!                 on_click = hn!(txt, txt_is_err, |_| {
//!                     match CLIPBOARD.text() {
//!                         Ok(p) => if let Some(t) = p {
//!                             txt.set(t);
//!                             txt_is_err.set(false);
//!                         },
//!                         Err(e) => {
//!                             let t = WIDGET.trace_path();
//!                             txt.set(formatx!("error in {t}: {e}"));
//!                             txt_is_err.set(true);
//!                         }
//!                     }
//!                 });
//!             },
//!             Text! {
//!                 txt;
//!                 when *#{txt_is_err} {
//!                     font_color = colors::RED;
//!                 }
//!             }
//!         ]
//!     };
//! }
//! # ;
//! ```
//!
//! The example above uses two services, `CLIPBOARD` and `WIDGET`. Services are represented
//! by an unit struct named like a static item, service functionality is available as methods on
//! this unit struct. Services are contextual, `CLIPBOARD` exists on the app context, it can only operate
//! in app threads, `WIDGET` represents the current widget and can only be used inside a widget.
//!
//! The default app provides multiple services, some common ones are [`APP`], [`WINDOWS`], [`WINDOW`], [`WIDGET`],
//! [`FOCUS`], [`POPUP`], [`DATA`] and more. Services all follow the same pattern, they are a unit struct named like a static
//! item, if you see such a type it is a service.
//!
//! Most services are synchronized with the update cycle. If the service provides a value that value does not change mid-update, all
//! widgets read the same value in the same update. If the service run some operation it takes requests to run the operation, the
//! requests are only applied after the current UI update. This is even true for the [`INSTANT`] service that provides the current
//! time.
//!
//! [`WINDOWS`]: window::WINDOWS
//! [`WINDOW`]: window::WINDOW
//! [`WIDGET`]: widget::WIDGET
//! [`FOCUS`]: focus::FOCUS
//! [`POPUP`]: popup::POPUP
//! [`DATA`]: data_context::DATA
//! [`INSTANT`]: app::INSTANT
//!
//! # Events & Commands
//!
//! Targeted messages send from the system to widgets or from one widget to another.
//!
//! ```no_run
//! use zng::{prelude::*, clipboard::{on_paste, CLIPBOARD, PASTE_CMD}};
//!
//! APP.defaults().run_window(async {
//!     let cmd = PASTE_CMD.scoped(WINDOW.id());
//!     let paste_btn = Button! {
//!         child = Text!(cmd.name());
//!         widget::enabled = cmd.is_enabled();
//!         widget::visibility = cmd.has_handlers().map_into();
//!         tooltip = Tip!(Text!(cmd.name_with_shortcut()));
//!         on_click = hn!(|args: &gesture::ClickArgs| {
//!             args.propagation().stop();
//!             cmd.notify();
//!         });
//!     };
//!
//!     let pasted_txt = var(Txt::from(""));
//!
//!     Window! {
//!         on_paste = hn!(pasted_txt, |_| {
//!             if let Some(t) = CLIPBOARD.text().ok().flatten() {
//!                 pasted_txt.set(t);
//!             }
//!         });
//!         
//!         child = Stack! {
//!             children_align = Align::CENTER;
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 20;             
//!             children = ui_vec![paste_btn, Text!(pasted_txt)];
//!         };
//!     }
//! });
//! ```
//!
//! The example above uses events and command events. Events are represented by a static instance
//! of [`Event<A>`] with name suffix `_EVENT`. Events are usually abstracted by
//! one or more event property, event properties are named with prefix `on_` and accept one input of
//! [`impl WidgetHandler<A>`]. Commands are specialized events represented by a static instance of [`Command`]
//! with name suffix `_CMD`. Every command is also an `Event<CommandArgs>`, unlike other events it is common
//! for the command instance to be used directly.
//!
//! The `on_click` property handles the `CLICK_EVENT` when the click was done with the primary button and targets
//! the widget or a descendant of the widget. The [`hn!`] is a widget handler that synchronously handles the event.
//! See the [`event`] module documentation for details about event propagation, targeting and route. And see
//! [`handler`] module for other handler types, including [`async_hn!`] that enables async `.await` in any event property.
//!
//! The example above defines a button for the `PASTE_CMD` command scoped on the window. Scoped commands are different
//! instances of [`Command`], the command scope can be a window or widget ID, the scope is the target of the command and
//! the context of the command metadata. In the example the button is only visible if the command scope (window) has
//! a paste handler, the button is only enabled it at least one paste handler on the scope is enabled, the button also
//! displays the command name and shortcut metadata, and finally on click the button notifies a command event that is
//! received in `on_click`.
//!
//! Commands enable separation of concerns, the button in the example does not need to know what the window will do on paste,
//! in fact the button does not even need to know what command it is requesting. Widgets can also be controlled using commands,
//! the `Scroll!` widget for example can be controlled from anywhere else in the app using the [`scroll::cmd`] commands. See
//! the [commands](event#commands) section in the event module documentation for more details.
//!
//! [`Event<A>`]: event::Event
//! [`Command`]: event::Command
//! [`impl WidgetHandler<A>`]: handler::WidgetHandler
//! [`hn!`]: handler::hn!
//! [`async_hn!`]: handler::async_hn!
//!
//! # Layout
//!
//! Contextual properties and constraints that affect how a widget is sized and placed on the screen.
//!
//! ```
//! use zng::prelude::*;
//! # let _app = APP.minimal();
//!
//! # let _ =
//! Container! {
//!     layout::size = (400, 350);
//!     widget::background_color = colors::BLUE.darken(70.pct());    
//!
//!     child = Button! {
//!         child = Text!("Text");
//!     
//!         layout::align = layout::Align::CENTER;
//!         layout::size = (60.pct(), 70.pct());
//!     };
//! }
//! # ;
//! ```
//!
//! In the example above the container widget sets an exact size using `layout::size` with exact units, the
//! button widget sets a relative size using percentage units and positions itself in the container using `layout::align`.
//! All the layout properties are stand-alone, in the example only the text widget implements layout directly. Layout
//! properties modify the layout context by setting constraints and defining units, this context is available for all
//! properties that need it during layout, see the [`layout`] module documentation for more details.
//!
//! # Error Handling
//!
//! Recoverable errors handled internally are logged using [`tracing`], in debug builds tracing events (info, warn and error)
//! are printed using [`app::print_tracing`] by default if no tracing subscriber is set before the app starts building.
//!
//! Components always attempt to recover from errors when possible, or at least attempt to contain errors and turn then into
//! a displayable message. The general idea is to at least give the end user a chance to workaround the issue.
//!
//! Components do not generally attempt to recover from panics, with some notable exceptions. The view-process will attempt to respawn
//! if it crashes, because all state is safe in the app-process all windows and frames can be recreated, this lets the app survive
//! some catastrophic video driver errors, like a forced disconnect caused by a driver update. The [`task::spawn`] and related
//! fire-and-forget task runners will also just log the panic as an error.
//!
//! You can also use the [`zng::app::crash_handler`] to collect panic backtraces, crash minidumps, show a crash dialog to the user
//! and restart the app. During development a debug crash dialog is provided, it shows the stdout/stderr, panics stacktrace and
//! minidumps collected if any non-panic fatal error happens.
//!
//! [`tracing`]: https://docs.rs/tracing
//!
//! # In-Depth Documentation
//!
//! This crate level documentation only gives an overview required to start making apps using existing widgets and properties.
//! All top-level modules in this crate contains in-depth documentation about their subject, of particular importance the
//! [`app`], [`widget`](mod@widget), [`layout`] and [`render`] modules should give you a solid understanding of how everything works.
//!
//! ## Cargo Features
//!
//! See the [Cargo Features] section in the crate README for Cargo features documentation.
//!
//! [Cargo Features]: https://github.com/zng-ui/zng/tree/master/crates/zng#cargo-features

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

// manually expanded enable_widget_macros to avoid error running doc tests:
// macro-expanded `extern crate` items cannot shadow names passed with `--extern`
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zng;
#[doc(hidden)]
pub use zng_app::__proc_macro_util;

pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

pub mod access;
pub mod ansi_text;
pub mod app;
pub mod button;
pub mod checkerboard;
pub mod clipboard;
pub mod color;
pub mod config;
pub mod container;
pub mod data_context;
pub mod data_view;
pub mod env;
pub mod event;
pub mod focus;
pub mod font;
pub mod fs_watcher;
pub mod gesture;
pub mod grid;
pub mod handler;
pub mod hot_reload;
pub mod icon;
pub mod image;
pub mod keyboard;
pub mod l10n;
pub mod label;
pub mod layer;
pub mod layout;
pub mod markdown;
pub mod menu;
pub mod mouse;
pub mod panel;
pub mod pointer_capture;
pub mod popup;
pub mod render;
pub mod rule_line;
pub mod scroll;
pub mod selectable;
pub mod stack;
pub mod state_map;
pub mod style;
pub mod task;
pub mod text;
pub mod text_input;
pub mod third_party;
pub mod timer;
pub mod tip;
pub mod toggle;
pub mod touch;
pub mod undo;
pub mod update;
pub mod var;
pub mod view_process;
pub mod widget;
pub mod window;
pub mod wrap;

/// Start and manage an app process.
pub struct APP;
impl std::ops::Deref for APP {
    type Target = zng_app::APP;

    fn deref(&self) -> &Self::Target {
        &zng_app::APP
    }
}

/// Types for general app development.
///
/// See also [`prelude_wgt`] for declaring new widgets and properties.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::__prelude::*;
}
mod __prelude {
    pub use crate::APP;
    pub use crate::{color, gesture, keyboard, layout, mouse, task, timer, touch, widget};

    pub use zng_task::rayon::prelude::{
        FromParallelIterator as _, IndexedParallelIterator as _, IntoParallelIterator as _, IntoParallelRefIterator as _,
        IntoParallelRefMutIterator as _, ParallelBridge as _, ParallelDrainFull as _, ParallelDrainRange as _, ParallelExtend as _,
        ParallelIterator as _, ParallelSlice as _, ParallelSliceMut as _, ParallelString as _,
    };

    pub use zng_task::io::{
        AsyncBufRead as _, AsyncRead as _, AsyncReadExt as _, AsyncSeek as _, AsyncSeekExt as _, AsyncWrite as _, AsyncWriteExt as _,
    };

    pub use zng_app::{
        event::{AnyEventArgs as _, CommandInfoExt as _, CommandNameExt as _, CommandParam, EventArgs as _},
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once},
        shortcut::{shortcut, CommandShortcutExt as _},
        widget::{
            easing,
            node::{ui_vec, UiNode, UiNodeList, UiNodeListChain as _, UiNodeVec},
            AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _, WidgetId, WIDGET,
        },
        window::{WindowId, WINDOW},
        INSTANT,
    };

    pub use zng_app::widget::inspector::WidgetInfoInspectorExt as _;

    pub use zng_var::{
        context_var, expr_var, getter_var, merge_var, state_var, var, var_from, when_var, AnyVar as _, AnyWeakVar as _, IntoValue, IntoVar,
        Var, VarValue, WeakVar as _,
    };

    pub use crate::var::animation::easing;

    pub use zng_layout::unit::{
        Align, AngleUnits as _, ByteUnits as _, DipToPx as _, FactorUnits as _, Layout1d as _, Layout2d as _, Length, LengthUnits as _,
        LineFromTuplesBuilder as _, PxToDip as _, RectFromTuplesBuilder as _, ResolutionUnits as _, TimeUnits as _,
    };

    pub use zng_txt::{formatx, ToTxt as _, Txt};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use zng_color::{colors, hex, hsl, hsla, hsv, hsva, rgb, rgba, web_colors, MixAdjust as _};

    pub use zng_ext_clipboard::CLIPBOARD;

    pub use zng_ext_config::CONFIG;

    pub use zng_ext_font::{FontStretch, FontStyle, FontWeight};

    pub use zng_ext_image::ImageSource;

    pub use zng_wgt_image::Image;

    pub use zng_ext_input::{
        focus::{cmd::CommandFocusExt as _, iter::IterFocusableExt as _, WidgetInfoFocusExt as _, FOCUS},
        gesture::{CommandShortcutMatchesExt as _, HeadlessAppGestureExt as _},
        keyboard::HeadlessAppKeyboardExt as _,
        mouse::WidgetInfoMouseExt as _,
    };

    pub use zng_ext_l10n::{l10n, lang, L10N};

    pub use zng_wgt_text::lang;

    pub use zng_ext_undo::{CommandUndoExt as _, REDO_CMD, UNDO, UNDO_CMD};

    pub use zng_ext_window::{
        AppRunWindowExt as _, HeadlessAppWindowExt as _, WINDOW_Ext as _, WidgetInfoImeArea as _, WindowCloseRequestedArgs, WindowIcon,
        WINDOWS,
    };

    pub use zng_wgt::Wgt;

    pub use crate::text;
    pub use zng_wgt_text::Text;

    pub use zng_wgt_text_input::{selectable::SelectableText, TextInput};

    pub use crate::window;
    pub use zng_wgt_window::Window;

    pub use zng_wgt_container::Container;

    pub use zng_wgt_button::Button;

    pub use zng_wgt_data::{data, DATA};

    pub use crate::grid;
    pub use zng_wgt_grid::Grid;

    pub use crate::layer;
    pub use zng_wgt_layer::{AnchorMode, LayerIndex, LAYERS};

    pub use zng_wgt_text::icon::CommandIconExt as _;

    pub use crate::popup;
    pub use zng_wgt_layer::popup::POPUP;

    pub use crate::menu;
    pub use zng_wgt_menu::{
        context::{context_menu, context_menu_fn, ContextMenu},
        sub::SubMenu,
        Menu,
    };

    pub use zng_wgt_rule_line::hr::Hr;

    pub use zng_wgt_scroll::{Scroll, SCROLL};

    pub use crate::toggle;
    pub use zng_wgt_toggle::Toggle;

    pub use crate::tip;
    pub use zng_wgt_tooltip::{tooltip, tooltip_fn, Tip};

    pub use zng_wgt::{wgt_fn, WidgetFn};

    pub use zng_wgt_style::{style_fn, Style};

    pub use zng_wgt_stack::{Stack, StackDirection};

    pub use zng_wgt_wrap::Wrap;

    pub use zng_wgt_data_view::{DataView, DataViewArgs};
}

/// Prelude for declaring new properties and widgets.
///
/// This prelude can be imported over [`prelude`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// use zng::{prelude::*, prelude_wgt::*};
///
/// /// A button with only text child.
/// #[widget($crate::TextButton)]
/// pub struct TextButton(Button);
///
/// /// Button text.
/// #[property(CHILD, capture, widget_impl(TextButton))]
/// pub fn txt(txt: impl IntoVar<Txt>) { }
///
/// impl TextButton {
///     fn widget_intrinsic(&mut self) {
///         self.widget_builder().push_build_action(|b| {
///             let txt = b
///                     .capture_var::<Txt>(property_id!(Self::txt))
///                     .unwrap_or_else(|| LocalVar(Txt::from("")).boxed());
///             b.set_child(Text!(txt));
///         });
///     }
/// }
/// ```
pub mod prelude_wgt {
    #[doc(no_inline)]
    pub use crate::__prelude_wgt::*;
}
mod __prelude_wgt {
    pub use zng_app::{
        event::{
            command, event, event_args, AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, CommandParam,
            Event, EventArgs as _, EventHandle, EventHandles, EventPropagationHandle,
        },
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, WidgetHandler},
        render::{FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, SpatialFrameId, TransformStyle},
        shortcut::{shortcut, CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts},
        timer::{DeadlineHandle, DeadlineVar, TimerHandle, TimerVar, TIMERS},
        update::{EventUpdate, UpdateDeliveryList, UpdateOp, WidgetUpdates, UPDATES},
        widget::{
            base::{WidgetBase, WidgetImpl},
            border::{BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER},
            builder::{property_id, NestGroup, WidgetBuilder, WidgetBuilding},
            easing,
            info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetMeasure, WidgetPath,
            },
            node::{
                match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, ArcNode, ArcNodeList, BoxedUiNode,
                BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, FillUiNode, NilUiNode, PanelList, SortingList, UiNode,
                UiNodeList, UiNodeListChain as _, UiNodeListObserver, UiNodeOp, UiNodeVec, ZIndex, SORTING_LIST,
            },
            property, ui_node, widget, widget_impl, widget_mixin, widget_set, AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _,
            WidgetId, WidgetUpdateMode, WIDGET,
        },
        window::{MonitorId, WindowId, WINDOW},
        DInstant, Deadline, INSTANT,
    };

    pub use zng_var::{
        context_var, expr_var, getter_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, when_var,
        AnyVar as _, AnyWeakVar as _, ArcVar, BoxedVar, ContextVar, IntoValue, IntoVar, LocalVar, ReadOnlyArcVar, ResponderVar,
        ResponseVar, Var, VarCapability, VarHandle, VarHandles, VarValue, WeakVar as _,
    };

    pub use zng_layout::{
        context::{LayoutDirection, LayoutMetrics, DIRECTION_VAR, LAYOUT},
        unit::{
            Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Dip, DipBox, DipPoint, DipRect, DipSideOffsets,
            DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _, Layout1d as _,
            Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox, PxConstraints,
            PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform, PxVector, Rect,
            RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
        },
    };

    pub use zng_txt::{formatx, ToTxt as _, Txt};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use crate::task;

    pub use zng_app_context::{
        app_local, context_local, CaptureFilter, ContextLocal, ContextValueSet, FullLocalContext, LocalContext, RunOnDrop,
    };

    pub use crate::state_map;
    pub use zng_state_map::{static_id, OwnedStateMap, StateId, StateMapMut, StateMapRef};

    pub use zng_wgt::prelude::{IdEntry, IdMap, IdSet};

    pub use zng_wgt::{wgt_fn, WidgetFn};

    pub use zng_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, gradient, hex, hsl, hsla, hsv, hsva, rgb, rgba, web_colors,
        ColorPair, ColorScheme, Hsla, Hsva, MixAdjust as _, MixBlendMode, Rgba,
    };

    pub use zng_wgt::node::{
        bind_state, border_node, command_property, event_property, event_state, event_state2, event_state3, event_state4, fill_node,
        list_presenter, presenter, presenter_opt, widget_state_get_state, widget_state_is_state, with_context_blend, with_context_local,
        with_context_local_init, with_context_var, with_context_var_init, with_widget_state, with_widget_state_modify,
    };

    pub use zng_ext_window::WidgetInfoBuilderImeArea as _;

    #[cfg(feature = "hot_reload")]
    pub use crate::hot_reload::hot_node;
}

mod defaults {
    use zng_app::{AppExtended, AppExtension};
    use zng_ext_clipboard::ClipboardManager;
    use zng_ext_config::ConfigManager;
    use zng_ext_font::FontManager;
    use zng_ext_fs_watcher::FsWatcherManager;
    use zng_ext_image::ImageManager;
    use zng_ext_input::{
        focus::FocusManager, gesture::GestureManager, keyboard::KeyboardManager, mouse::MouseManager,
        pointer_capture::PointerCaptureManager, touch::TouchManager,
    };
    use zng_ext_l10n::L10nManager;
    use zng_ext_undo::UndoManager;
    use zng_ext_window::WindowManager;

    #[cfg(feature = "dyn_app_extension")]
    macro_rules! DefaultsAppExtended {
        () => {
            AppExtended<Vec<Box<dyn zng_app::AppExtensionBoxed>>>
        }
    }
    #[cfg(not(feature = "dyn_app_extension"))]
    macro_rules! DefaultsAppExtended {
        () => {
            AppExtended<impl AppExtension>
        }
    }

    impl super::APP {
        /// App with default extensions.
        ///     
        /// # Extensions
        ///
        /// Extensions included.
        ///
        /// * [`FsWatcherManager`]
        /// * [`ConfigManager`]
        /// * [`L10nManager`]
        /// * [`PointerCaptureManager`]
        /// * [`MouseManager`]
        /// * [`TouchManager`]
        /// * [`KeyboardManager`]
        /// * [`GestureManager`]
        /// * [`WindowManager`]
        /// * [`FontManager`]
        /// * [`FocusManager`]
        /// * [`ImageManager`]
        /// * [`ClipboardManager`]
        /// * [`UndoManager`]
        /// * [`SingleInstanceManager`] if the `"single_instance"` feature is enabled.
        /// * [`HotReloadManager`] if the `"hot_reload"` feature is enabled.
        /// * [`MaterialFonts`] if any `"material_icons*"` feature is enabled.
        ///
        /// [`MaterialFonts`]: zng_wgt_material_icons::MaterialFonts
        /// [`SingleInstanceManager`]: zng_ext_single_instance::SingleInstanceManager
        /// [`HotReloadManager`]: zng_ext_hot_reload::HotReloadManager
        /// [`ConfigManager`]: zng_ext_config::ConfigManager
        /// [`L10nManager`]: zng_ext_l10n::L10nManager
        /// [`FontManager`]: zng_ext_font::FontManager
        pub fn defaults(&self) -> DefaultsAppExtended![] {
            let r = self
                .minimal()
                .extend(FsWatcherManager::default())
                .extend(ConfigManager::default())
                .extend(L10nManager::default())
                .extend(PointerCaptureManager::default())
                .extend(MouseManager::default())
                .extend(TouchManager::default())
                .extend(KeyboardManager::default())
                .extend(GestureManager::default())
                .extend(WindowManager::default())
                .extend(FontManager::default())
                .extend(FocusManager::default())
                .extend(ImageManager::default())
                .extend(ClipboardManager::default())
                .extend(UndoManager::default());

            #[cfg(all(feature = "view", feature = "view_prebuilt"))]
            tracing::warn!(r#"both "view" and "view_prebuilt" enabled, will use only one, indeterminate witch"#);

            #[cfg(feature = "single_instance")]
            let r = r.extend(zng_ext_single_instance::SingleInstanceManager::default());

            #[cfg(feature = "hot_reload")]
            let r = r.extend(zng_ext_hot_reload::HotReloadManager::default());

            #[cfg(any(
                feature = "material_icons_outlined",
                feature = "material_icons_filled",
                feature = "material_icons_rounded",
                feature = "material_icons_sharp",
            ))]
            let r = r.extend(zng_wgt_material_icons::MaterialFonts);

            r.extend(DefaultsInit {})
        }
    }

    struct DefaultsInit {}
    impl AppExtension for DefaultsInit {
        fn init(&mut self) {
            zng_ext_window::WINDOWS.register_root_extender(|a| {
                let child = a.root;

                #[cfg(feature = "inspector")]
                let child = zng_wgt_inspector::inspector(child, zng_wgt_inspector::live_inspector(true));

                // setup COLOR_SCHEME_VAR for all windows, this is not done in `Window!` because
                // WindowRoot is used directly by some headless renderers.
                zng_wgt::node::with_context_var_init(child, zng_color::COLOR_SCHEME_VAR, || {
                    use zng_ext_window::WINDOW_Ext as _;
                    use zng_var::Var as _;

                    zng_app::window::WINDOW.vars().actual_color_scheme().boxed()
                })
            });
            tracing::debug!("defaults init, root_extender set");

            crate::third_party::setup_default_view();
            tracing::debug!("defaults init, third_party set");

            #[cfg(feature = "single_instance")]
            {
                crate::app::APP_INSTANCE_EVENT
                    .on_pre_event(crate::handler::app_hn!(|args: &crate::app::AppInstanceArgs, _| {
                        use crate::window::*;

                        // focus a window if none are focused.
                        if !args.is_current() && WINDOWS.focused_window_id().is_none() {
                            for w in WINDOWS.widget_trees() {
                                if w.is_rendered()
                                    && WINDOWS.mode(w.window_id()) == Ok(WindowMode::Headed)
                                    && WINDOWS.focus(w.window_id()).is_ok()
                                {
                                    break;
                                }
                            }
                        }
                    }))
                    .perm();
                tracing::debug!("defaults init, single_instance set");
            }

            #[cfg(feature = "material_icons_outlined")]
            {
                use zng_ext_clipboard::*;
                use zng_ext_undo::*;
                use zng_ext_window::cmd::*;
                use zng_wgt::wgt_fn;
                use zng_wgt_input::cmd::*;
                use zng_wgt_material_icons::outlined as icons;
                use zng_wgt_scroll::cmd::*;
                use zng_wgt_text::icon::CommandIconExt as _;
                use zng_wgt_text::icon::Icon;

                CUT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::CUT)));
                COPY_CMD.init_icon(wgt_fn!(|_| Icon!(icons::COPY)));
                PASTE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::PASTE)));

                UNDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::UNDO)));
                REDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::REDO)));

                CLOSE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::CLOSE)));
                MINIMIZE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MINIMIZE)));
                MAXIMIZE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MAXIMIZE)));
                FULLSCREEN_CMD.init_icon(wgt_fn!(|_| Icon!(icons::FULLSCREEN)));

                CONTEXT_MENU_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MENU_OPEN)));

                #[cfg(feature = "inspector")]
                zng_wgt_inspector::INSPECT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::SCREEN_SEARCH_DESKTOP)));

                SCROLL_TO_TOP_CMD.init_icon(wgt_fn!(|_| Icon!(icons::VERTICAL_ALIGN_TOP)));
                SCROLL_TO_BOTTOM_CMD.init_icon(wgt_fn!(|_| Icon!(icons::VERTICAL_ALIGN_BOTTOM)));

                ZOOM_IN_CMD.init_icon(wgt_fn!(|_| Icon!(icons::ZOOM_IN)));
                ZOOM_OUT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::ZOOM_OUT)));

                OPEN_CMD.init_icon(wgt_fn!(|_| Icon!(icons::FILE_OPEN)));

                tracing::debug!("defaults init, command_icons set");
            }
        }
    }
}

#[doc = include_str!("../../README.md")]
#[cfg(doctest)]
pub mod read_me_test {}
