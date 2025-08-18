#![expect(clippy::needless_doctest_main)]
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
//! zng = { version = "0.16.1", features = ["view_prebuilt"] }
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
//! Widget instances can be of any type, usually they are an opaque [`UiNode`] or a type that is [`IntoUiNode`],
//! some special widgets have non node instance type, the [`Window!`] widget for example has the instance type [`WindowRoot`].
//! Property instances are always of type [`UiNode`], each property function takes an `impl IntoUiNode` input plus one or more value
//! inputs and returns an `UiNode` output that wraps the input node adding the property behavior, the widgets take care of this
//! node chaining nesting each property instance in the proper order, internally every widget instance is a tree of nested node instances.
//!
//! Widgets and properties are very versatile and extendable, widget docs will promote properties that are explicitly associated
//! with the widget type, but that is only a starting point, many other standalone properties can be set in any widget.
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
//! the layout bounds of the widget, [`background_color`] fills the bounds with color and [`is_hovered`] reacts to pointer interaction.
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
//! [`UiNode`]: widget::node::UiNode
//! [`IntoUiNode`]: widget::node::IntoUiNode
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
//!             child = Text! {
//!                 txt = "Press Me!";
//!             };
//!             gesture::is_pressed = btn_pressed.clone();
//!         },
//!         Text! {
//!             txt = btn_pressed.map(|&b| if b { "Button is pressed!" } else { "Button is not pressed." }.into());
//!         }
//!     ];
//! }
//! # ;
//! ```
//!
//! The example above binds the pressed state of a widget with the text content of another using a [`var`]. Variables
//! are the most common property input kind, in the example `direction`, `spacing`, `is_pressed` and `txt` all accept
//! an [`IntoVar<T>`] input that gets converted into a [`Var<T>`] when the property is instantiated.
//!
//! There are multiple variable kinds, they can be a simple constant value, a shared observable and modifiable value or a
//! contextual value. Variables can also depend on other variables automatically updating when input variables update.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _app = APP.minimal();
//! fn ui(txt: impl IntoVar<Txt>) -> UiNode {
//!     Text!(txt)
//! }
//!
//! ui("const value");
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
//! *environment* where they are used, the UI in the example can show a different text depending on where it is placed.
//! Context variables are usually encapsulated by properties strongly associated with a widget, most of [`Text!`] properties just
//! set a context var that affects all text instances in the widget they are placed and descendant widgets.
//!
//! There are other useful variable kinds, see the [var module] module documentation for more details.
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
//!         Button! {
//!             child = Text!("Text 1");
//!         },
//!         Button! {
//!             child = Text!("Text 2");
//!         },
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
//! use zng::clipboard::CLIPBOARD;
//! use zng::prelude::*;
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
//!                         Ok(p) => {
//!                             if let Some(t) = p {
//!                                 txt.set(t);
//!                                 txt_is_err.set(false);
//!                             }
//!                         }
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
//! use zng::{
//!     clipboard::{CLIPBOARD, PASTE_CMD, on_paste},
//!     prelude::*,
//! };
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
//! The [`zng::app::crash_handler`] is enabled by default, it collect panic backtraces, crash minidumps, show a crash dialog to the user
//! and restart the app. During development a debug crash dialog is provided, it shows the stdout/stderr, panics stacktrace and
//! minidumps collected if any non-panic fatal error happens. Note that the crash handler **stops debuggers from working**, see the
//! [Debugger section] of the crash-handler docs on how to automatically disable the crash handler for debugger runs.
//!
//! [`tracing`]: https://docs.rs/tracing
//! [Debugger section]: zng::app::crash_handler#debugger
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
//! [Cargo Features]: https://github.com/zng-ui/zng/tree/main/crates/zng#cargo-features

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
pub mod dialog;
pub mod drag_drop;
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
pub mod progress;
pub mod render;
pub mod rule_line;
pub mod scroll;
pub mod selectable;
pub mod slider;
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
        INSTANT,
        event::{AnyEventArgs as _, CommandInfoExt as _, CommandNameExt as _, CommandParam, EventArgs as _},
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once},
        shortcut::{CommandShortcutExt as _, shortcut},
        widget::{
            AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _, WIDGET, WidgetId, easing,
            node::{IntoUiNode, UiNode, UiVec, ui_vec},
        },
        window::{WINDOW, WindowId},
    };

    pub use zng_app::widget::inspector::WidgetInfoInspectorExt as _;

    pub use zng_var::{
        IntoValue, IntoVar, Var, VarValue, const_var, context_var, expr_var, merge_var, var, var_from, var_getter, var_state, when_var,
    };

    pub use crate::var::animation::easing;

    pub use zng_layout::unit::{
        Align, AngleUnits as _, ByteUnits as _, DipToPx as _, FactorUnits as _, Layout1d as _, Layout2d as _, Length, LengthUnits as _,
        LineFromTuplesBuilder as _, PxToDip as _, RectFromTuplesBuilder as _, ResolutionUnits as _, TimeUnits as _,
    };

    pub use zng_txt::{ToTxt as _, Txt, formatx};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use zng_color::{LightDarkVarExt as _, MixAdjust as _, colors, hex, hsl, hsla, hsv, hsva, light_dark, rgb, rgba, web_colors};

    #[cfg(feature = "clipboard")]
    pub use zng_ext_clipboard::CLIPBOARD;

    #[cfg(feature = "config")]
    pub use zng_ext_config::CONFIG;

    pub use zng_ext_font::{FontStretch, FontStyle, FontWeight};

    #[cfg(feature = "image")]
    pub use zng_ext_image::ImageSource;

    #[cfg(feature = "image")]
    pub use zng_wgt_image::Image;

    pub use zng_ext_input::{
        focus::{FOCUS, WidgetInfoFocusExt as _, cmd::CommandFocusExt as _, iter::IterFocusableExt as _},
        gesture::{CommandShortcutMatchesExt as _, HeadlessAppGestureExt as _},
        keyboard::HeadlessAppKeyboardExt as _,
        mouse::WidgetInfoMouseExt as _,
    };

    pub use zng_ext_l10n::{L10N, l10n, lang};

    pub use zng_wgt_text::lang;

    #[cfg(feature = "undo")]
    pub use zng_ext_undo::{CommandUndoExt as _, REDO_CMD, UNDO, UNDO_CMD};

    #[cfg(feature = "window")]
    pub use zng_ext_window::{
        AppRunWindowExt as _, HeadlessAppWindowExt as _, WINDOW_Ext as _, WINDOWS, WidgetInfoImeArea as _, WindowCloseRequestedArgs,
        WindowIcon,
    };

    pub use zng_wgt::{CommandIconExt as _, ICONS, Wgt};

    pub use crate::text;
    pub use zng_wgt_text::Text;

    #[cfg(feature = "text_input")]
    pub use zng_wgt_text_input::{TextInput, selectable::SelectableText};

    #[cfg(feature = "window")]
    pub use crate::window;
    #[cfg(feature = "window")]
    pub use zng_wgt_window::Window;

    pub use zng_wgt_container::Container;

    #[cfg(feature = "button")]
    pub use zng_wgt_button::Button;

    #[cfg(feature = "data_context")]
    pub use zng_wgt_data::{DATA, data};

    #[cfg(feature = "grid")]
    pub use crate::grid;
    #[cfg(feature = "grid")]
    pub use zng_wgt_grid::Grid;

    pub use crate::layer;
    pub use zng_wgt_layer::{AnchorMode, LAYERS, LayerIndex};

    pub use crate::popup;
    pub use zng_wgt_layer::popup::POPUP;

    #[cfg(feature = "menu")]
    pub use crate::menu;
    #[cfg(feature = "menu")]
    pub use zng_wgt_menu::{
        Menu,
        context::{ContextMenu, context_menu, context_menu_fn},
        sub::SubMenu,
    };

    #[cfg(feature = "rule_line")]
    pub use zng_wgt_rule_line::hr::Hr;

    #[cfg(feature = "scroll")]
    pub use zng_wgt_scroll::{SCROLL, Scroll};

    #[cfg(feature = "toggle")]
    pub use crate::toggle;
    #[cfg(feature = "toggle")]
    pub use zng_wgt_toggle::Toggle;

    #[cfg(feature = "tooltip")]
    pub use crate::tip;
    #[cfg(feature = "tooltip")]
    pub use zng_wgt_tooltip::{Tip, tooltip, tooltip_fn};

    pub use zng_wgt::{
        WidgetFn,
        node::{VarPresent as _, VarPresentData as _, VarPresentList as _, VarPresentListFromIter as _, VarPresentOpt as _},
        wgt_fn,
    };

    pub use zng_wgt_style::{Style, style_fn};

    #[cfg(feature = "stack")]
    pub use zng_wgt_stack::{Stack, StackDirection};

    #[cfg(feature = "wrap")]
    pub use zng_wgt_wrap::Wrap;

    #[cfg(feature = "data_view")]
    pub use zng_wgt_data_view::{DataView, DataViewArgs};

    #[cfg(feature = "settings_editor")]
    pub use zng_wgt_settings::SettingBuilderEditorExt as _;

    #[cfg(feature = "dialog")]
    pub use crate::dialog;
    #[cfg(feature = "dialog")]
    pub use zng_wgt_dialog::DIALOG;
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
/// pub fn txt(txt: impl IntoVar<Txt>) {}
///
/// impl TextButton {
///     fn widget_intrinsic(&mut self) {
///         self.widget_builder().push_build_action(|b| {
///             let txt = b
///                 .capture_var::<Txt>(property_id!(Self::txt))
///                 .unwrap_or_else(|| const_var(Txt::from("")));
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
        DInstant, Deadline, INSTANT,
        event::{
            AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, CommandParam, Event, EventArgs as _,
            EventHandle, EventHandles, EventPropagationHandle, command, event, event_args,
        },
        handler::{AppHandler, WidgetHandler, app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once},
        render::{FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, SpatialFrameId, TransformStyle},
        shortcut::{CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts, shortcut},
        timer::{DeadlineHandle, DeadlineVar, TIMERS, TimerHandle, TimerVar},
        update::{EventUpdate, UPDATES, UpdateDeliveryList, UpdateOp, WidgetUpdates},
        widget::{
            AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _, WIDGET, WidgetId, WidgetUpdateMode,
            base::{WidgetBase, WidgetImpl},
            border::{BORDER, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle},
            builder::{NestGroup, WidgetBuilder, WidgetBuilding, property_id},
            easing,
            info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetMeasure, WidgetPath,
            },
            node::{
                ArcNode, ChainList, EditableUiVec, EditableUiVecRef, FillUiNode, IntoUiNode, PanelList, SORTING_LIST, SortingList, UiNode,
                UiNodeImpl, UiNodeListObserver, UiNodeOp, UiVec, ZIndex, match_node, match_node_leaf, match_widget, ui_vec,
            },
            property, widget, widget_impl, widget_mixin, widget_set,
        },
        window::{MonitorId, WINDOW, WindowId},
    };

    pub use zng_var::{
        ContextVar, IntoValue, IntoVar, ResponderVar, ResponseVar, Var, VarCapability, VarHandle, VarHandles, VarValue, const_var,
        context_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, var, var_getter, var_state, when_var,
    };

    pub use zng_layout::{
        context::{DIRECTION_VAR, LAYOUT, LayoutDirection, LayoutMetrics},
        unit::{
            Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Dip, DipBox, DipPoint, DipRect, DipSideOffsets,
            DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _, Layout1d as _,
            Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox, PxConstraints,
            PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform, PxVector, Rect,
            RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
        },
    };

    pub use zng_txt::{ToTxt as _, Txt, formatx};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use crate::task;

    pub use zng_app_context::{CaptureFilter, ContextLocal, ContextValueSet, LocalContext, RunOnDrop, app_local, context_local};

    pub use crate::state_map;
    pub use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, static_id};

    pub use zng_wgt::prelude::{IdEntry, IdMap, IdSet};

    pub use zng_wgt::{WidgetFn, wgt_fn};

    pub use zng_color::{
        ColorScheme, Hsla, Hsva, LightDark, MixAdjust as _, MixBlendMode, Rgba, colors, gradient, hex, hsl, hsla, hsv, hsva, light_dark,
        rgb, rgba, web_colors,
    };

    pub use zng_wgt::node::{
        VarPresent as _, VarPresentData as _, VarPresentList as _, VarPresentListFromIter, VarPresentOpt as _, bind_state, bind_state_init,
        border_node, command_property, event_property, event_state, event_state2, event_state3, event_state4, fill_node, list_presenter,
        list_presenter_from_iter, presenter, presenter_opt, widget_state_get_state, widget_state_is_state, with_context_blend,
        with_context_local, with_context_local_init, with_context_var, with_context_var_init, with_widget_state, with_widget_state_modify,
    };

    #[cfg(feature = "window")]
    pub use zng_ext_window::WidgetInfoBuilderImeArea as _;

    #[cfg(hot_reload)]
    pub use crate::hot_reload::hot_node;
}

mod defaults {
    use zng_app::{AppExtended, AppExtension};
    #[cfg(feature = "clipboard")]
    use zng_ext_clipboard::ClipboardManager;
    #[cfg(feature = "config")]
    use zng_ext_config::ConfigManager;
    use zng_ext_font::FontManager;
    #[cfg(feature = "fs_watcher")]
    use zng_ext_fs_watcher::FsWatcherManager;
    #[cfg(feature = "image")]
    use zng_ext_image::ImageManager;
    use zng_ext_input::{
        focus::FocusManager, gesture::GestureManager, keyboard::KeyboardManager, mouse::MouseManager,
        pointer_capture::PointerCaptureManager, touch::TouchManager,
    };

    #[cfg(feature = "drag_drop")]
    use zng_ext_input::drag_drop::DragDropManager;

    use zng_ext_l10n::L10nManager;
    #[cfg(feature = "undo")]
    use zng_ext_undo::UndoManager;

    #[cfg(feature = "window")]
    use zng_ext_window::WindowManager;

    use crate::default_editors;

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
        /// * [`FsWatcherManager`] if the `"fs_watcher"` feature is enabled.
        /// * [`ConfigManager`] if the `"config"` feature is enabled.
        /// * [`L10nManager`]
        /// * [`PointerCaptureManager`]
        /// * [`MouseManager`]
        /// * [`TouchManager`]
        /// * [`KeyboardManager`]
        /// * [`GestureManager`]
        /// * [`WindowManager`] if the `"window"` feature is enabled.
        /// * [`FontManager`]
        /// * [`FocusManager`]
        /// * [`DragDropManager`] if the `"drag_drop"` feature is enabled.
        /// * [`ImageManager`] if the `"image"` feature is enabled.
        /// * [`ClipboardManager`] if the `"clipboard"` feature is enabled.
        /// * [`UndoManager`]
        /// * [`SingleInstanceManager`] if the `"single_instance"` feature is enabled.
        /// * [`HotReloadManager`] if the `"hot_reload"` feature is enabled.
        /// * [`MaterialIconsManager`] if any `"material_icons*"` feature is enabled.
        /// * [`SvgManager`] if the `"svg"` feature is enabled.
        ///
        /// [`MaterialIconsManager`]: zng_wgt_material_icons::MaterialIconsManager
        /// [`SingleInstanceManager`]: zng_ext_single_instance::SingleInstanceManager
        /// [`HotReloadManager`]: zng_ext_hot_reload::HotReloadManager
        /// [`ConfigManager`]: zng_ext_config::ConfigManager
        /// [`L10nManager`]: zng_ext_l10n::L10nManager
        /// [`FontManager`]: zng_ext_font::FontManager
        /// [`SvgManager`]: zng_ext_svg::SvgManager
        pub fn defaults(&self) -> DefaultsAppExtended![] {
            let r = self.minimal();

            #[cfg(feature = "fs_watcher")]
            let r = r.extend(FsWatcherManager::default());

            #[cfg(feature = "config")]
            let r = r.extend(ConfigManager::default());

            let r = r.extend(L10nManager::default());

            let r = r.extend(PointerCaptureManager::default());

            let r = r.extend(MouseManager::default());

            let r = r.extend(TouchManager::default());

            let r = r.extend(KeyboardManager::default());

            let r = r.extend(GestureManager::default());

            #[cfg(feature = "window")]
            let r = r.extend(WindowManager::default());

            let r = r.extend(FontManager::default());

            let r = r.extend(FocusManager::default());

            #[cfg(feature = "drag_drop")]
            let r = r.extend(DragDropManager::default());

            #[cfg(feature = "image")]
            let r = r.extend(ImageManager::default());

            #[cfg(feature = "clipboard")]
            let r = r.extend(ClipboardManager::default());

            #[cfg(feature = "undo")]
            let r = r.extend(UndoManager::default());

            #[cfg(all(view, view_prebuilt))]
            tracing::debug!(r#"both "view" and "view_prebuilt" enabled, will use only one, indeterminate witch"#);

            #[cfg(single_instance)]
            let r = r.extend(zng_ext_single_instance::SingleInstanceManager::default());

            #[cfg(hot_reload)]
            let r = r.extend(zng_ext_hot_reload::HotReloadManager::default());

            #[cfg(any(
                feature = "material_icons_outlined",
                feature = "material_icons_filled",
                feature = "material_icons_rounded",
                feature = "material_icons_sharp",
            ))]
            let r = r.extend(zng_wgt_material_icons::MaterialIconsManager::default());

            #[cfg(feature = "svg")]
            let r = r.extend(zng_ext_svg::SvgManager::default());

            r.extend(DefaultsInit {})
        }
    }

    struct DefaultsInit {}
    impl AppExtension for DefaultsInit {
        fn init(&mut self) {
            // Common editors.
            zng_wgt::EDITORS.register_fallback(zng_wgt::WidgetFn::new(default_editors::handler));
            tracing::debug!("defaults init, var_editor set");

            // injected in all windows
            #[cfg(feature = "window")]
            {
                zng_ext_window::WINDOWS.register_root_extender(|a| {
                    let child = a.root;

                    #[cfg(feature = "inspector")]
                    let child = zng_wgt_inspector::inspector(child, zng_wgt_inspector::live_inspector(true));

                    child
                });
                tracing::debug!("defaults init, root_extender set");
            }
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                zng_ext_window::WINDOWS.register_open_nested_handler(crate::window::default_mobile_nested_open_handler);
                tracing::debug!("defaults init, open_nested_handler set");
            }

            // setup OPEN_LICENSES_CMD handler
            #[cfg(all(feature = "third_party_default", feature = "third_party"))]
            {
                crate::third_party::setup_default_view();
                tracing::debug!("defaults init, third_party set");
            }

            // setup SETTINGS_CMD handler
            #[cfg(feature = "settings_editor")]
            {
                zng_wgt_settings::handle_settings_cmd();
                tracing::debug!("defaults init, settings set");
            }

            #[cfg(all(single_instance, feature = "window"))]
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
        }

        fn deinit(&mut self) {
            // ensure zng_view_prebuilt is linked, macOS system linker can "optimize" the entire
            // crate away because it is only referenced by `linkme` in `on_process_start!`
            #[cfg(all(view_prebuilt, any(target_os = "macos", target_os = "ios")))]
            if std::env::var("=").is_ok() {
                crate::view_process::prebuilt::run_same_process(|| unreachable!());
            }
        }
    }
}

#[doc = include_str!("../../README.md")]
#[cfg(doctest)]
pub mod read_me_test {}

mod default_editors {
    use zng::{
        prelude::*,
        widget::{EditorRequestArgs, node::UiNode},
    };

    pub fn handler(args: EditorRequestArgs) -> UiNode {
        #[cfg(feature = "text_input")]
        if let Some(txt) = args.value::<Txt>() {
            return TextInput! {
                txt;
            };
        }
        #[cfg(feature = "text_input")]
        if let Some(s) = args.value::<String>() {
            return TextInput! {
                txt = s.map_bidi(|s| Txt::from_str(s), |t: &Txt| t.to_string());
            };
        }
        #[cfg(feature = "text_input")]
        if let Some(c) = args.value::<char>() {
            return TextInput! {
                txt_parse::<char> = c;
                style_fn = crate::text_input::FieldStyle!();
            };
        }

        #[cfg(feature = "toggle")]
        if let Some(checked) = args.value::<bool>() {
            return Toggle! {
                style_fn = toggle::CheckStyle!();
                checked;
            };
        }

        macro_rules! parse {
            ($($ty:ty),+ $(,)?) => {
                $(
                    #[cfg(feature = "text_input")]
                    if let Some(n) = args.value::<$ty>() {
                        return TextInput! {
                            txt_parse::<$ty> = n;
                            style_fn = crate::text_input::FieldStyle!();
                        };
                    }

                )+
            }
        }
        parse! { u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, f32, f64 }

        let _ = args;
        UiNode::nil()
    }
}
