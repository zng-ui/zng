#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]
// suppress nag about very simple boxed closure signatures.
#![allow(clippy::type_complexity)]

//! Zero-Ui is a GUI framework.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! zero-ui-view = "0.1"
//! ```
//!
//! Then create your first window:
//!
//! ```no_run
//! # mod zero_ui_view { pub fn init() { } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     zero_ui_view::init();
//!
//!     App::default().run_window(async {
//!         let size = var_from((800, 600));
//!         window! {
//!             title = size.map(|s: &Size| formatx!("Button Example - {s}"));
//!             size;
//!             child = button! {
//!                 on_click = hn!(|_| {
//!                     println!("Button clicked!");
//!                 });
//!                 margin = 10;
//!                 size = (300, 200);
//!                 align = Align::CENTER;
//!                 font_size = 28;
//!                 child = text!("Click Me!");
//!             }
//!         }
//!     })
//! }
//! ```
//!
//! # Vars
//!
//! TODO
//!
//! # Events
//!
//! TODO
//!
//! ## Routes
//!
//! TODO
//!
//! # Contexts
//!
//! TODO
//!
//! # Tasks
//!
//! TODO

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;

#[allow(unused_imports)]
#[macro_use]
extern crate bitflags;

#[doc(no_inline)]
pub use zero_ui_core as core;

pub(crate) mod crate_util;
pub mod properties;
pub mod widgets;

/// All the types you need to start building an app.
///
/// Use glob import (`*`) and start implementing your app.
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(async {
///     // ..
/// # unimplemented!()
/// })
/// ```
///
/// # Other Preludes
///
/// There are prelude modules for other contexts, [`new_property`] for
/// creating new properties, [`new_widget`] for creating new widgets.
///
/// The [`rayon`] crate's prelude is inlined in the preludes.
///
/// [`new_property`]: crate::prelude::new_property
/// [`new_widget`]: crate::prelude::new_widget
/// [`rayon`]: https://docs.rs/rayon
pub mod prelude {
    #[cfg(feature = "http")]
    #[doc(no_inline)]
    pub use crate::core::task::http::Uri;

    #[doc(no_inline)]
    pub use crate::core::{
        app::App,
        async_clmv,
        border::{BorderSides, BorderStyle, LineOrientation, LineStyle},
        clmv,
        color::{self, color_scheme_map, colors, filters, hex, hsl, hsla, rgb, rgba, ColorScheme, Rgba},
        context::{LayoutDirection, WIDGET, WINDOW},
        event::{AnyEventArgs, Command, CommandArgs, CommandInfoExt, CommandNameExt, CommandScope, EventArgs, EVENTS},
        focus::{DirectionalNav, FocusChangedArgs, ReturnFocusChangedArgs, TabIndex, TabNav, FOCUS},
        gesture::{shortcut, ClickArgs, CommandShortcutExt, GestureKey, Shortcut, ShortcutArgs, Shortcuts},
        gradient::{stops, ExtendMode, GradientStop, GradientStops},
        handler::*,
        image::ImageSource,
        keyboard::{CharInputArgs, Key, KeyInputArgs, KeyState, ModifiersChangedArgs, ModifiersState},
        mouse::{ButtonState, ClickMode, MouseButton, MouseMoveArgs},
        render::RenderMode,
        task::{self, rayon::prelude::*},
        text::{
            font_features::{
                CapsVariant, CharVariant, CnVariant, EastAsianWidth, FontPosition, FontStyleSet, JpVariant, NumFraction, NumSpacing,
                NumVariant,
            },
            formatx, lang, FontFeatures, FontName, FontNames, FontStretch, FontStyle, FontWeight, Hyphens, Justify, LineBreak, Text,
            TextTransformFn, ToText, UnderlinePosition, UnderlineSkip, WhiteSpace, WordBreak, FONTS,
        },
        timer::TIMERS,
        units::{
            rotate, scale, scale_x, scale_xy, scale_y, skew, skew_x, skew_y, translate, translate_x, translate_y, Align, AngleUnits,
            ByteUnits, EasingStep, EasingTime, FactorUnits, Length, LengthUnits, Line, LineFromTuplesBuilder, LineHeight, Point, Px,
            PxPoint, PxSize, Rect, RectFromTuplesBuilder, SideOffsets, Size, TimeUnits, Transform, Vector,
        },
        var::{
            animation::{self, easing},
            expr_var, merge_var, state_var, var, var_default, var_from, AnyVar, ArcVar, IntoVar, Var, VarReceiver, VarSender, VarValue,
            VARS,
        },
        widget_base::HitTestMode,
        widget_info::{InteractionPath, Visibility, WidgetPath},
        widget_instance::{
            ui_vec, z_index, ArcNode, EditableUiNodeList, EditableUiNodeListRef, FillUiNode, NilUiNode, UiNode, UiNodeList,
            UiNodeListChain, UiNodeVec, WidgetId, ZIndex,
        },
        window::{
            AppRunWindowExt, AutoSize, CursorIcon, FocusIndicator, HeadlessAppWindowExt, MonitorId, MonitorQuery, StartPosition, Window,
            WindowChangedArgs, WindowChrome, WindowCloseRequestedArgs, WindowIcon, WindowId, WindowOpenArgs, WindowState, WindowVars,
            WINDOWS, WINDOW_CTRL,
        },
    };

    #[doc(no_inline)]
    pub use crate::properties::*;
    #[doc(no_inline)]
    pub use crate::widgets::*;

    #[doc(no_inline)]
    pub use crate::properties::commands::*;
    #[doc(no_inline)]
    pub use crate::properties::events::{gesture::*, keyboard::*, mouse::on_mouse_move, widget::on_move};
    #[doc(no_inline)]
    pub use crate::properties::filters::*;
    #[doc(no_inline)]
    pub use crate::properties::focus::*;
    #[doc(no_inline)]
    pub use crate::properties::states::*;
    #[doc(no_inline)]
    pub use crate::properties::transform::{transform, *};
    #[doc(no_inline)]
    pub use crate::widgets::text::{
        direction, font_family, font_size, font_stretch, font_style, font_weight, lang, letter_spacing, line_height, tab_length, txt_align,
        txt_color, txt_transform, word_spacing, TEXT_COLOR_VAR,
    };

    #[doc(no_inline)]
    pub use crate::widgets::image::ImageFit;
    #[doc(no_inline)]
    pub use crate::widgets::layouts::{stack::StackDirection, *};
    #[doc(no_inline)]
    pub use crate::widgets::scroll::ScrollMode;
    #[doc(no_inline)]
    pub use crate::widgets::style::style_fn;
    #[doc(no_inline)]
    pub use crate::widgets::window::{AnchorMode, AnchorOffset, LayerIndex, LAYERS};

    /// All the types you need to declare a new property.
    ///
    /// Use glob import (`*`) and start implement your custom properties.
    ///
    /// ```
    /// # fn main() {}
    /// use zero_ui::prelude::new_property::*;
    ///
    /// #[property(CONTEXT)]
    /// pub fn my_property(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     MyPropertyNode { child, value: value.into_var() }
    /// }
    ///
    /// #[ui_node(struct MyPropertyNode {
    ///     child: impl UiNode,
    ///     #[var] value: impl Var<bool>,
    /// })]
    /// impl UiNode for MyPropertyNode {
    ///     fn update(&mut self, updates: &WidgetUpdates) {
    ///         self.child.update(updates);
    ///         if let Some(new_value) = self.value.get_new() {
    ///             // ..
    ///         }
    ///     }
    /// }
    /// ```
    pub mod new_property {
        #[doc(no_inline)]
        pub use crate::core::border::*;
        #[doc(no_inline)]
        pub use crate::core::color::{self, *};
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::gesture::*;
        #[doc(no_inline)]
        pub use crate::core::handler::*;
        #[doc(no_inline)]
        pub use crate::core::keyboard::KeyState;
        #[doc(no_inline)]
        pub use crate::core::mouse::ButtonState;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::task::{self, rayon::prelude::*, ui::UiTask};
        #[doc(no_inline)]
        pub use crate::core::text::Text;
        #[doc(no_inline)]
        pub use crate::core::units::{self, *};
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::widget_base::HitTestMode;
        #[doc(no_inline)]
        pub use crate::core::window::{WindowId, INTERACTIVITY_CHANGED_EVENT};
        #[doc(no_inline)]
        pub use crate::core::{
            property, ui_node, widget, widget_base,
            widget_base::nodes::interactive_node,
            widget_info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout,
                WidgetMeasure,
            },
            widget_instance::{
                ui_vec, BoxedUiNode, EditableUiNodeList, EditableUiNodeListRef, FillUiNode, NilUiNode, SortingList, SortingListParent,
                UiNode, UiNodeList, UiNodeListChain, UiNodeListObserver, UiNodeVec, WidgetId,
            },
        };
        #[doc(no_inline)]
        pub use crate::widgets::{layouts::stack_nodes, wgt_fn, DataUpdate, WidgetFn};
    }

    /// All the types you need to declare a new widget or widget mix-in.
    ///
    /// Use glob import (`*`) and start implement your custom widgets.
    ///
    /// ```
    /// # fn main() { }
    /// use zero_ui::prelude::new_widget::*;
    ///
    /// #[widget($crate::my_widget)]
    /// pub mod my_widget {
    ///     use super::*;
    ///
    ///     inherit!(widget_base::base);
    ///
    ///     properties! {
    ///         background_color = colors::BLUE;
    ///     }
    /// }
    /// ```
    pub mod new_widget {
        #[doc(no_inline)]
        pub use crate::core::border::*;
        #[doc(no_inline)]
        pub use crate::core::color::*;
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::handler::*;
        #[doc(no_inline)]
        pub use crate::core::image::Image;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::task::{self, rayon::prelude::*, ui::UiTask};
        #[doc(no_inline)]
        pub use crate::core::text::*;
        #[doc(no_inline)]
        pub use crate::core::units::*;
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::widget_builder::*;
        #[doc(no_inline)]
        pub use crate::core::window::{CursorIcon, WindowId, INTERACTIVITY_CHANGED_EVENT};
        #[doc(no_inline)]
        pub use crate::core::{
            property, ui_node, widget,
            widget_base::{self, HitTestMode},
            widget_info::{
                InlineSegment, InlineSegmentInfo, InlineSegmentPos, InteractionPath, Interactivity, Visibility, WidgetBorderInfo,
                WidgetBoundsInfo, WidgetInfoBuilder, WidgetInlineMeasure, WidgetLayout, WidgetMeasure,
            },
            widget_instance::{
                ui_vec, z_index, AdoptiveNode, BoxedUiNode, BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, FillUiNode,
                NilUiNode, PanelList, SortingList, SortingListParent, UiNode, UiNodeList, UiNodeListChain, UiNodeListObserver, UiNodeVec,
                WidgetId, ZIndex,
            },
            widget_mixin,
        };
        #[doc(no_inline)]
        pub use crate::properties::events::{self, gesture::*, keyboard::*};
        #[doc(no_inline)]
        pub use crate::properties::filters::*;
        #[doc(no_inline)]
        pub use crate::properties::focus::focusable;
        #[doc(no_inline)]
        pub use crate::properties::focus::*;
        #[doc(no_inline)]
        pub use crate::properties::states::*;
        #[doc(no_inline)]
        pub use crate::properties::transform::{transform, *};
        #[doc(no_inline)]
        pub use crate::properties::*;
        #[doc(no_inline)]
        pub use crate::widgets::mixins::*;
        #[doc(no_inline)]
        pub use crate::widgets::text::{
            font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, txt_align, txt_color,
            txt_transform, word_spacing,
        };
        #[doc(no_inline)]
        pub use crate::widgets::{
            container,
            layouts::{stack_nodes, stack_nodes_layout_by},
            mixins::style_mixin,
            style,
            style::{style_fn, StyleFn},
            wgt_fn, DataUpdate, WidgetFn,
        };
    }
}

/// Standalone documentation.
///
/// This module contains empty modules that hold *integration docs*, that is
/// documentation that cannot really be associated with API items because they encompass
/// multiple items.
pub mod docs {
    /// `README.md`
    ///
    #[doc = include_str!("../../README.md")]
    pub mod readme {}

    /// `CHANGELOG.md`
    ///
    #[doc = include_str!("../../CHANGELOG.md")]
    pub mod changelog {}
}

// see test-crates/no-direct-deps
#[doc(hidden)]
pub fn crate_reference_called() -> bool {
    true
}
#[doc(hidden)]
#[macro_export]
macro_rules! crate_reference_call {
    () => {
        $crate::crate_reference_called()
    };
}

#[allow(missing_docs)]
pub mod test {
    use std::{ops::{Deref, DerefMut}, cell::RefCell};

    use zero_ui_core::{
        gesture::ClickArgs,
        handler::WidgetHandler,
        hn,
        widget_instance::{BoxedUiNode, NilUiNode},
    };

    use crate::prelude::{
        new_widget::{Importance, IntoValue, WidgetBuilder},
        IntoVar, Size, UiNode, WidgetId,
    };

    pub trait on_click: Sized {
        fn on_click(&mut self, handler: impl WidgetHandler<ClickArgs>);
    }
    impl on_click for WidgetBase {
        fn on_click(&mut self, handler: impl WidgetHandler<ClickArgs>) {
            self.push_property(())
        }
    }

    pub struct WidgetBase {
        pub builder: RefCell<WidgetBuilder>,
    }
    impl WidgetBase {
        pub fn new() -> Self {
            todo!()
        }

        pub fn into_builder(self) -> WidgetBuilder {
            self.builder.into_inner()
        }

        pub fn build(self) -> BoxedUiNode {
            self.into_builder().build()
        }

        /// Widget property are `&self`, extension properties are `&mut self`, to highlight on the widget macro.
        pub fn push_property(&self, todo: ()) {
            let _todo = self.builder.borrow_mut();
        }
        
        pub fn begin_when(&mut self, condition: impl IntoVar<bool>) {
            todo!("retarget `push_property` to a when builder")
        }

        pub fn end_when(&mut self) {
            todo!("finish the when builder");
        }
    }
    impl WidgetBase {
        // #[property(WIDGET)] // attribute generates metadata methods in place when set on a method, otherwise generates a trait.
        /// Widget unique ID.
        pub fn id(&self, id: impl Into<WidgetId>) {
            self.push_property(());
        }
    }

    pub struct Container {
        base: WidgetBase,
    }
    impl Deref for Container {
        type Target = WidgetBase;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }
    impl DerefMut for Container {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }
    impl Container {
        pub fn new() -> Self {
            Self { base: WidgetBase::new() }
        }

        pub fn into_builder(self) -> WidgetBuilder {
            self.base.into_builder()
        }

        pub fn build(self) -> BoxedUiNode {
            self.base.build()
        }
    }
    impl Container {        
        /// Child widget.
        pub fn child(&self, child: impl UiNode) {
            todo!()
        }
    }

    pub struct Button {
        base: Container,
    }
    impl Deref for Button {
        type Target = Container;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }
    impl DerefMut for Button {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }
    impl Button {
        pub fn new() -> Self {
            todo!()
        }

        pub fn into_builder(self) -> WidgetBuilder {
            self.base.into_builder()
        }

        pub fn build(self) -> BoxedUiNode {
            self.base.build()
        }
    }
    impl Button {
        #[doc(hidden)]
        pub fn __on_click_info__(&self) {
            todo!()
        }

        /// Special docs for `button::on_click`.
        pub fn on_click(&self, handler: impl WidgetHandler<ClickArgs>) {
            todo!()
        }
    }

    #[doc(hidden)]
    pub trait size: Sized {
        /// Size property.
        fn size(&mut self, size: impl IntoVar<Size>);
    }
    impl size for WidgetBase {
        fn size(&mut self, size: impl IntoVar<Size>) {
            todo!()
        }
    }
    /// Size property.
    pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
        child
    }

    pub trait background_gradient: Sized {
        fn background_gradient(&mut self, axis: impl IntoVar<()>, stops: impl IntoVar<()>);

        /// Names inputs expand to this.
        #[doc(hidden)]
        fn __background_gradient_sorted__(&mut self, axis: impl IntoVar<()>, stops: impl IntoVar<()>) {
            self.background_gradient(axis, stops);
        }
    }
    impl background_gradient for WidgetBase {
        fn background_gradient(&mut self, axis: impl IntoVar<()>, stops: impl IntoVar<()>) {
            todo!()
        }
    }

    #[macro_export]
    macro_rules! wgt_properties {
        ($init:expr => $($property:ident = $($arg:expr),+);* $(;)?) => {
            {
                let mut wgt = $init;
                $(
                    wgt.$property($($arg),*);
                )*
                wgt.build()
            }
        };
        (@stmt { $wgt:ident => $property:ident = $($arg:expr),+ }) => {
            $wgt.$property($($arg),+);
        };
        (@stmt { $wgt:ident => $property:ident = { $( $member:ident : $arg:expr ),+ $(,)? } }) => {
            // proc-macro that sorts members.
            $crate::named_property_init! {
                $wgt.$property = {
                    $($member : $arg),+
                }
            }
        };
        (@stmt { $wgt:ident => $property:ident }) => {
            $wgt.$property($property);
        };
        (@stmt { $wgt:ident => when $($tt:tt)* }) => {
            $wgt.begin_when($condition); // proc-macro parses then when.
            $wgt.end_when();
        }
    }

    #[macro_export]
    macro_rules! Button {
        ($($tt:tt)*) => {
            $crate::wgt_properties! {
                $crate::test::Button::new() => $($tt)*
            }
        }
    }
}

pub mod other_test {
    use zero_ui_core::{units::Size, var::IntoVar};

    use crate::test::WidgetBase;

    #[doc(hidden)]
    pub trait min_size: Sized {
        fn __min_size_info__() -> min_size_meta {
            todo!()
        }

        /// Docs.
        fn min_size(&mut self, min_size: impl IntoVar<Size>);
    }
    impl min_size for WidgetBase {
        fn min_size(&mut self, min_size: impl IntoVar<Size>) {
            todo!()
        }
    }

    pub struct min_size_meta {}

    impl min_size_meta {
        pub fn id(self, name: &'static str) {}
    }

    pub fn read_meta() {
        let id = <WidgetBase as min_size>::__min_size_info__().id("lol");
    }
}

fn test() {
    use test::*;

    let mut btn = Button::new();
    btn.id("btn");
    btn.child(core::widget_instance::NilUiNode);
    btn.on_click(core::handler::hn!(|_| {}));
    btn.size(50);
    let btn = btn.build();

    // Widget property are `&self`, extension properties are `&mut self`, to highlight on the widget macro.
    let btn = Button! {
        id = "btn";
        child = core::widget_instance::NilUiNode;
        on_click = core::handler::hn!(|_| {});
        // min_size = 40;
        size = 50;
        background_gradient = (), ();
        // background_gradient = {
        //     axis: (),
        //     stops: (),
        // };
    };
}

// Open questions:
//
// * Path properties `properties::size = 50;`. Is just a localized use?
//     - Right now we can do `window::size = 10` to get the window property.
//     - Because window size is directly associated with the type there is no need to this.
// * Widget modules that contain stuff (like `text::TXT_COLOR_VAR`).
//     - Can still have it, but negates `Text! { text = ""; }` name collision.
//     - Maybe we can have the `text_wgt` suffix?
// * Mix-ins
//     - They are a `#[widget] trait FocusableMixin: WidgetProperties { }`, then Button can manually `impl Focusable for Button`.
//
// Features:
//
// * Extension properties can select what widget they work on, by default impl for `WidgetBase`, but can be limited to an
//   widget and descendants.