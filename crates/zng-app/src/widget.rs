//! Widget, UI node API.

pub mod base;
pub mod border;
pub mod builder;
pub mod info;
pub mod inspector;
pub mod node;

mod easing;
pub use easing::*;

use atomic::Atomic;
use parking_lot::{Mutex, RwLock};
use std::{
    borrow::Cow,
    pin::Pin,
    sync::{Arc, atomic::Ordering::Relaxed},
};
use zng_app_context::context_local;
use zng_clone_move::clmv;
use zng_handle::Handle;
use zng_layout::unit::{DipPoint, DipToPx as _, Layout1d, Layout2d, Px, PxPoint, PxTransform};
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, StateValue};
use zng_task::UiTask;
use zng_txt::{Txt, formatx};
use zng_var::{AnyVar, BoxAnyVarValue, ResponseVar, Var, VarHandle, VarHandles, VarValue};
use zng_view_api::display_list::ReuseRange;

use crate::{
    event::{Event, EventArgs, EventHandle, EventHandles},
    handler::{APP_HANDLER, AppWeakHandle, Handler, HandlerExt as _, HandlerResult},
    update::{LayoutUpdates, RenderUpdates, UPDATES, UpdateFlags, UpdateOp, UpdatesTrace},
    window::WINDOW,
};

use self::info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo};

// proc-macros used internally during widget creation.
#[doc(hidden)]
pub use zng_app_proc_macros::{property_impl, property_meta, widget_new};

pub use zng_app_proc_macros::{property, widget, widget_mixin};

/// <span data-del-macro-root></span> Sets properties and when condition on a widget builder.
///
/// # Examples
///
/// ```
/// # use zng_app::{*, widget::{base::*, node::*, widget, property}};
/// # use zng_var::*;
/// # #[property(CONTEXT)] pub fn enabled(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode { child.into_node() }
/// # #[widget($crate::Wgt)]
/// # pub struct Wgt(WidgetBase);
/// # fn main() {
/// # let flag = true;
/// #
/// let mut wgt = Wgt::widget_new();
///
/// if flag {
///     widget_set! {
///         &mut wgt;
///         enabled = false;
///     }
/// }
///
/// widget_set! {
///     &mut wgt;
///     id = "wgt";
/// }
///
/// let wgt = wgt.widget_build();
/// # }
/// ```
///
/// In the example above the widget will always build with custom `id`, but only will set `enabled = false` when `flag` is `true`.
///
/// Note that properties are designed to have a default *neutral* value that behaves as if unset, in the example case you could more easily write:
///
/// ```
/// # zng_app::enable_widget_macros!();
/// # use zng_app::{*, widget::{node::*, base::*, widget, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # #[widget($crate::Wgt)] pub struct Wgt(WidgetBase);
/// # #[property(CONTEXT)] pub fn enabled(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let flag = true;
/// let wgt = Wgt! {
///     enabled = !flag;
///     id = "wgt";
/// };
/// # }
/// ```
///
/// You should use this macro only in contexts where a widget will be build in steps, or in very hot code paths where a widget
/// has many properties and only some will be non-default per instance.
///
/// # Property Assign
///
/// Properties can be assigned using the `property = value;` syntax, this expands to a call to the property method, either
/// directly implemented on the widget or from a trait.
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// id = "name";
/// background_color = colors::BLUE;
/// # }; }
/// ```
///
/// The example above is equivalent to:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let mut wgt = zng_app::widget::base::WidgetBase::widget_new();
/// wgt.id("name");
/// wgt.background_color(colors::BLUE);
/// # }
/// ```
///
/// Note that `id` is an intrinsic property inherited from [`WidgetBase`], but `background_color` is an extension property declared
/// by a [`property`] function. Extension properties require `&mut self` access to the widget, intrinsic properties only require `&self`,
/// this is done so that IDEs that use a different style for mutable methods highlight the properties that are not intrinsic to the widget.
///
/// ## Path Assign
///
/// A full or partial path can be used to specify exactly what extension property will be set:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// self::background_color = colors::BLUE;
/// # }; }
/// ```
///
/// In the example above `self::background_color` specify that an extension property that is imported in the `self` module must be set,
/// even if the widget gets an intrinsic `background_color` property the extension property will still be used.
///
/// The example above is equivalent to:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let mut wgt = zng_app::widget::base::WidgetBase::widget_new();
/// self::background_color::background_color(&mut wgt, colors::BLUE);
/// # }
/// ```
///
/// ## Named Assign
///
/// Properties can have multiple parameters, multiple parameters can be set using the struct init syntax:
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// border = {
///     widths: 1,
///     sides: colors::RED,
/// };
/// # }; }
/// ```
///
/// Note that just like in struct init the parameters don't need to be in order:
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// border = {
///     sides: colors::RED,
///     widths: 1,
/// };
/// # }; }
/// ```
///
/// Internally each property method has auxiliary methods that validate the member names and construct the property using sorted params, therefore
/// accepting any parameter order. Note each parameter is evaluated in the order they appear, even if they are assigned in a different order after.
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// let mut eval_order = vec![];
///
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// border = {
///     sides: {
///         eval_order.push("sides");
///         colors::RED
///     },
///     widths: {
///         eval_order.push("widths");
///         1
///     },
/// };
/// # };
///
/// assert_eq!(eval_order, vec!["sides", "widths"]);
/// # }
/// ```
///
/// ## Unnamed Assign Multiple
///
/// Properties with multiple parameters don't need to be set using the named syntax:
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// border = 1, colors::RED;
/// # }; }
/// ```
///
/// The example above is equivalent to:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let mut wgt = zng_app::widget::base::WidgetBase::widget_new();
/// wgt.border(1, colors::RED);
/// # }
/// ```
///
/// ## Shorthand Assign
///
/// Is a variable with the same name as a property is in context the `= name` can be omitted:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// let id = "name";
/// let background_color = colors::BLUE;
/// let widths = 1;
///
/// let wgt = zng_app::widget::base::WidgetBase! {
///     id;
///     self::background_color;
///     border = {
///         widths,
///         sides: colors::RED,
///     };
/// };
/// # }
/// ```
///
/// Note that the shorthand syntax also works for path properties and parameter names.
///
/// The above is equivalent to:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// let id = "name";
/// let background_color = colors::BLUE;
/// let widths = 1;
///
/// let wgt = zng_app::widget::base::WidgetBase! {
///     id = id;
///     self::background_color = background_color;
///     border = {
///         widths: widths,
///         sides: colors::RED,
///     };
/// };
/// # }
/// ```
///
/// # Property Unset
///
/// All properties can be assigned to an special value `unset!`, that *removes* a property, when the widget is build the
/// unset property will not be instantiated:
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// border = unset!;
/// # }; }
/// ```
///
/// The example above is equivalent to:
///
/// ```
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let mut wgt = zng_app::widget::base::WidgetBase::widget_new();
/// wgt.unset_border();
/// # }
/// ```
///
/// Each property method generates an auxiliary `unset_property` method, the unset is registered in the widget builder using the current
/// importance, in `widget_intrinsic` they only unset already inherited default assigns, in instances it unsets all inherited or
/// previous assigns, see [`WidgetBuilder::push_unset`] for more details.
///
/// # Generic Properties
///
/// Generic properties need a *turbofish* annotation on assign:
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn value<T: VarValue>(child: impl IntoUiNode, value: impl IntoVar<T>) -> UiNode { child.into_node() }
/// #
/// # fn main() {
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// value::<f32> = 1.0;
/// # };}
/// ```
///
/// # When
///
/// Conditional property assigns can be setup using `when` blocks. A `when` block has a `bool` expression and property assigns,
/// when the expression is `true` each property has the assigned value, unless it is overridden by a later `when` block.
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// # #[property(CONTEXT)] pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode { child.into_node() }
/// # #[property(EVENT)] pub fn is_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode { child.into_node() }
/// # fn main() {
/// # let _scope = APP.minimal();
/// # let wgt = zng_app::widget::base::WidgetBase! {
/// background_color = colors::RED;
///
/// when *#is_pressed {
///     background_color = colors::GREEN;
/// }
/// # }; }
/// ```
///
/// ## When Condition
///
/// The `when` block defines a condition expression, in the example above this is `*#is_pressed`. The expression can be any Rust expression
/// that results in a [`bool`] value, you can reference properties in it using the `#` token followed by the property name or path and you
/// can reference variables in it using the `#{var}` syntax. If a property or var is referenced the `when` block is dynamic, updating all
/// assigned properties when the expression result changes.
///
/// ### Property Reference
///
/// The most common `when` expression reference is a property, in the example above the `is_pressed` property is instantiated for the widget
/// and it controls when the background is set to green. Note that a reference to the value is inserted in the expression
/// so an extra deref `*` is required. A property can also be referenced with a path, `#properties::is_pressed` also works.
///
/// The syntax seen so far is actually a shorthand way to reference the first input of a property, the full syntax is `#is_pressed.0` or
/// `#is_pressed.state`. You can use the extended syntax to reference inputs of properties with more than one input, the input can be
/// reference by tuple-style index or by name. Note that if the value it self is a tuple or `struct` you need to use the extended syntax
/// to reference a member of the value, `#foo.0.0` or `#foo.0.name`. Methods have no ambiguity, `#foo.name()` is the same as `#foo.0.name()`.
///
/// Not all properties can be referenced in `when` conditions, only inputs of type `impl IntoVar<T>` and `impl IntoValue<T>` are
/// allowed, attempting to reference a different kind of input generates a compile error.
///
/// ### Variable Reference
///
/// Other variable can also be referenced, context variables or any locally declared variable can be referenced. Like with properties
/// the variable value is inserted in the expression as a reference so you may need to deref in case the var is a simple [`Copy`] value.
///
/// ```rust,no_fmt
/// # use zng_app::{*, widget::{node::*, property, self}};
/// # use zng_color::*;
/// # use zng_var::*;
/// # use zng_layout::unit::*;
/// #
/// # #[property(FILL)]
/// # pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
/// #   let _ = color;
/// #   child.into_node()
/// # }
/// #
/// context_var! {
///     pub static FOO_VAR: Vec<&'static str> = vec![];
///     pub static BAR_VAR: bool = false;
/// }
///
/// # fn main() {
/// # let _scope = APP.minimal();
/// # let wgt = widget::base::WidgetBase! {
/// background_color = colors::RED;
/// when !*#{BAR_VAR} && #{FOO_VAR}.contains(&"green") {
///     background_color = colors::GREEN;
/// }
/// # };}
/// ```
///
/// ## When Assigns
///
/// Inside the `when` block a list of property assigns is expected, most properties can be assigned, but `impl IntoValue<T>` properties cannot,
/// you also cannot `unset!` in when assigns, a compile time error happens if the property cannot be assigned.
///
/// On instantiation a single instance of the property will be generated, the parameters will track the when expression state and update
/// to the value assigned when it is `true`. When no block is `true` the value assigned to the property outside `when` blocks is used, or the property default value. When more then one block is `true` the *last* one sets the value.
///
/// ### Default Values
///
/// A when assign can be defined by a property without setting a default value, during instantiation if the property declaration has
/// a default value it is used, or if the property was later assigned a value it is used as *default*, if it is not possible to generate
/// a default value the property is not instantiated and the when assign is not used.
///
/// The same apply for properties referenced in the condition expression, note that all `is_state` properties have a default value so
/// it is more rare that a default value is not available. If a condition property cannot be generated the entire when block is ignored.
///
/// [`WidgetBase`]: struct@crate::widget::base::WidgetBase
/// [`WidgetBuilder::push_unset`]: crate::widget::builder::WidgetBuilder::push_unset
#[macro_export]
macro_rules! widget_set {
    (
        $(#[$skip:meta])*
        $($invalid:ident)::+ = $($tt:tt)*
    ) => {
        compile_error!{
            "expected `&mut <wgt>;` at the beginning"
        }
    };
    (
        $(#[$skip:meta])*
        when = $($invalid:tt)*
    ) => {
        compile_error!{
            "expected `&mut <wgt>;` at the beginning"
        }
    };
    (
        $wgt_mut:ident;
        $($tt:tt)*
    ) => {
        $crate::widget::widget_set! {
            &mut *$wgt_mut;
            $($tt)*
        }
    };
    (
        $wgt_borrow_mut:expr;
        $($tt:tt)*
    ) => {
        $crate::widget::widget_new! {
            new {
                let wgt__ = $wgt_borrow_mut;
            }
            build { }
            set { $($tt)* }
        }
    };
}
#[doc(inline)]
pub use widget_set;

/// <span data-del-macro-root></span> Implement a property on the widget to strongly associate it with the widget.
///
/// Widget implemented properties can be used on the widget without needing to be imported, they also show in
/// the widget documentation page. As a general rule only properties that are captured by the widget, or only work with the widget,
/// or have an special meaning in the widget are implemented like this, standalone properties that can be used in
/// any widget are not implemented.
///
/// Note that you can also implement a property for a widget in the property declaration using the
/// `impl(Widget)` directive in the [`property`] macro.
///
/// # Syntax
///
/// The macro syntax is one or more impl declarations, each declaration can have docs followed by the implementation
/// visibility, usually `pub`, followed by the path to the property function, followed by a parenthesized list of
/// the function input arguments, terminated by semicolon.
///
/// `pub path::to::property(input: impl IntoVar<bool>);`
///
/// # Examples
///
/// The example below declares a widget and uses this macro to implements the `align` property for the widget.
///
/// ```
/// # fn main() { }
/// # use zng_app::widget::{*, node::{UiNode, IntoUiNode}, base::WidgetBase};
/// # use zng_layout::unit::Align;
/// # use zng_var::IntoVar;
/// # mod zng { use super::*; pub mod widget { use super::*; #[zng_app::widget::property(LAYOUT)] pub fn align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode { child.into_node() } } }
/// #
/// #[widget($crate::MyWgt)]
/// pub struct MyWgt(WidgetBase);
///
/// impl MyWgt {
///     widget_impl! {
///         /// Docs for the property in the widget.
///         pub zng::widget::align(align: impl IntoVar<Align>);
///     }
/// }
/// ```
#[macro_export]
macro_rules! widget_impl {
    (
        $(
            $(#[$attr:meta])*
            $vis:vis $($property:ident)::+ ($($arg:ident : $arg_ty:ty)*);
        )+
    ) => {
        $(
            $crate::widget::property_impl! {
                attrs { $(#[$attr])* }
                vis { $vis }
                path { $($property)::* }
                args { $($arg:$arg_ty),* }
            }
        )+
    }
}
#[doc(inline)]
pub use widget_impl;

zng_unique_id::unique_id_64! {
    /// Unique ID of a widget.
    ///
    /// # Name
    ///
    /// IDs are only unique for the same process.
    /// You can associate a [`name`] with an ID to give it a persistent identifier.
    ///
    /// [`name`]: WidgetId::name
    pub struct WidgetId;
}
zng_unique_id::impl_unique_id_name!(WidgetId);
zng_unique_id::impl_unique_id_fmt!(WidgetId);
zng_unique_id::impl_unique_id_bytemuck!(WidgetId);

zng_var::impl_from_and_into_var! {
    /// Calls [`WidgetId::named`].
    fn from(name: &'static str) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: String) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Cow<'static, str>) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: char) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Txt) -> WidgetId {
        WidgetId::named(name)
    }
    fn from(id: WidgetId) -> zng_view_api::access::AccessNodeId {
        zng_view_api::access::AccessNodeId(id.get())
    }

    fn from(some: WidgetId) -> Option<WidgetId>;
}
impl serde::Serialize for WidgetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unnamed `WidgetId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for WidgetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(WidgetId::named(name))
    }
}

/// Defines how widget update requests inside [`WIDGET::with_context`] are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetUpdateMode {
    /// All updates flagged during the closure call are discarded, previous pending
    /// requests are retained.
    ///
    /// This mode is used by [`UiNodeOp::Measure`].
    ///
    /// [`UiNodeOp::Measure`]: crate::widget::node::UiNodeOp::Measure
    Ignore,
    /// All updates flagged after the closure call are retained and propagate to the parent widget flags.
    ///
    /// This is the mode is used for all [`UiNodeOp`] delegation, except measure.
    ///
    /// [`UiNodeOp`]: crate::widget::node::UiNodeOp
    Bubble,
}

/// Current context widget.
///
/// # Panics
///
/// Most of the methods on this service panic if not called inside a widget context.
pub struct WIDGET;
impl WIDGET {
    /// Returns `true` if called inside a widget.
    pub fn is_in_widget(&self) -> bool {
        !WIDGET_CTX.is_default()
    }

    /// Get the widget ID, if called inside a widget.
    pub fn try_id(&self) -> Option<WidgetId> {
        if self.is_in_widget() { Some(WIDGET_CTX.get().id) } else { None }
    }

    /// Gets a text with detailed path to the current widget.
    ///
    /// This can be used to quickly identify the current widget during debug, the path printout will contain
    /// the widget types if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of a widget.
    pub fn trace_path(&self) -> Txt {
        if let Some(w_id) = WINDOW.try_id() {
            if let Some(id) = self.try_id() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_path()
                } else {
                    formatx!("{w_id:?}//<no-info>/{id:?}")
                }
            } else {
                formatx!("{w_id:?}//<no-widget>")
            }
        } else if let Some(id) = self.try_id() {
            formatx!("<no-window>//{id:?}")
        } else {
            Txt::from_str("<no-widget>")
        }
    }

    /// Gets a text with a detailed widget id.
    ///
    /// This can be used to quickly identify the current widget during debug, the printout will contain the widget
    /// type if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of a widget.
    pub fn trace_id(&self) -> Txt {
        if let Some(id) = self.try_id() {
            if WINDOW.try_id().is_some() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_id()
                } else {
                    formatx!("{id:?}")
                }
            } else {
                formatx!("{id:?}")
            }
        } else {
            Txt::from("<no-widget>")
        }
    }

    /// Get the widget ID.
    pub fn id(&self) -> WidgetId {
        WIDGET_CTX.get().id
    }

    /// Gets the widget info.
    pub fn info(&self) -> WidgetInfo {
        WINDOW.info().get(WIDGET.id()).expect("widget info not init")
    }

    /// Widget bounds, updated every layout.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        WIDGET_CTX.get().bounds.lock().clone()
    }

    /// Widget border, updated every layout.
    pub fn border(&self) -> WidgetBorderInfo {
        WIDGET_CTX.get().border.lock().clone()
    }

    /// Gets the parent widget or `None` if is root.
    ///
    /// Panics if not called inside a widget.
    pub fn parent_id(&self) -> Option<WidgetId> {
        WIDGET_CTX.get().parent_id.load(Relaxed)
    }

    /// Schedule an [`UpdateOp`] for the current widget.
    pub fn update_op(&self, op: UpdateOp) -> &Self {
        match op {
            UpdateOp::Update => self.update(),
            UpdateOp::Info => self.update_info(),
            UpdateOp::Layout => self.layout(),
            UpdateOp::Render => self.render(),
            UpdateOp::RenderUpdate => self.render_update(),
        }
    }

    fn update_impl(&self, flag: UpdateFlags) -> &Self {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(flag) {
                f.insert(flag);
                Some(f)
            } else {
                None
            }
        });
        self
    }

    /// Schedule an update for the current widget.
    ///
    /// After the current update the app-extensions, parent window and widgets will update again.
    pub fn update(&self) -> &Self {
        UpdatesTrace::log_update();
        self.update_impl(UpdateFlags::UPDATE)
    }

    /// Schedule an info rebuild for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-build the info tree.
    pub fn update_info(&self) -> &Self {
        UpdatesTrace::log_info();
        self.update_impl(UpdateFlags::INFO)
    }

    /// Schedule a re-layout for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-layout.
    pub fn layout(&self) -> &Self {
        UpdatesTrace::log_layout();
        self.update_impl(UpdateFlags::LAYOUT)
    }

    /// Schedule a re-render for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will re-render.
    ///
    /// This also overrides any pending [`render_update`] request.
    ///
    /// [`render_update`]: Self::render_update
    pub fn render(&self) -> &Self {
        UpdatesTrace::log_render();
        self.update_impl(UpdateFlags::RENDER)
    }

    /// Schedule a frame update for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will update the frame.
    ///
    /// This request is supplanted by any [`render`] request.
    ///
    /// [`render`]: Self::render
    pub fn render_update(&self) -> &Self {
        UpdatesTrace::log_render();
        self.update_impl(UpdateFlags::RENDER_UPDATE)
    }

    /// Flags the widget to re-init after the current update returns.
    ///
    /// The widget responds to this request differently depending on the node method that calls it:
    ///
    /// * [`UiNode::init`] and [`UiNode::deinit`]: Request is ignored, removed.
    /// * [`UiNode::event`]: If the widget is pending a reinit, it is reinited first, then the event is propagated to child nodes.
    ///   If a reinit is requested during event handling the widget is reinited immediately after the event handler.
    /// * [`UiNode::update`]: If the widget is pending a reinit, it is reinited and the update ignored.
    ///   If a reinit is requested during update the widget is reinited immediately after the update.
    /// * Other methods: Reinit request is flagged and an [`UiNode::update`] is requested for the widget.
    ///
    /// [`UiNode::init`]: crate::widget::node::UiNode::init
    /// [`UiNode::deinit`]: crate::widget::node::UiNode::deinit
    /// [`UiNode::event`]: crate::widget::node::UiNode::event
    /// [`UiNode::update`]: crate::widget::node::UiNode::update
    pub fn reinit(&self) {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(UpdateFlags::REINIT) {
                f.insert(UpdateFlags::REINIT);
                Some(f)
            } else {
                None
            }
        });
    }

    /// Calls `f` with a read lock on the current widget state map.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.read().borrow())
    }

    /// Calls `f` with a write lock on the current widget state map.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.write().borrow_mut())
    }

    /// Get the widget state `id`, if it is set.
    pub fn get_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        let id = id.into();
        self.with_state(|s| s.get_clone(id))
    }

    /// Require the widget state `id`.
    ///
    /// Panics if the `id` is not set.
    pub fn req_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> T {
        let id = id.into();
        self.with_state(|s| s.req(id).clone())
    }

    /// Set the widget state `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_state<T: StateValue>(&self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        let id = id.into();
        let value = value.into();
        self.with_state_mut(|mut s| s.set(id, value))
    }

    /// Sets the widget state `id` without value.
    ///
    /// Returns if the state `id` was already flagged.
    pub fn flag_state(&self, id: impl Into<StateId<()>>) -> bool {
        let id = id.into();
        self.with_state_mut(|mut s| s.flag(id))
    }

    /// Calls `init` and sets `id` if it is not already set in the widget.
    pub fn init_state<T: StateValue>(&self, id: impl Into<StateId<T>>, init: impl FnOnce() -> T) {
        let id = id.into();
        self.with_state_mut(|mut s| {
            s.entry(id).or_insert_with(init);
        });
    }

    /// Sets the `id` to the default value if it is not already set.
    pub fn init_state_default<T: StateValue + Default>(&self, id: impl Into<StateId<T>>) {
        self.init_state(id.into(), Default::default)
    }

    /// Returns `true` if the `id` is set or flagged in the widget.
    pub fn contains_state<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        let id = id.into();
        self.with_state(|s| s.contains(id))
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes.
    pub fn sub_var_op(&self, op: UpdateOp, var: &AnyVar) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe(op, w.id);

        // function to avoid generics code bloat
        fn push(w: Arc<WidgetCtxData>, s: VarHandle) {
            if WIDGET_HANDLES_CTX.is_default() {
                w.handles.var_handles.lock().push(s);
            } else {
                WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
            }
        }
        push(w, s);

        self
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes and `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_op_when<T: VarValue>(
        &self,
        op: UpdateOp,
        var: &Var<T>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe_when(op, w.id, predicate);

        // function to avoid generics code bloat
        fn push(w: Arc<WidgetCtxData>, s: VarHandle) {
            if WIDGET_HANDLES_CTX.is_default() {
                w.handles.var_handles.lock().push(s);
            } else {
                WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
            }
        }
        push(w, s);

        self
    }

    /// Subscribe to receive updates when the `var` changes.
    pub fn sub_var(&self, var: &AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Update, var)
    }
    /// Subscribe to receive updates when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_when<T: VarValue>(&self, var: &Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Update, var, predicate)
    }

    /// Subscribe to receive info rebuild requests when the `var` changes.
    pub fn sub_var_info(&self, var: &AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Info, var)
    }
    /// Subscribe to receive info rebuild requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_info_when<T: VarValue>(&self, var: &Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Info, var, predicate)
    }

    /// Subscribe to receive layout requests when the `var` changes.
    pub fn sub_var_layout(&self, var: &AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Layout, var)
    }
    /// Subscribe to receive layout requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_layout_when<T: VarValue>(&self, var: &Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Layout, var, predicate)
    }

    /// Subscribe to receive render requests when the `var` changes.
    pub fn sub_var_render(&self, var: &AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Render, var)
    }
    /// Subscribe to receive render requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_when<T: VarValue>(&self, var: &Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Render, var, predicate)
    }

    /// Subscribe to receive render update requests when the `var` changes.
    pub fn sub_var_render_update(&self, var: &AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::RenderUpdate, var)
    }
    /// Subscribe to receive render update requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_update_when<T: VarValue>(&self, var: &Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::RenderUpdate, var, predicate)
    }

    /// Subscribe to receive events from `event` when the event targets this widget.
    pub fn sub_event<A: EventArgs>(&self, event: &Event<A>) -> &Self {
        let w = WIDGET_CTX.get();
        let s = event.subscribe(w.id);

        // function to avoid generics code bloat
        fn push(w: Arc<WidgetCtxData>, s: EventHandle) {
            if WIDGET_HANDLES_CTX.is_default() {
                w.handles.event_handles.lock().push(s);
            } else {
                WIDGET_HANDLES_CTX.get().event_handles.lock().push(s);
            }
        }
        push(w, s);

        self
    }

    /// Hold the event `handle` until the widget is deinited.
    pub fn push_event_handle(&self, handle: EventHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().push(handle);
        }
    }

    /// Hold the event `handles` until the widget is deinited.
    pub fn push_event_handles(&self, handles: EventHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().extend(handles);
        }
    }

    /// Hold the var `handle` until the widget is deinited.
    pub fn push_var_handle(&self, handle: VarHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(handle);
        }
    }

    /// Hold the var `handles` until the widget is deinited.
    pub fn push_var_handles(&self, handles: VarHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().extend(handles);
        }
    }

    /// Transform point in the window space to the widget inner bounds.
    pub fn win_point_to_wgt(&self, point: DipPoint) -> Option<PxPoint> {
        let wgt_info = WIDGET.info();
        wgt_info
            .inner_transform()
            .inverse()?
            .transform_point(point.to_px(wgt_info.tree().scale_factor()))
    }

    /// Gets the transform from the window space to the widget inner bounds.
    pub fn win_to_wgt(&self) -> Option<PxTransform> {
        WIDGET.info().inner_transform().inverse()
    }

    /// Calls `f` with an override target for var and event subscription handles.
    ///
    /// By default when vars and events are subscribed using the methods of this service the
    /// subscriptions live until the widget is deinited. This method intersects these
    /// subscriptions, registering then in `handles` instead.
    pub fn with_handles<R>(&self, handles: &mut WidgetHandlesCtx, f: impl FnOnce() -> R) -> R {
        WIDGET_HANDLES_CTX.with_context(&mut handles.0, f)
    }

    /// Calls `f` while the widget is set to `ctx`.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the `ctx` after `f` will be copied to the
    /// caller widget context, otherwise they are ignored.
    ///
    /// This method can be used to manually define a widget context, note that widgets already define their own context.
    #[inline(always)]
    pub fn with_context<R>(&self, ctx: &mut WidgetCtx, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> R {
        struct Restore<'a> {
            update_mode: WidgetUpdateMode,
            parent_id: Option<WidgetId>,
            prev_flags: UpdateFlags,
            ctx: &'a mut WidgetCtx,
        }
        impl<'a> Restore<'a> {
            fn new(ctx: &'a mut WidgetCtx, update_mode: WidgetUpdateMode) -> Self {
                let parent_id = WIDGET.try_id();

                if let Some(ctx) = ctx.0.as_mut() {
                    ctx.parent_id.store(parent_id, Relaxed);
                } else {
                    unreachable!()
                }

                let prev_flags = match update_mode {
                    WidgetUpdateMode::Ignore => ctx.0.as_mut().unwrap().flags.load(Relaxed),
                    WidgetUpdateMode::Bubble => UpdateFlags::empty(),
                };

                Self {
                    update_mode,
                    parent_id,
                    prev_flags,
                    ctx,
                }
            }
        }
        impl<'a> Drop for Restore<'a> {
            fn drop(&mut self) {
                let ctx = match self.ctx.0.as_mut() {
                    Some(c) => c,
                    None => return, // can happen in case of panic
                };

                match self.update_mode {
                    WidgetUpdateMode::Ignore => {
                        ctx.flags.store(self.prev_flags, Relaxed);
                    }
                    WidgetUpdateMode::Bubble => {
                        let wgt_flags = ctx.flags.load(Relaxed);

                        if let Some(parent) = self.parent_id.map(|_| WIDGET_CTX.get()) {
                            let propagate = wgt_flags
                                & (UpdateFlags::UPDATE
                                    | UpdateFlags::INFO
                                    | UpdateFlags::LAYOUT
                                    | UpdateFlags::RENDER
                                    | UpdateFlags::RENDER_UPDATE);

                            let _ = parent.flags.fetch_update(Relaxed, Relaxed, |mut u| {
                                if !u.contains(propagate) {
                                    u.insert(propagate);
                                    Some(u)
                                } else {
                                    None
                                }
                            });
                            ctx.parent_id.store(None, Relaxed);
                        } else if let Some(window_id) = WINDOW.try_id() {
                            // is at root, register `UPDATES`
                            UPDATES.update_flags_root(wgt_flags, window_id, ctx.id);
                            // some builders don't clear the root widget flags like they do for other widgets.
                            ctx.flags.store(wgt_flags & UpdateFlags::REINIT, Relaxed);
                        } else {
                            // used outside window
                            UPDATES.update_flags(wgt_flags, ctx.id);
                            ctx.flags.store(UpdateFlags::empty(), Relaxed);
                        }
                    }
                }
            }
        }

        let mut _restore = Restore::new(ctx, update_mode);
        WIDGET_CTX.with_context(&mut _restore.ctx.0, f)
    }
    /// Calls `f` while no widget is available in the context.
    #[inline(always)]
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WIDGET_CTX.with_default(f)
    }

    #[cfg(any(test, doc, feature = "test_util"))]
    pub(crate) fn test_root_updates(&self) {
        let ctx = WIDGET_CTX.get();
        // is at root, register `UPDATES`
        UPDATES.update_flags_root(ctx.flags.load(Relaxed), WINDOW.id(), ctx.id);
        // some builders don't clear the root widget flags like they do for other widgets.
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
    }

    pub(crate) fn layout_is_pending(&self, layout_widgets: &LayoutUpdates) -> bool {
        let ctx = WIDGET_CTX.get();
        ctx.flags.load(Relaxed).contains(UpdateFlags::LAYOUT) || layout_widgets.delivery_list().enter_widget(ctx.id)
    }

    /// Remove update flag and returns if it intersected.
    pub(crate) fn take_update(&self, flag: UpdateFlags) -> bool {
        let mut r = false;
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(flag) {
                r = true;
                f.remove(flag);
                Some(f)
            } else {
                None
            }
        });
        r
    }

    /// Current pending updates.
    #[cfg(debug_assertions)]
    pub(crate) fn pending_update(&self) -> UpdateFlags {
        WIDGET_CTX.get().flags.load(Relaxed)
    }

    /// Remove the render reuse range if render was not invalidated on this widget.
    pub(crate) fn take_render_reuse(&self, render_widgets: &RenderUpdates, render_update_widgets: &RenderUpdates) -> Option<ReuseRange> {
        let ctx = WIDGET_CTX.get();
        let mut try_reuse = true;

        // take RENDER, RENDER_UPDATE
        let _ = ctx.flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE) {
                try_reuse = false;
                f.remove(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);
                Some(f)
            } else {
                None
            }
        });

        if try_reuse && !render_widgets.delivery_list().enter_widget(ctx.id) && !render_update_widgets.delivery_list().enter_widget(ctx.id)
        {
            ctx.render_reuse.lock().take()
        } else {
            None
        }
    }

    pub(crate) fn set_render_reuse(&self, range: Option<ReuseRange>) {
        *WIDGET_CTX.get().render_reuse.lock() = range;
    }
}

context_local! {
    pub(crate) static WIDGET_CTX: WidgetCtxData = WidgetCtxData::no_context();
    static WIDGET_HANDLES_CTX: WidgetHandlesCtxData = WidgetHandlesCtxData::dummy();
}

/// Defines the backing data of [`WIDGET`].
///
/// Each widget owns this data and calls [`WIDGET.with_context`] to delegate to it's child node.
///
/// [`WIDGET.with_context`]: WIDGET::with_context
pub struct WidgetCtx(Option<Arc<WidgetCtxData>>);
impl WidgetCtx {
    /// New widget context.
    pub fn new(id: WidgetId) -> Self {
        Self(Some(Arc::new(WidgetCtxData {
            parent_id: Atomic::new(None),
            id,
            flags: Atomic::new(UpdateFlags::empty()),
            state: RwLock::new(OwnedStateMap::default()),
            handles: WidgetHandlesCtxData::dummy(),
            bounds: Mutex::new(WidgetBoundsInfo::default()),
            border: Mutex::new(WidgetBorderInfo::default()),
            render_reuse: Mutex::new(None),
        })))
    }

    /// Drops all var and event handles, clears all state.
    ///
    /// If `retain_state` is enabled the state will not be cleared and can still read.
    pub fn deinit(&mut self, retain_state: bool) {
        let ctx = self.0.as_mut().unwrap();
        ctx.handles.var_handles.lock().clear();
        ctx.handles.event_handles.lock().clear();
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
        *ctx.render_reuse.lock() = None;

        if !retain_state {
            ctx.state.write().clear();
        }
    }

    /// Returns `true` if reinit was requested for the widget.
    ///
    /// Note that widget implementers must use [`take_reinit`] to fulfill the request.
    ///
    /// [`take_reinit`]: Self::take_reinit
    pub fn is_pending_reinit(&self) -> bool {
        self.0.as_ref().unwrap().flags.load(Relaxed).contains(UpdateFlags::REINIT)
    }

    /// Returns `true` if an [`WIDGET.reinit`] request was made.
    ///
    /// Unlike other requests, the widget implement must re-init immediately.
    ///
    /// [`WIDGET.reinit`]: WIDGET::reinit
    pub fn take_reinit(&mut self) -> bool {
        let ctx = self.0.as_mut().unwrap();

        let mut flags = ctx.flags.load(Relaxed);
        let r = flags.contains(UpdateFlags::REINIT);
        if r {
            flags.remove(UpdateFlags::REINIT);
            ctx.flags.store(flags, Relaxed);
        }

        r
    }

    /// Gets the widget id.
    pub fn id(&self) -> WidgetId {
        self.0.as_ref().unwrap().id
    }
    /// Gets the widget bounds.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        self.0.as_ref().unwrap().bounds.lock().clone()
    }

    /// Gets the widget borders.
    pub fn border(&self) -> WidgetBorderInfo {
        self.0.as_ref().unwrap().border.lock().clone()
    }

    /// Call `f` with an exclusive lock to the widget state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WIDGET>) -> R) -> R {
        f(&mut self.0.as_mut().unwrap().state.write())
    }

    /// Clone a reference to the widget context.
    ///
    /// This must be used only if the widget implementation is split.
    pub fn share(&mut self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) struct WidgetCtxData {
    parent_id: Atomic<Option<WidgetId>>,
    pub(crate) id: WidgetId,
    flags: Atomic<UpdateFlags>,
    state: RwLock<OwnedStateMap<WIDGET>>,
    handles: WidgetHandlesCtxData,
    pub(crate) bounds: Mutex<WidgetBoundsInfo>,
    border: Mutex<WidgetBorderInfo>,
    render_reuse: Mutex<Option<ReuseRange>>,
}
impl WidgetCtxData {
    #[track_caller]
    fn no_context() -> Self {
        panic!("no widget in context")
    }
}

struct WidgetHandlesCtxData {
    var_handles: Mutex<VarHandles>,
    event_handles: Mutex<EventHandles>,
}

impl WidgetHandlesCtxData {
    const fn dummy() -> Self {
        Self {
            var_handles: Mutex::new(VarHandles::dummy()),
            event_handles: Mutex::new(EventHandles::dummy()),
        }
    }
}

/// Defines the backing data for [`WIDGET.with_handles`].
///
/// [`WIDGET.with_handles`]: WIDGET::with_handles
pub struct WidgetHandlesCtx(Option<Arc<WidgetHandlesCtxData>>);
impl WidgetHandlesCtx {
    /// New empty.
    pub fn new() -> Self {
        Self(Some(Arc::new(WidgetHandlesCtxData::dummy())))
    }

    /// Drop all handles.
    pub fn clear(&mut self) {
        let h = self.0.as_ref().unwrap();
        h.var_handles.lock().clear();
        h.event_handles.lock().clear();
    }
}
impl Default for WidgetHandlesCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension method to subscribe any widget to a variable.
///
/// Also see [`WIDGET`] methods for the primary way to subscribe from inside a widget.
pub trait AnyVarSubscribe {
    /// Register the widget to receive an [`UpdateOp`] when this variable is new.
    ///
    /// Variables without the [`NEW`] capability return [`VarHandle::dummy`].
    ///
    /// [`NEW`]: zng_var::VarCapability::NEW
    /// [`VarHandle::dummy`]: zng_var::VarHandle
    fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle;
}
impl AnyVarSubscribe for AnyVar {
    fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle {
        if !self.capabilities().is_const() {
            self.hook(move |_| {
                UPDATES.update_op(op, widget_id);
                true
            })
        } else {
            VarHandle::dummy()
        }
    }
}

/// Extension methods to subscribe any widget to a variable or app handlers to a variable.
///
/// Also see [`WIDGET`] methods for the primary way to subscribe from inside a widget.
pub trait VarSubscribe<T: VarValue>: AnyVarSubscribe {
    /// Register the widget to receive an [`UpdateOp`] when this variable is new and the `predicate` approves the new value.
    ///
    /// Variables without the [`NEW`] capability return [`VarHandle::dummy`].
    ///
    /// [`NEW`]: zng_var::VarCapability::NEW
    /// [`VarHandle::dummy`]: zng_var::VarHandle
    fn subscribe_when(&self, op: UpdateOp, widget_id: WidgetId, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> VarHandle;

    /// Add a preview `handler` that is called every time this variable updates,
    /// the handler is called before UI update.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] used inside will have the default value.
    ///
    /// [`ContextVar<T>`]: zng_var::ContextVar
    fn on_pre_new(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle;

    /// Add a `handler` that is called every time this variable updates,
    /// the handler is called after UI update.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] used inside will have the default value.
    ///
    /// [`ContextVar<T>`]: zng_var::ContextVar
    fn on_new(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle;
}
impl<T: VarValue> AnyVarSubscribe for Var<T> {
    fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle {
        self.as_any().subscribe(op, widget_id)
    }
}
impl<T: VarValue> VarSubscribe<T> for Var<T> {
    fn subscribe_when(&self, op: UpdateOp, widget_id: WidgetId, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> VarHandle {
        self.hook(move |a| {
            if let Some(a) = a.downcast_value::<T>() {
                if predicate(a) {
                    UPDATES.update_op(op, widget_id);
                }
                true
            } else {
                false
            }
        })
    }

    fn on_pre_new(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle {
        var_on_new(self, handler, true)
    }

    fn on_new(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle {
        var_on_new(self, handler, false)
    }
}

/// Extension methods to subscribe app handlers to a response variable.
pub trait ResponseVarSubscribe<T: VarValue> {
    /// Add a `handler` that is called once when the response is received,
    /// the handler is called before all other UI updates.
    ///
    /// The handler is not called if already [`is_done`], in this case a dummy handle is returned.
    ///
    /// [`is_done`]: ResponseVar::is_done
    fn on_pre_rsp(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle;

    /// Add a `handler` that is called once when the response is received,
    /// the handler is called after all other UI updates.
    ///
    /// The handler is not called if already [`is_done`], in this case a dummy handle is returned.
    ///
    /// [`is_done`]: ResponseVar::is_done
    fn on_rsp(&self, handler: Handler<OnVarArgs<T>>) -> VarHandle;
}
impl<T: VarValue> ResponseVarSubscribe<T> for ResponseVar<T> {
    fn on_pre_rsp(&self, mut handler: Handler<OnVarArgs<T>>) -> VarHandle {
        if self.is_done() {
            return VarHandle::dummy();
        }

        self.on_pre_new(Box::new(move |args| {
            if let zng_var::Response::Done(value) = &args.value {
                APP_HANDLER.unsubscribe();
                handler(&OnVarArgs::new(value.clone(), args.tags.clone()))
            } else {
                HandlerResult::Done
            }
        }))
    }

    fn on_rsp(&self, mut handler: Handler<OnVarArgs<T>>) -> VarHandle {
        if self.is_done() {
            return VarHandle::dummy();
        }

        self.on_new(Box::new(move |args| {
            if let zng_var::Response::Done(value) = &args.value {
                APP_HANDLER.unsubscribe();
                handler(&OnVarArgs::new(value.clone(), args.tags.clone()))
            } else {
                HandlerResult::Done
            }
        }))
    }
}

fn var_on_new<T>(var: &Var<T>, handler: Handler<OnVarArgs<T>>, is_preview: bool) -> VarHandle
where
    T: VarValue,
{
    if var.capabilities().is_const() {
        return VarHandle::dummy();
    }

    let handler = handler.into_arc();
    let (inner_handle_owner, inner_handle) = Handle::new(());
    var.hook(move |args| {
        if inner_handle_owner.is_dropped() {
            return false;
        }

        let handle = inner_handle.downgrade();
        let value = args.value().clone();
        let tags: Vec<_> = args.tags().to_vec();

        let update_once: Handler<crate::update::UpdateArgs> = Box::new(clmv!(handler, |_| {
            APP_HANDLER.unsubscribe(); // once
            APP_HANDLER.with(handle.clone_boxed(), is_preview, || {
                handler.call(&OnVarArgs::new(value.clone(), tags.clone()))
            })
        }));

        if is_preview {
            UPDATES.on_pre_update(update_once).perm();
        } else {
            UPDATES.on_update(update_once).perm();
        }
        true
    })
}

/// Arguments for a var event handler.
#[non_exhaustive]
pub struct OnVarArgs<T: VarValue> {
    /// The new value.
    pub value: T,
    /// Custom tag objects that where set when the value was modified.
    pub tags: Vec<BoxAnyVarValue>,
}
impl<T: VarValue> OnVarArgs<T> {
    /// New from value and custom modify tags.
    pub fn new(value: T, tags: Vec<BoxAnyVarValue>) -> Self {
        Self { value, tags }
    }

    /// Reference all custom tag values of type `T`.
    pub fn downcast_tags<Ta: VarValue>(&self) -> impl Iterator<Item = &Ta> + '_ {
        self.tags.iter().filter_map(|t| (*t).downcast_ref::<Ta>())
    }
}
impl<T: VarValue> Clone for OnVarArgs<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            tags: self.tags.iter().map(|t| (*t).clone_boxed()).collect(),
        }
    }
}

/// Extension methods to layout var values.
pub trait VarLayout<T: VarValue> {
    /// Compute the pixel value in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout(&self) -> T::Px
    where
        T: Layout2d;

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_dft(&self, default: T::Px) -> T::Px
    where
        T: Layout2d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_x(&self) -> Px
    where
        T: Layout1d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_y(&self) -> Px
    where
        T: Layout1d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_z(&self) -> Px
    where
        T: Layout1d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis with `default`.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_dft_x(&self, default: Px) -> Px
    where
        T: Layout1d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis with `default`.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_dft_y(&self, default: Px) -> Px
    where
        T: Layout1d;

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis with `default`.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    fn layout_dft_z(&self, default: Px) -> Px
    where
        T: Layout1d;
}
impl<T: VarValue> VarLayout<T> for Var<T> {
    fn layout(&self) -> <T>::Px
    where
        T: Layout2d,
    {
        self.with(|s| s.layout())
    }

    fn layout_dft(&self, default: <T>::Px) -> <T>::Px
    where
        T: Layout2d,
    {
        self.with(move |s| s.layout_dft(default))
    }

    fn layout_x(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_x())
    }

    fn layout_y(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_y())
    }

    fn layout_z(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_z())
    }

    fn layout_dft_x(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_x(default))
    }

    fn layout_dft_y(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_y(default))
    }

    fn layout_dft_z(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_z(default))
    }
}

/// Integrate [`UiTask`] with widget updates.
pub trait UiTaskWidget<R> {
    /// Create a UI bound future executor.
    ///
    /// The `task` is inert and must be polled using [`update`] to start, and it must be polled every
    /// [`UiNode::update`] after that, in widgets the `target` can be set so that the update requests are received.
    ///
    /// [`update`]: UiTask::update
    /// [`UiNode::update`]: crate::widget::node::UiNode::update
    /// [`UiNode::info`]: crate::widget::node::UiNode::info
    fn new<F>(target: Option<WidgetId>, task: impl IntoFuture<IntoFuture = F>) -> Self
    where
        F: Future<Output = R> + Send + 'static;

    /// Like [`new`], from an already boxed and pinned future.
    ///
    /// [`new`]: UiTaskWidget::new
    fn new_boxed(target: Option<WidgetId>, task: Pin<Box<dyn Future<Output = R> + Send + 'static>>) -> Self;
}
impl<R> UiTaskWidget<R> for UiTask<R> {
    fn new<F>(target: Option<WidgetId>, task: impl IntoFuture<IntoFuture = F>) -> Self
    where
        F: Future<Output = R> + Send + 'static,
    {
        UiTask::new_raw(UPDATES.waker(target), task)
    }

    fn new_boxed(target: Option<WidgetId>, task: Pin<Box<dyn Future<Output = R> + Send + 'static>>) -> Self {
        UiTask::new_raw_boxed(UPDATES.waker(target), task)
    }
}
