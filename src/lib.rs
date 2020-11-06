#![warn(unused_extern_crates)]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```no_run
//! use zero_ui::prelude::*;
//!
//! fn main () {
//!     App::default().run_window(|_| {
//!         let size = var_from((800., 600.));
//!         let title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!         window! {
//!             size;
//!             title;
//!             content: example();
//!         }
//!     })
//! }
//!
//! fn example() -> impl UiNode {
//!     button! {
//!         on_click: |_,_| {
//!             println!("Button clicked!");
//!         };
//!         margin: 10.0;
//!         size: (300.0, 200.0);
//!         align: Alignment::CENTER;
//!         font_size: 28;
//!         content: text("Click Me!");
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! Zero-Ui apps are organized in a hierarchy of contexts that represents the lifetime of components.
//!
//! ```text
//! +----------------------------------------------------------------------+
//! | # Level 0 - App                                                      |
//! |                                                                      |
//! |  App, AppExtension, AppContext, AppServices, Events                  |
//! |                                                                      |
//! | +------------------------------------------------------------------+ |
//! | | # Level 1 - Window                                               | |
//! | |                                                                  | |
//! | |   Window, WindowContext, WindowServices, WindowId                | |
//! | |                                                                  | |
//! | | +--------------------------------------------------------------+ | |
//! | | | # Level 2 - Widget                                           | | |
//! | | |                                                              | | |
//! | | | UiNode, WidgetContext, widget!, #[property], WidgetId        | | |
//! | | |                                                              | | |
//! | | +--------------------------------------------------------------+ | |
//! | +------------------------------------------------------------------+ |
//! +----------------------------------------------------------------------+
//! ```
//!
//! ## Level 0 - App
//!
//! Components at this level live for the duration of the application, the root type is [`App`](crate::core::app::App).
//! An app is built from multiple extensions ([`AppExtension`](crate::core::app::AppExtension)) and then [`run`](crate::core::app::AppExtended::run).
//!
//! When the app is run, before the main loop starts, the extensions are [init](crate::core::app::AppExtension) with access to an especial context
//! [`AppInitContext`](crate::core::context::AppInitContext). [Services](#services) and [events](#services) can only be registered with
//! this context, they live for the duration of the application.
//!
//! After the app init, the main loop starts and the other extension methods are called with the [`AppContext`](crate::core::context::AppContext).
//!
//! ### Services
//!
//! Services are utilities that can be accessed by every component in every level, this includes [opening windows](crate::core::window::Windows)
//! and [shutting down](crate::core::app::AppProcess) the app it-self. All services implement [`AppService`](crate::core::service::AppService)
//! and can be requested from a [`AppServices`](crate::core::service::AppServices) that is provided by every context.
//!
//! ### Events
//!
//! Events are a list of [`EventArgs`](crate::core::event::EventArgs) that can be observed every update. New events can be generated from the
//! app extension methods or from other events. All events implement [`Event`](crate::core::event::Event) and a listener can be requested from
//! an [`Events`](crate::core::event::Events) that is provided by every context.
//!
//! Note that in [Level 2](#level-2-widget) events are abstracted further into a property that setups a listener and call a handler for every
//! event update.
//!
//! ## Level 1 - Window
//!
//! Components at this level live for the duration of a window instance. A window owns instances [window services](window-services)
//! and the root widget, it manages layout and rendering the widget tree.
//!
//! By default the [`WindowManager`](crate::core::window::WindowManager) extension sets-up the window contexts,
//! but that is not a requirement, you can implement your own *windows*.
//!
//! ### Window Services
//!
//! Services that require a [`WindowContext`](crate::core::context::WindowContext) to be instantiated. They live for the
//! duration of the window instance and every window has the same services. Window service builders must be registered only at
//! [Level 0](level-0-app) during the app initialization, the builders then are called during the window initialization to instantiate
//! the window services.
//!
//! These services can be requested from a [`WindowServices`](crate::core::context::WindowServices) that is provided by the window
//! and widget contexts.
//!
//! ## Level 2 - Widget
//!
//! The UI tree is composed of types that implement [`UiNode`](crate::core::UiNode), they can own one or more child nodes
//! that are also UI nodes, some special nodes introduce a new [`WidgetContext`](crate::core::context::WidgetContext) that
//! defines that sub-tree branch as a widget.
//!
//! The behavior and appearance of a widget is defined in these nodes, a widget
//! is usually composed of multiple nodes, one that defines the context, another that defines its unique behavior
//! plus more nodes introduced by [properties](#properties) that modify the widget.
//!
//! ### Properties
//!
//! A property is in essence a function that takes an UI node and some other arguments and returns a new UI node.
//! This is in fact the signature used for declaring one, see [`#[property]`](crate::core::property) for more details.
//!
//! Multiple properties are grouped together to form a widget.
//!
//! ### Widgets
//!
//! A widget is a bundle of preset properties plus two optional functions, see [`widget!`](crate::core::widget) for more details.
//!
//! During widget instantiation an UI node tree is build by feeding each property node to a subsequent property, in a debug build
//! inspector nodes are inserted also, to monitor the properties. You can press `CTRL+SHIT+I` to inspect a window.
//!
//! The widget root node introduces a new [`WidgetContext`](crate::core::context::WidgetContext) that can be accessed by all
//! widget properties.
//!
//! # State
//!
//! TODO how to keep state, and contextual states.
//!
//! ## Variables
//!
//! TODO
//!
//! # Updates
//!
//! TODO how the UI is updated.
//!
//! # Async
//!
//! TODO how to run async tasks that interact back with the UI.
//!
//! # Lifecycle Overview
//!
//!
//! ```text
//! +------------------------------------+
//! | # Setup                            |
//! |                  ↓↑                |
//! | App::default().extend(CustomExt)   |
//! |      ::empty()                     |
//! +------------------------------------+
//!    |
//!    | .run(|ctx: &mut AppContext| { .. })
//!    | .run_window(|ctx: &mut AppContext| { window! { .. } })
//!    ↓
//! +---------------------------------------+
//! | # Run                                 |
//! |                                       |
//! | services.register(AppProcess)         |
//! |    |                                  |
//! |    ↓            ↓↑                    |
//! | AppExtension::init(AppInitContext)    |
//! |    |                                  |
//! |    ↓     ↓OS  ↓timer  ↓UpdateNotifier |
//! | +---------------------------------------------+
//! | | # EventLoop                                 |
//! | |                                             |
//! | |  AppExtension ↓↑                            |
//! | |      ::on_window_event(..)                  |
//! | |      ::on_device_event(..)                  |
//! | |      ::on_new_frame_ready(..)               |
//! | |   |                                         |
//! | |   ↓      ↓update                            |
//! | | +-----------------------------------------------+
//! | | | # Updates                                     |
//! | | |                                               |
//! | | |   ↓↑ sync - pending assign, notify requests   |
//! | | |   ↓↑ vars - setup new values                  |
//! | | |   ↓↑ events - setup update arguments          |
//! | | |   ↓                                           |
//! | | |   if any -> AppExtension::update(..) ↑        |
//! | | |   |            UiNode::update(..)             |
//! | | |   |            UiNode::update_hp(..)          |
//! | | |   |               event handlers              |
//! | | |   ↓                                           |
//! | | +-----------------------------------------------+
//! | |     ↓                                        |
//! | | +-----------------------------------------------+
//! | | | # Layout/Render                               |
//! | | |                                               |
//! | | | AppExtension::update_display(..)              |
//! | | |           UiNode::measure(..)                 |
//! | | |           UiNode::arrange(..)                 |
//! | | |           UiNode::render(..)                  |
//! | | |           UiNode::render_update(..)           |
//! | | +-----------------------------------------------+
//! | |     ↓                                       |
//! | |   EventLoop                                 |
//! | +---------------------------------------------+
//! |   | AppProcess::shutdown()            |
//! |   ↓                                   |
//! |   0                                   |
//! +---------------------------------------+
//! ```

// for proc_macros that don't have $self.
extern crate self as zero_ui;

#[macro_use]
extern crate bitflags;

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
#[allow(unused)]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

/// Calls `eprintln!("warning: {}", format_args!($))` with `warning` colored bright yellow and bold.
#[allow(unused)]
macro_rules! warn_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "warning".bright_yellow().bold(), format_args!($($tt)*))
    }}
}

/// Declare a new unique id type.
macro_rules! unique_id {
    ($(#[$docs:meta])* $vis:vis struct $Type:ident;) => {

        $(#[$docs])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU64);

        impl $Type {
            fn next() -> &'static std::sync::atomic::AtomicU64 {
                use std::sync::atomic::AtomicU64;
                static NEXT: AtomicU64 = AtomicU64::new(1);
                &NEXT
            }

            /// Generates a new unique ID.
            ///
            /// # Panics
            /// Panics if called more then `u64::MAX` times.
            pub fn new_unique() -> Self {
                use std::sync::atomic::Ordering;

                let id = Self::next().fetch_add(1, Ordering::Relaxed);

                if let Some(id) = std::num::NonZeroU64::new(id) {
                    $Type(id)
                } else {
                    Self::next().store(0, Ordering::SeqCst);
                    panic!("`{}` reached `u64::MAX` IDs.", stringify!($Type))
                }
            }

            /// Retrieve the underlying `u64` value.
            #[allow(dead_code)]
            #[inline]
            pub fn get(self) -> u64 {
                self.0.get()
            }

            /// Creates an id from a raw value.
            ///
            /// # Safety
            ///
            /// This is only safe if called with a value provided by [`get`](Self::get).
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u64) -> $Type {
                $Type(std::num::NonZeroU64::new_unchecked(raw))
            }

            /// Creates an id from a raw value.
            ///
            /// Checks if `raw` is in the range of generated widgets.
            #[inline]
            #[allow(dead_code)]
            pub fn new(raw: u64) -> Option<$Type> {
                use std::sync::atomic::Ordering;

                if raw >= 1 && raw < Self::next().load(Ordering::Relaxed) {
                    // SAFETY: we just validated raw.
                    Some(unsafe { Self::from_raw(raw) })
                } else {
                    None
                }
            }
        }
    };
}

/// Implements From and IntoVar without boilerplate.
macro_rules! impl_from_and_into_var {
    ($(
        $(#[$docs:meta])*
        fn from($name:ident : $From:ty) -> $To:ty {
            $convert:expr
        }
    )+) => {
        $(
            impl From<$From> for $To {
                $(#[$docs])*
                #[inline]
                fn from($name: $From) -> Self {
                    $convert
                }
            }

            impl $crate::core::var::IntoVar<$To> for $From {
                type Var = $crate::core::var::OwnedVar<$To>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    $crate::core::var::OwnedVar(self.into())
                }
            }
        )+
    };
}

/// Generates a type that can only have a single instance at a time.
macro_rules! singleton_assert {
    ($Singleton:ident) => {
        struct $Singleton {}

        impl $Singleton {
            fn flag() -> &'static std::sync::atomic::AtomicBool {
                static ALIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                &ALIVE
            }

            pub fn assert_new() -> Self {
                if Self::flag().load(std::sync::atomic::Ordering::Acquire) {
                    panic!("only a single instance of `{}` can exist at at time", stringify!($Singleton))
                }

                Self::flag().store(true, std::sync::atomic::Ordering::Release);

                $Singleton {}
            }
        }

        impl Drop for $Singleton {
            fn drop(&mut self) {
                Self::flag().store(false, std::sync::atomic::Ordering::Release);
            }
        }
    };
}

#[doc(hidden)]
pub use zero_ui_macros::{widget_new, widget_stage2, widget_stage3};

pub mod core;
pub mod properties;
pub mod widgets;

/// All the types you need to build an app.
///
/// Use glob import (`*`) to quickly start implementing an application.
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(|_| {
///     todo!()
/// })
/// ```
///
/// # Other Preludes
///
/// There are prelude modules for other contexts, [`new_property`](crate::prelude::new_property) for
/// new properties, [`new_widget`](crate::prelude::new_widget) for creating widgets.
pub mod prelude {
    pub use crate::core::{
        app::App,
        color::{
            self, blur, brightness, contrast, drop_shadow, grayscale, hex, hsl, hsla, hue_rotate, opacity, rgb, rgba, saturate, sepia,
            web_colors, Rgba,
        },
        context::WidgetContext,
        service::{AppServices, WindowServices},
        focus::{DirectionalNav, TabIndex, TabNav},
        gesture::shortcut,
        render::WidgetPath,
        sync::Sync,
        text::{
            font_features::{
                CapsVariant, CharVariant, CnVariant, EastAsianWidth, FontPosition, FontStyleSet, JpVariant, NumFraction, NumSpacing,
                NumVariant,
            },
            formatx, FontFeatures, FontName, FontStretch, FontStyle, FontWeight, Fonts, Hyphens, LineBreak, Text, TextAlign,
            TextTransformFn, WhiteSpace, WordBreak,
        },
        types::{BorderRadius, ElementState, ModifiersState, MouseButton, VirtualKeyCode},
        ui_vec,
        units::{
            rotate, skew, translate, Alignment, AngleUnits, FactorUnits, Length, LengthUnits, LineHeight, Point, Rect, SideOffsets, Size,
            TimeUnits,
        },
        var::{merge_var, state_var, switch_var, var, var_from, RcVar, Var, VarObj, Vars},
        window::{AppRunWindow, CursorIcon, Window, Windows},
        UiNode, UiVec, Widget, WidgetId,
    };

    pub use crate::properties::*;
    pub use crate::widgets::*;

    pub use crate::properties::background::{background, *};
    pub use crate::properties::events::*;
    pub use crate::properties::filters::*;
    pub use crate::properties::focus::*;
    pub use crate::properties::foreground::{foreground, *};
    pub use crate::properties::size::{size, *};
    pub use crate::properties::states::*;
    pub use crate::properties::text_theme::{
        font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
        text_transform, word_spacing,
    };
    pub use crate::properties::transform::{transform, *};

    pub use crate::widgets::layouts::*;
    pub use crate::widgets::text::{text, *};

    /// All the types you need to declare a new property.
    ///
    /// Use glob import (`*`) to quickly start implementing properties.
    pub mod new_property {
        pub use crate::core::color::{self, *};
        pub use crate::core::context::*;
        pub use crate::core::event::*;
        pub use crate::core::gesture::*;
        pub use crate::core::render::*;
        pub use crate::core::text::Text;
        pub use crate::core::types::*;
        pub use crate::core::units::{self, *};
        pub use crate::core::var::*;
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, property, ui_vec, FillUiNode, UiNode, UiVec, Widget, WidgetId, LAYOUT_ANY_SIZE,
        };
        pub use crate::properties::{
            events::{on_event, on_event_filtered, on_preview_event, on_preview_event_filtered},
            set_widget_state, with_context_var,
        };
    }

    /// All the types you need to declare a new widget or widget mix-in.
    ///
    /// Use glob import (`*`) to quickly start implementing widgets.
    pub mod new_widget {
        pub use crate::core::color::*;
        pub use crate::core::context::*;
        pub use crate::core::render::*;
        pub use crate::core::text::*;
        pub use crate::core::types::*;
        pub use crate::core::units::*;
        pub use crate::core::var::*;
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, ui_vec, widget, widget_mixin, FillUiNode, UiNode, UiVec, Widget, WidgetId, LAYOUT_ANY_SIZE,
        };
        pub use crate::properties::background::{background, *};
        pub use crate::properties::capture_only::*;
        pub use crate::properties::events::*;
        pub use crate::properties::filters::*;
        pub use crate::properties::focus::*;
        pub use crate::properties::foreground::{foreground, *};
        pub use crate::properties::size::{size, *};
        pub use crate::properties::states::*;
        pub use crate::properties::text_theme::{
            font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
            text_transform, word_spacing,
        };
        pub use crate::properties::transform::{transform, *};
        pub use crate::properties::*;
        pub use crate::widgets::container;
        pub use crate::widgets::mixins::*;
    }
}
