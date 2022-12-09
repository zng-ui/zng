//! Widget and property builder types.

use std::{
    any::{Any, TypeId},
    fmt, ops,
    sync::Arc,
};

use linear_map::{set::LinearSet, LinearMap};
use parking_lot::Mutex;

use crate::{
    handler::WidgetHandler,
    impl_from_and_into_var,
    text::{formatx, Text},
    var::{types::AnyWhenVarBuilder, *},
    widget_instance::{
        ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, NilUiNode, UiNode, UiNodeList, WhenUiNodeBuilder, WhenUiNodeListBuilder,
    },
};

///<span data-del-macro-root></span> New [`SourceLocation`] that represents the location you call this macro.
#[macro_export]
macro_rules! source_location {
    () => {
        $crate::widget_builder::SourceLocation {
            file: std::file!(),
            line: std::line!(),
            column: std::column!(),
        }
    };
}
#[doc(inline)]
pub use crate::source_location;

/// A location in source-code.
///
/// Use [`source_location!`] to construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// [`file!`]
    pub file: &'static str,
    /// [`line!`]
    pub line: u32,
    /// [`column!`]
    pub column: u32,
}
impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! when_condition_expr_var {
    ($($tt:tt)*) => {
        $crate::var::Var::boxed($crate::var::expr_var!{$($tt)*})
    };
}
#[doc(hidden)]
pub use when_condition_expr_var;

///<span data-del-macro-root></span> New [`PropertyId`] that represents the type and name.
///
/// # Syntax
///
/// * `property::path`: Gets the ID for the property function.
/// * `property::path as rename`: Gets the ID, but with the new name.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{property, widget_builder::property_id, widget_instance::UiNode, var::IntoValue};
/// # pub mod path {
/// #   use super::*;
/// #   #[property(CONTEXT)]
/// #   pub fn foo(child: impl UiNode, bar: impl IntoValue<bool>) -> impl UiNode {
/// #     child
/// #   }
/// # }
/// # fn main() {
/// let foo_id = property_id!(path::foo);
/// let renamed_id = property_id!(path::foo as bar);
///
/// assert_ne!(foo_id, renamed_id);
/// assert_eq!(foo_id.impl_id, renamed_id.impl_id);
/// assert_ne!(foo_id.name, renamed_id.name);
/// # }
/// ```
#[macro_export]
macro_rules! property_id {
    ($($property:ident)::+) => {{
        $($property)::+::__id__($crate::widget_builder::property_id_name(stringify!($($property)::+)))
    }};
    ($($property:ident)::+::<$($generics:ty),*>) => {{
        $($property)::+ ::<$($generics),*>::__id__($crate::widget_builder::property_id_name(stringify!($($property)::+)))
    }};
    ($($property:ident)::+ as $rename:ident) => {{
        $($property)::+::__id__($crate::widget_builder::property_id_name(stringify!($rename)))
    }};
    ($($property:ident)::+ ::<$($generics:ty),*> as $rename:ident) => {{
        $($property)::+::<$($generics),*>::__id__($crate::widget_builder::property_id_name(stringify!($rename)))
    }};
}
#[doc(inline)]
pub use crate::property_id;

///<span data-del-macro-root></span> New [`PropertyInfo`] from property path.
///
/// # Syntax
///
/// * `property::path`: Gets the info for the property function.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{property, widget_builder::*, widget_instance::*, var::*};
/// # pub mod path {
/// #   use super::*;
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoValue<bool>) -> impl UiNode {
/// #     child
/// }
/// # }
/// # fn main() {
///
/// let foo_info = property_info!(path::foo);
///
/// assert_eq!(foo_info.inputs[0].name, "bar");
/// # }
/// ```
#[macro_export]
macro_rules! property_info {
    ($($property:ident)::+) => {{
        $($property)::+::__property__()
    }};
    ($($property:ident)::+ ::<$($generics:ty),*>) => {{
        $($property)::+::<$($generics),*>::__property__()
    }};
}
#[doc(inline)]
pub use crate::property_info;

///<span data-del-macro-root></span> Gets the strong input storage types from a property path.
///
/// See [`PropertyInputTypes<Tuple>`] for more details.
///
/// # Syntax
///
/// * `property::path`: Gets the info for the property function.
#[macro_export]
macro_rules! property_input_types {
    ($($property:ident)::+) => {{
        $($property)::+::__input_types__()
    }};
    ($($property:ident)::+ ::<$($generics:ty),*>) => {{
        $($property)::+::<$($generics),*>::__input_types__()
    }};
}
#[doc(inline)]
pub use crate::property_input_types;

#[doc(hidden)]
pub fn property_id_name(path: &'static str) -> &'static str {
    path.rsplit(':').next().unwrap_or("").trim()
}

///<span data-del-macro-root></span> New [`PropertyArgs`] box from a property and value.
///
/// # Syntax
///
/// The syntax is similar to a property assign in a widget, with some extra means to reference widget properties.
///
/// * `property::path = <value>;`: Args for the property function.
/// * `property::path as rename = <value>;`: Args for the property, but the ID is renamed.
///
/// In all of these the `<value>` is the standard property init expression or named fields patterns that are used in widget assigns.
///
/// * `property = "value-0", "value-1";`: Unnamed args.
/// * `property = { value_0: "value-0", value_1: "value-1" }`: Named args.
///
/// Note that `unset!` is not a property arg, trying to use it will cause a panic.
#[macro_export]
macro_rules! property_args {
    ($($property:ident)::+ $(as $rename:ident)? = $($value:tt)*) => {
        {
            $crate::widget_builder::property_args_getter! {
                $($property)::+ $(as $rename)? = $($value)*
            }
        }
    };
    ($($property:ident)::+ ::<$($generics:ty),*> $(as $rename:ident)? = $($value:tt)*) => {
        {
            $crate::widget_builder::property_args_getter! {
                $($property)::+ ::<$($generics),*> $(as $rename)? = $($value)*
            }
        }
    };
    ($property:ident $(;)?) => {
        {
            $crate::widget_builder::property_args_getter! {
                $property
            }
        }
    }
}
#[doc(inline)]
pub use crate::property_args;

///<span data-del-macro-root></span> Gets the [`WidgetMod`] info of a widget.
#[macro_export]
macro_rules! widget_mod {
    ($widget_path:path) => {{
        #[rustfmt::skip] use $widget_path::{__widget__};
        __widget__::mod_info()
    }};
}
#[doc(inline)]
pub use widget_mod;

#[doc(hidden)]
#[crate::widget($crate::widget_builder::property_args_getter)]
pub mod property_args_getter {
    use super::*;

    fn build(mut wgt: WidgetBuilder) -> Box<dyn PropertyArgs> {
        match wgt.p.items.remove(0).item {
            WidgetItem::Property { args, .. } => args,
            WidgetItem::Intrinsic { .. } => unreachable!(),
        }
    }
}

/// Represents the sort index of a property or intrinsic node in a widget instance.
///
/// Each node "wraps" the next one, so the sort defines `(context#0 (context#1 (event (size (border..)))))`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NestPosition {
    /// The major position.
    pub group: NestGroup,
    /// Extra sorting within items of the same group.
    pub index: u16,
}
impl NestPosition {
    /// Default index used for intrinsic nodes, is `u16::MAX / 3`.
    pub const INTRINSIC_INDEX: u16 = u16::MAX / 3;

    /// Default index used for properties, is `INTRINSIC_INDEX * 2`.
    pub const PROPERTY_INDEX: u16 = Self::INTRINSIC_INDEX * 2;

    /// New position for property.
    pub fn property(group: NestGroup) -> Self {
        NestPosition {
            group,
            index: Self::PROPERTY_INDEX,
        }
    }

    /// New position for intrinsic node.
    pub fn intrinsic(group: NestGroup) -> Self {
        NestPosition {
            group,
            index: Self::INTRINSIC_INDEX,
        }
    }
}
impl fmt::Debug for NestPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct IndexName(u16);
        impl fmt::Debug for IndexName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    NestPosition::INTRINSIC_INDEX => write!(f, "INTRINSIC_INDEX"),
                    NestPosition::PROPERTY_INDEX => write!(f, "PROPERTY_INDEX"),
                    i => write!(f, "{i}"),
                }
            }
        }

        f.debug_struct("NestPosition")
            .field("group", &self.group)
            .field("index", &IndexName(self.index))
            .finish()
    }
}

/// Property nest position group.
///
/// See [`NestPosition`] for more details.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NestGroup(u8);
impl NestGroup {
    /// Property defines a contextual value or variable.
    ///
    /// Usually these properties don't define behavior, they just configure the widget. A common pattern
    /// is defining all widget config as context vars, that are all used by an widget intrinsic node.
    ///
    /// These properties are not expected to affect layout or render, if they do some errors may be logged by the default widget base.
    pub const CONTEXT: Self = Self(0);
    /// Property defines an event handler, or state monitor, they are placed inside all context properties, so can be configured
    /// by context, but are still outside of the layout and render nodes.
    ///
    /// Event handlers can be notified before or after the inner child delegation, if handled before the event is said to be *preview*.
    /// Implementers can use this intrinsic feature of the UI tree to interrupt notification for child properties and widgets.
    ///
    /// These properties are not expected to affect layout or render, if they do some errors may be logged by the default widget base.
    pub const EVENT: Self = Self(Self::CONTEXT.0 + 1);
    /// Property defines the position and size of the widget inside the space made available by the parent widget.
    ///
    /// These properties must accumulatively affect the measure and layout, they must avoid rendering. The computed layout is
    /// usually rendered by the widget as a single transform, the layout properties don't need to render transforms.
    pub const LAYOUT: Self = Self(Self::EVENT.0 + 1);
    /// Property strongly enforces a widget size.
    ///
    /// Usually the widget final size is a side-effect of all the layout properties, but some properties may enforce a size, they
    /// can use this group to ensure that they are inside the other layout properties.
    pub const SIZE: Self = Self(Self::LAYOUT.0 + 1);
    /// Property renders a border visual.
    ///
    /// Borders are strictly coordinated, see the [`border`] module for more details. All nodes of this group
    /// may render at will, the renderer is already configured to apply the final layout and size.
    ///
    /// [`border`]: crate::border
    pub const BORDER: Self = Self(Self::SIZE.0 + 1);
    /// Property defines a visual of the  widget.
    ///
    /// This is the main render group, it usually defines things like a background fill, but it can render over child nodes simply
    /// by choosing to render after the render is delegated to the inner child.
    pub const FILL: Self = Self(Self::BORDER.0 + 1);
    /// Property defines contextual value or variable for the inner child or children widgets. Config set here does not affect
    /// the widget where it is set, it affects the descendants.
    pub const CHILD_CONTEXT: Self = Self(Self::FILL.0 + 1);
    /// Property starts defining the layout and size of the child or children widgets. These properties don't affect the layout
    /// of the widget where they are set. Some properties are functionally the same, only changing their effect depending on their
    /// group, the `margin` and `padding` properties are like this, `margin` is `layout` and `padding` is `child_layout`.
    pub const CHILD_LAYOUT: Self = Self(Self::CHILD_CONTEXT.0 + 1);

    /// All priorities, from outermost([`CONTEXT`]) to innermost([`CHILD_LAYOUT`]).
    ///
    /// [`CONTEXT`]: Self::CONTEXT
    /// [`CHILD_LAYOUT`]: Self::CHILD_LAYOUT
    pub const ITEMS: [Self; 8] = [
        Self::CONTEXT,
        Self::EVENT,
        Self::LAYOUT,
        Self::SIZE,
        Self::BORDER,
        Self::FILL,
        Self::CHILD_CONTEXT,
        Self::CHILD_LAYOUT,
    ];

    /// Group const name.
    pub const fn name(self) -> &'static str {
        if self.0 == Self::CONTEXT.0 {
            "CONTEXT"
        } else if self.0 == Self::EVENT.0 {
            "EVENT"
        } else if self.0 == Self::LAYOUT.0 {
            "LAYOUT"
        } else if self.0 == Self::SIZE.0 {
            "SIZE"
        } else if self.0 == Self::BORDER.0 {
            "BORDER"
        } else if self.0 == Self::FILL.0 {
            "FILL"
        } else if self.0 == Self::CHILD_CONTEXT.0 {
            "CHILD_CONTEXT"
        } else if self.0 == Self::CHILD_LAYOUT.0 {
            "CHILD_LAYOUT"
        } else {
            unreachable!()
        }
    }
}
impl fmt::Debug for NestGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "NestGroup::")?;
        }
        write!(f, "{}", self.name())
    }
}

/// Kind of property input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InputKind {
    /// Input is `impl IntoVar<T>`, build value is `BoxedVar<T>`.
    Var,
    /// Input and build value is `StateVar`.
    StateVar,
    /// Input is `impl IntoValue<T>`, build value is `T`.
    Value,
    /// Input is `impl UiNode`, build value is `ArcNode<BoxedUiNode>`.
    UiNode,
    /// Input is `impl UiNodeList`, build value is `ArcNodeList<BoxedUiNodeList>`.
    UiNodeList,
    /// Input is `impl WidgetHandler<A>`, build value is `ArcWidgetHandler<A>`.
    WidgetHandler,
}

/// Represents a [`WidgetHandler<A>`] that can be reused.
///
/// Note that [`hn_once!`] will still only be used once, and [`async_hn!`] tasks are bound to the specific widget
/// context that spawned then. This `struct` is cloneable to support handler properties in styleable widgets, but the
/// general expectation is that the handler will be used on one property instance at a time.
#[derive(Clone)]
pub struct ArcWidgetHandler<A: Clone + 'static>(Arc<Mutex<dyn WidgetHandler<A>>>);
impl<A: Clone + 'static> ArcWidgetHandler<A> {
    /// New from `handler`.
    pub fn new(handler: impl WidgetHandler<A>) -> Self {
        Self(Arc::new(Mutex::new(handler)))
    }
}
impl<A: Clone + 'static> WidgetHandler<A> for ArcWidgetHandler<A> {
    fn event(&mut self, ctx: &mut crate::context::WidgetContext, args: &A) -> bool {
        self.0.lock().event(ctx, args)
    }

    fn update(&mut self, ctx: &mut crate::context::WidgetContext) -> bool {
        self.0.lock().update(ctx)
    }
}

/// Represents a type erased [`ArcWidgetHandler<A>`].
pub trait AnyArcWidgetHandler: Any {
    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Clone the handler reference.
    fn clone_boxed(&self) -> Box<dyn AnyArcWidgetHandler>;
}
impl<A: Clone + 'static> AnyArcWidgetHandler for ArcWidgetHandler<A> {
    fn clone_boxed(&self) -> Box<dyn AnyArcWidgetHandler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// When builder for [`AnyArcWidgetHandler`] values.
///
/// This builder is used to generate a composite handler that redirects to active `when`
pub struct AnyWhenArcWidgetHandlerBuilder {
    default: Box<dyn AnyArcWidgetHandler>,
    conditions: Vec<(BoxedVar<bool>, Box<dyn AnyArcWidgetHandler>)>,
}
impl AnyWhenArcWidgetHandlerBuilder {
    /// New from default value.
    pub fn new(default: Box<dyn AnyArcWidgetHandler>) -> Self {
        Self {
            default,
            conditions: vec![],
        }
    }

    /// Push a conditional handler.
    pub fn push(&mut self, condition: BoxedVar<bool>, handler: Box<dyn AnyArcWidgetHandler>) {
        self.conditions.push((condition, handler));
    }

    /// Build the handler.
    pub fn build<A: Clone + 'static>(self) -> ArcWidgetHandler<A> {
        match self.default.into_any().downcast::<ArcWidgetHandler<A>>() {
            Ok(default) => {
                let mut conditions = Vec::with_capacity(self.conditions.len());
                for (c, h) in self.conditions {
                    match h.into_any().downcast::<ArcWidgetHandler<A>>() {
                        Ok(h) => conditions.push((c, *h)),
                        Err(_) => continue,
                    }
                }
                ArcWidgetHandler::new(WhenWidgetHandler {
                    default: *default,
                    conditions,
                })
            }
            Err(_) => panic!("unexpected build type in widget handler when builder"),
        }
    }
}

struct WhenWidgetHandler<A: Clone + 'static> {
    default: ArcWidgetHandler<A>,
    conditions: Vec<(BoxedVar<bool>, ArcWidgetHandler<A>)>,
}
impl<A: Clone + 'static> WidgetHandler<A> for WhenWidgetHandler<A> {
    fn event(&mut self, ctx: &mut crate::context::WidgetContext, args: &A) -> bool {
        for (c, h) in &mut self.conditions {
            if c.get() {
                return h.event(ctx, args);
            }
        }
        self.default.event(ctx, args)
    }

    fn update(&mut self, ctx: &mut crate::context::WidgetContext) -> bool {
        let mut pending = self.default.update(ctx);
        for (_, h) in &mut self.conditions {
            pending |= h.update(ctx);
        }
        pending
    }
}

/// Property build actions that must be applied to property args.
///
/// See [`PropertyNewArgs::build_actions`] for more details.
pub type PropertyBuildActions = Vec<Vec<Box<dyn AnyPropertyBuildAction>>>;

/// Data for property build actions associated with when conditions.
///
/// See [`PropertyNewArgs::build_actions_when_data`] for more details.
pub type PropertyBuildActionsWhenData = Vec<Vec<Option<WhenBuildActionData>>>;

/// Args for [`PropertyInfo::new`].
pub struct PropertyNewArgs {
    /// Defines the [`PropertyArgs::instance`] value.
    pub inst_info: PropertyInstInfo,

    /// The args vec must have a value for each input in the same order they appear in [`PropertyInfo::inputs`], types must match
    /// the input kind and type, the function panics if the types don't match or not all inputs are provided.
    ///
    /// The expected types for each [`InputKind`] are:
    ///
    /// | Kind                | Expected Type
    /// |---------------------|-------------------------------------------------
    /// | [`Var`]             | `Box<BoxedVar<T>>` or `Box<AnyWhenVarBuilder>`
    /// | [`StateVar`]        | `Box<StateVar>`
    /// | [`Value`]           | `Box<T>`
    /// | [`UiNode`]          | `Box<ArcNode<BoxedUiNode>>` or `Box<WhenUiNodeBuilder>`
    /// | [`UiNodeList`]      | `Box<ArcNodeList<BoxedUiNodeList>>` or `Box<WhenUiNodeListBuilder>`
    /// | [`WidgetHandler`]   | `Box<ArcWidgetHandler<A>>` or `Box<AnyWhenArcWidgetHandlerBuilder>`
    ///
    /// The expected type must be casted as `Box<dyn Any>`, the new function will downcast and unbox the args.
    ///
    /// [`Var`]: InputKind::Var
    /// [`StateVar`]: InputKind::StateVar
    /// [`Value`]: InputKind::Value
    /// [`UiNode`]: InputKind::UiNode
    /// [`UiNodeList`]: InputKind::UiNodeList
    /// [`WidgetHandler`]: InputKind::WidgetHandler
    pub args: Vec<Box<dyn Any>>,

    /// The property build actions can be empty or each item must contain one builder for each input in the same order they
    /// appear in [`PropertyInfo::inputs`], the function panics if the types don't match or not all inputs are provided.
    ///
    /// The expected types for each [`InputKind`] are:
    ///
    /// | Kind                | Expected Type
    /// |---------------------|-------------------------------------------------
    /// | [`Var`]             | `Box<PropertyBuildAction<BoxedVar<T>>>`
    /// | [`StateVar`]        | `Box<PropertyBuildAction<StateVar>>`
    /// | [`Value`]           | `Box<PropertyBuildAction<T>>`
    /// | [`UiNode`]          | `Box<PropertyBuildAction<ArcNode<BoxedUiNode>>>`
    /// | [`UiNodeList`]      | `Box<PropertyBuildAction<ArcNodeList<BoxedUiNodeList>>>`
    /// | [`WidgetHandler`]   | `Box<PropertyBuildAction<ArcWidgetHandler<A>>>`
    ///
    /// The expected type must be casted as `Box<dyn AnyPropertyBuildAction>`, the new function will downcast and unbox the args.
    ///
    /// [`Var`]: InputKind::Var
    /// [`StateVar`]: InputKind::StateVar
    /// [`Value`]: InputKind::Value
    /// [`UiNode`]: InputKind::UiNode
    /// [`UiNodeList`]: InputKind::UiNodeList
    /// [`WidgetHandler`]: InputKind::WidgetHandler
    pub build_actions: PropertyBuildActions,

    /// When build action data for each [`build_actions`].
    ///
    /// If not empty, each item is the [`PropertyBuildActionArgs::when_conditions_data`] for each action.
    ///
    /// [`build_actions`]: Self::build_actions
    pub build_actions_when_data: PropertyBuildActionsWhenData,
}

/// Property info.
///
/// You can use the [`property_info!`] macro to retrieve a property ID.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property nest position group.
    pub group: NestGroup,
    /// Property is "capture-only", no standalone implementation is provided, instantiating does not add a node, just returns the child.
    ///
    /// Note that all properties can be captured, but if this is `false` they provide an implementation that works standalone.
    pub capture: bool,

    /// Unique type ID that identifies the property implementation.
    pub impl_id: PropertyImplId,
    /// Property original name.
    pub name: &'static str,

    /// Property declaration location.
    pub location: SourceLocation,

    /// New default property args.
    ///
    /// This is `Some(_)` only if the `#[property(_, default(..))]` was set in the property declaration.
    pub default: Option<fn(PropertyInstInfo) -> Box<dyn PropertyArgs>>,

    /// New property args from dynamically typed args.
    ///
    /// # Instance
    ///
    /// This function outputs build property args, not a property node instance.
    /// You can use [`PropertyArgs::instantiate`] on the output to generate a property node from the args. If the
    /// property is known at compile time you can use [`property_args!`] to generate args instead, and you can just
    /// call the property function directly to instantiate a node.
    ///
    pub new: fn(PropertyNewArgs) -> Box<dyn PropertyArgs>,

    /// Property inputs info, always at least one.
    pub inputs: Box<[PropertyInput]>,
}
impl PropertyInfo {
    /// Gets the index that can be used to get a property value in [`PropertyArgs`].
    pub fn input_idx(&self, name: &str) -> Option<usize> {
        self.inputs.iter().position(|i| i.name == name)
    }
}

/// Property instance info.
#[derive(Debug, Clone)]
pub struct PropertyInstInfo {
    /// Property name in this instance.
    ///
    /// This can be different from [`PropertyInfo::name`] if the property was renamed by the widget.
    pub name: &'static str,

    /// Property instantiation location.
    pub location: SourceLocation,
}
impl PropertyInstInfo {
    /// No info.
    pub fn none() -> Self {
        PropertyInstInfo {
            name: "",
            location: SourceLocation {
                file: "",
                line: 0,
                column: 0,
            },
        }
    }

    /// Returns `true` if there is no instance info.
    pub fn is_none(&self) -> bool {
        self.name.is_empty()
    }
}

/// Property input info.
#[derive(Debug, Clone)]
pub struct PropertyInput {
    /// Input name.
    pub name: &'static str,
    /// Input kind.
    pub kind: InputKind,
    /// Type as defined by kind.
    pub ty: TypeId,
    /// Type name.
    pub ty_name: &'static str,
}
impl PropertyInput {
    /// Shorter [`ty_name`].
    ///
    /// [`ty_name`]: Self::ty_name
    pub fn display_ty_name(&self) -> Text {
        pretty_type_name::pretty_type_name_str(self.ty_name).into()
    }
}

/// Represents a property instantiation request.
pub trait PropertyArgs: Send + Sync {
    /// Clones the arguments.
    fn clone_boxed(&self) -> Box<dyn PropertyArgs>;

    /// Property info.
    fn property(&self) -> PropertyInfo;

    /// Instance info.
    fn instance(&self) -> PropertyInstInfo;

    /// Gets a [`InputKind::Var`] or [`InputKind::StateVar`].
    ///
    /// Is a `BoxedVar<T>` or `StateVar`.
    fn var(&self, i: usize) -> &dyn AnyVar {
        panic_input(&self.property(), i, InputKind::Var)
    }

    /// Gets a [`InputKind::StateVar`].
    fn state_var(&self, i: usize) -> &StateVar {
        panic_input(&self.property(), i, InputKind::StateVar)
    }

    /// Gets a [`InputKind::Value`].
    fn value(&self, i: usize) -> &dyn AnyVarValue {
        panic_input(&self.property(), i, InputKind::Value)
    }

    /// Gets a [`InputKind::UiNode`].
    fn ui_node(&self, i: usize) -> &ArcNode<BoxedUiNode> {
        panic_input(&self.property(), i, InputKind::UiNode)
    }

    /// Gets a [`InputKind::UiNodeList`].
    fn ui_node_list(&self, i: usize) -> &ArcNodeList<BoxedUiNodeList> {
        panic_input(&self.property(), i, InputKind::UiNodeList)
    }

    /// Gets a [`InputKind::WidgetHandler`].
    ///
    /// Is a `ArcWidgetHandler<A>`.
    fn widget_handler(&self, i: usize) -> &dyn AnyArcWidgetHandler {
        panic_input(&self.property(), i, InputKind::WidgetHandler)
    }

    /// Create a property instance with args clone or taken.
    ///
    /// If the property is [`PropertyInfo::capture`] the `child` is returned.
    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode;
}
impl dyn PropertyArgs + '_ {
    /// Unique ID.
    pub fn id(&self) -> PropertyId {
        PropertyId {
            impl_id: self.property().impl_id,
            name: self.instance().name,
        }
    }

    /// Gets a strongly typed [`value`].
    ///
    /// [`value`]: PropertyArgs::value
    pub fn downcast_value<T>(&self, i: usize) -> &T
    where
        T: VarValue,
    {
        self.value(i).as_any().downcast_ref::<T>().expect("cannot downcast value to type")
    }
    /// Gets a strongly typed [`var`].
    ///
    /// [`var`]: PropertyArgs::var
    pub fn downcast_var<T>(&self, i: usize) -> &BoxedVar<T>
    where
        T: VarValue,
    {
        self.var(i)
            .as_any()
            .downcast_ref::<BoxedVar<T>>()
            .expect("cannot downcast var to type")
    }

    /// Gets a strongly typed [`widget_handler`].
    ///
    /// [`widget_handler`]: PropertyArgs::widget_handler
    pub fn downcast_handler<A>(&self, i: usize) -> &ArcWidgetHandler<A>
    where
        A: 'static + Clone,
    {
        self.widget_handler(i)
            .as_any()
            .downcast_ref::<ArcWidgetHandler<A>>()
            .expect("cannot downcast handler to type")
    }

    /// Gets the property input as a debug variable.
    ///
    /// If the input is a variable the returned variable will update with it, if not it is a static print.
    pub fn live_debug(&self, i: usize) -> BoxedVar<Text> {
        let p = self.property();
        match p.inputs[i].kind {
            InputKind::Var => {
                let in_var = self.var(i);
                let out_var = var(formatx!("{:?}", in_var.get_any()));
                let wk_out_var = out_var.downgrade();
                in_var
                    .hook(Box::new(move |vars, _, value| {
                        if let Some(out_var) = wk_out_var.upgrade() {
                            let _ = out_var.set_any(vars, Box::new(formatx!("{:?}", value)));
                            true
                        } else {
                            false
                        }
                    }))
                    .perm();
                out_var.boxed()
            }
            InputKind::StateVar => self.state_var(i).map_debug().boxed(),
            InputKind::Value => LocalVar(formatx!("{:?}", self.value(i))).boxed(),
            InputKind::UiNode => LocalVar(Text::from_static("<impl UiNode>")).boxed(),
            InputKind::UiNodeList => LocalVar(Text::from_static("<impl UiNodeList>")).boxed(),
            InputKind::WidgetHandler => LocalVar(formatx!("<impl WidgetHandler<{}>>", p.inputs[i].display_ty_name())).boxed(),
        }
    }

    /// Gets the property input current value as a debug
    pub fn debug(&self, i: usize) -> Text {
        let p = self.property();
        match p.inputs[i].kind {
            InputKind::Var => formatx!("{:?}", self.var(i).get_any()),
            InputKind::StateVar => match self.state_var(i).get() {
                true => Text::from_static("true"),
                false => Text::from_static("false"),
            },
            InputKind::Value => formatx!("{:?}", self.value(i)),
            InputKind::UiNode => Text::from_static("<impl UiNode>"),
            InputKind::UiNodeList => Text::from_static("<impl UiNodeList>"),
            InputKind::WidgetHandler => formatx!("<impl WidgetHandler<{}>>", p.inputs[i].display_ty_name()),
        }
    }

    /// Call [`new`] with the same instance info and args, but with the `build_actions` and `build_actions_when_data`.
    ///
    /// [`new`]: PropertyInfo::new
    pub fn new_build(
        &self,
        build_actions: PropertyBuildActions,
        build_actions_when_data: PropertyBuildActionsWhenData,
    ) -> Box<dyn PropertyArgs> {
        let p = self.property();

        let mut args: Vec<Box<dyn Any>> = Vec::with_capacity(p.inputs.len());
        for (i, input) in p.inputs.iter().enumerate() {
            match input.kind {
                InputKind::Var => args.push(self.var(i).clone_any().double_boxed_any()),
                InputKind::StateVar => args.push(Box::new(self.state_var(i).clone())),
                InputKind::Value => args.push(self.value(i).clone_boxed().into_any()),
                InputKind::UiNode => args.push(Box::new(self.ui_node(i).clone())),
                InputKind::UiNodeList => args.push(Box::new(self.ui_node_list(i).clone())),
                InputKind::WidgetHandler => args.push(self.widget_handler(i).clone_boxed().into_any()),
            }
        }

        (p.new)(PropertyNewArgs {
            inst_info: self.instance(),
            args,
            build_actions,
            build_actions_when_data,
        })
    }
}

#[doc(hidden)]
pub fn panic_input(info: &PropertyInfo, i: usize, kind: InputKind) -> ! {
    if i > info.inputs.len() {
        panic!("index out of bounds, the input len is {}, but the index is {i}", info.inputs.len())
    } else if info.inputs[i].kind != kind {
        panic!(
            "invalid input request `{:?}`, but `{}` is `{:?}`",
            kind, info.inputs[i].name, info.inputs[i].kind
        )
    } else {
        panic!("invalid input `{}`", info.inputs[i].name)
    }
}

#[doc(hidden)]
pub fn var_input_to_args<T: VarValue>(var: impl IntoVar<T>) -> BoxedVar<T> {
    var.into_var().boxed()
}

#[doc(hidden)]
pub fn value_to_args<T: VarValue>(value: impl IntoValue<T>) -> T {
    value.into()
}

#[doc(hidden)]
pub fn ui_node_to_args(node: impl UiNode) -> ArcNode<BoxedUiNode> {
    ArcNode::new(node.boxed())
}

#[doc(hidden)]
pub fn ui_node_list_to_args(node_list: impl UiNodeList) -> ArcNodeList<BoxedUiNodeList> {
    ArcNodeList::new(node_list.boxed())
}

#[doc(hidden)]
pub fn widget_handler_to_args<A: Clone + 'static>(handler: impl WidgetHandler<A>) -> ArcWidgetHandler<A> {
    ArcWidgetHandler::new(handler)
}

#[doc(hidden)]
pub fn iter_input_build_actions<'a>(
    actions: &'a PropertyBuildActions,
    data: &'a PropertyBuildActionsWhenData,
    index: usize,
) -> impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])> {
    let mut actions = actions.iter();
    let mut data = data.iter();

    std::iter::from_fn(move || {
        let action = &*actions.next()?[index];
        let data = if let Some(data) = data.next() { &data[..] } else { &[None] };

        Some((action, data))
    })
}

fn apply_build_actions<'a, I: Any + Send>(
    mut item: I,
    mut actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> I {
    if let Some((action, data)) = actions.next() {
        let action = action
            .as_any()
            .downcast_ref::<PropertyBuildAction<I>>()
            .expect("property build action type did not match expected var type");

        item = action.build(PropertyBuildActionArgs {
            input: item,
            when_conditions_data: data,
        });
    }
    item
}

#[doc(hidden)]
pub fn new_dyn_var<'a, T: VarValue>(
    inputs: &mut std::vec::IntoIter<Box<dyn Any>>,
    actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> BoxedVar<T> {
    let item = inputs.next().expect("missing input");

    let item = match item.downcast::<AnyWhenVarBuilder>() {
        Ok(builder) => builder.contextualized_build::<T>().expect("invalid when builder").boxed(),
        Err(item) => *item.downcast::<BoxedVar<T>>().expect("input did not match expected var types"),
    };

    apply_build_actions(item, actions)
}

#[doc(hidden)]
pub fn new_dyn_ui_node<'a>(
    inputs: &mut std::vec::IntoIter<Box<dyn Any>>,
    actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> ArcNode<BoxedUiNode> {
    let item = inputs.next().expect("missing input");

    let item = match item.downcast::<WhenUiNodeBuilder>() {
        Ok(builder) => ArcNode::new(builder.build().boxed()),
        Err(item) => *item
            .downcast::<ArcNode<BoxedUiNode>>()
            .expect("input did not match expected UiNode types"),
    };

    apply_build_actions(item, actions)
}

#[doc(hidden)]
pub fn new_dyn_ui_node_list<'a>(
    inputs: &mut std::vec::IntoIter<Box<dyn Any>>,
    actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> ArcNodeList<BoxedUiNodeList> {
    let item = inputs.next().expect("missing input");

    let item = match item.downcast::<WhenUiNodeListBuilder>() {
        Ok(builder) => ArcNodeList::new(builder.build().boxed()),
        Err(item) => *item
            .downcast::<ArcNodeList<BoxedUiNodeList>>()
            .expect("input did not match expected UiNodeList types"),
    };

    apply_build_actions(item, actions)
}

#[doc(hidden)]
pub fn new_dyn_widget_handler<'a, A: Clone + 'static>(
    inputs: &mut std::vec::IntoIter<Box<dyn Any>>,
    actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> ArcWidgetHandler<A> {
    let item = inputs.next().expect("missing input");

    let item = match item.downcast::<AnyWhenArcWidgetHandlerBuilder>() {
        Ok(builder) => builder.build(),
        Err(item) => *item
            .downcast::<ArcWidgetHandler<A>>()
            .expect("input did not match expected WidgetHandler types"),
    };

    apply_build_actions(item, actions)
}

#[doc(hidden)]
pub fn new_dyn_other<'a, T: Any + Send>(
    inputs: &mut std::vec::IntoIter<Box<dyn Any>>,
    actions: impl Iterator<Item = (&'a dyn AnyPropertyBuildAction, &'a [Option<WhenBuildActionData>])>,
) -> T {
    let item = *inputs
        .next()
        .expect("missing input")
        .downcast::<T>()
        .expect("input did not match expected var type");

    apply_build_actions(item, actions)
}

/// Error value used in a reference to an [`UiNode`] property input is made in `when` expression.
///
/// Only variables and values can be referenced in `when` expression.
#[derive(Clone)]
pub struct UiNodeInWhenExprError;
impl fmt::Debug for UiNodeInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
impl fmt::Display for UiNodeInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot ref `impl UiNode` in when expression, only var and value properties allowed"
        )
    }
}
impl std::error::Error for UiNodeInWhenExprError {}

/// Error value used in a reference to an [`UiNodeList`] property input is made in `when` expression.
///
/// Only variables and values can be referenced in `when` expression.
#[derive(Clone)]
pub struct UiNodeListInWhenExprError;
impl fmt::Debug for UiNodeListInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
impl fmt::Display for UiNodeListInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot ref `impl UiNodeList` in when expression, only var and value properties allowed"
        )
    }
}
impl std::error::Error for UiNodeListInWhenExprError {}

/// Error value used in a reference to an [`UiNodeList`] property input is made in `when` expression.
///
/// Only variables and values can be referenced in `when` expression.
#[derive(Clone)]
pub struct WidgetHandlerInWhenExprError;
impl fmt::Debug for WidgetHandlerInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
impl fmt::Display for WidgetHandlerInWhenExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot ref `impl WidgetHandler<A>` in when expression, only var and value properties allowed"
        )
    }
}
impl std::error::Error for WidgetHandlerInWhenExprError {}

/*

 WIDGET

*/

/// Value that indicates the override importance of a property instance, higher overrides lower.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Importance(pub u32);
impl Importance {
    /// Importance of default values defined in the widget declaration.
    pub const WIDGET: Importance = Importance(1000);
    /// Importance of values defined in the widget instantiation.
    pub const INSTANCE: Importance = Importance(1000 * 10);
}
impl_from_and_into_var! {
    fn from(imp: u32) -> Importance {
        Importance(imp)
    }
}

unique_id_32! {
    /// Unique ID of a widget implementation.
    ///
    /// This ID identifies a widget implementation. Widgets are identified
    /// by this ID and a module path, see [`WidgetMod`].
    pub struct WidgetImplId;
}
impl fmt::Debug for WidgetImplId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WidgetImplId").field(&self.get()).finish()
    }
}

unique_id_32! {
    /// Unique ID of a property implementation.
    ///
    /// This ID identifies a property implementation, disregarding renames. Properties are identified
    /// by this ID and a name string, see [`PropertyId`].
    pub struct PropertyImplId;
}
impl fmt::Debug for PropertyImplId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PropertyImplId").field(&self.get()).finish()
    }
}

/// Unique identifier of a property, properties with the same id override each other in a widget and are joined
/// into a single instance is assigned in when blocks.
///
/// You can use the [`property_id!`] macro to retrieve a property ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId {
    /// The [`PropertyInfo::impl_id`].
    pub impl_id: PropertyImplId,
    /// The [`PropertyInstInfo::name`].
    pub name: &'static str,
}

/// Unique identifier of a widget module.
///
/// Equality and hash is defined by the `impl_id` only.
///
/// You can use the [`widget_mod!`] macro to get the mod info of a widget.
#[derive(Clone, Copy)]
pub struct WidgetMod {
    /// The widget module unique ID.
    pub impl_id: WidgetImplId,
    /// The widget public module path.
    pub path: &'static str,
    /// Source code location.
    pub location: SourceLocation,
}
impl WidgetMod {
    /// Get the last part of the path.
    pub fn name(&self) -> &'static str {
        self.path.rsplit_once(':').map(|(_, n)| n).unwrap_or(self.path)
    }
}
impl PartialEq for WidgetMod {
    fn eq(&self, other: &Self) -> bool {
        self.impl_id == other.impl_id
    }
}
impl Eq for WidgetMod {}
impl std::hash::Hash for WidgetMod {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.impl_id.hash(state);
    }
}

/// Represents what member and how it was accessed in a [`WhenInput`].
#[derive(Clone, Copy, Debug)]
pub enum WhenInputMember {
    /// Member was accessed by name.
    Named(&'static str),
    /// Member was accessed by index.
    Index(usize),
}

/// Input var read in a `when` condition expression.
#[derive(Clone)]
pub struct WhenInput {
    /// Property.
    pub property: PropertyId,
    /// What member and how it was accessed for this input.
    pub member: WhenInputMember,
    /// Input var.
    pub var: WhenInputVar,
    /// Constructor that generates the default property instance.
    pub property_default: Option<fn(PropertyInstInfo) -> Box<dyn PropertyArgs>>,
}
impl fmt::Debug for WhenInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhenInput")
            .field("property", &self.property)
            .field("member", &self.member)
            .finish_non_exhaustive()
    }
}

enum WhenInputVarActual<T: VarValue> {
    None,
    Some(BoxedVar<T>),
}
trait AnyWhenInputVarInner: Any + Send {
    fn as_any(&self) -> &dyn Any;
    fn is_some(&self) -> bool;
    fn set(&mut self, var: BoxedAnyVar);
}
impl<T: VarValue> AnyWhenInputVarInner for WhenInputVarActual<T> {
    fn set(&mut self, var: BoxedAnyVar) {
        let var = var
            .double_boxed_any()
            .downcast::<BoxedVar<T>>()
            .expect("incorrect when input var type");
        *self = Self::Some(var);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }
}

/// Represents a [`WhenInput`] variable that can be rebound.
#[derive(Clone)]
pub struct WhenInputVar {
    var: Arc<Mutex<dyn AnyWhenInputVarInner>>,
}
impl WhenInputVar {
    /// New input setter and input var.
    ///
    /// Trying to use the input var before [`can_use`] is `true` will panic. The input var will pull
    /// the actual var on first use of each instance, that means you can *refresh* the input var by cloning.
    ///
    /// [`can_use`]: Self::can_use
    pub fn new<T: VarValue>() -> (Self, impl Var<T>) {
        let rc: Arc<Mutex<dyn AnyWhenInputVarInner>> = Arc::new(Mutex::new(WhenInputVarActual::<T>::None));
        (
            WhenInputVar { var: rc.clone() },
            crate::var::types::ContextualizedVar::new(Arc::new(move || {
                match rc.lock().as_any().downcast_ref::<WhenInputVarActual<T>>().unwrap() {
                    WhenInputVarActual::Some(var) => var.read_only(),
                    WhenInputVarActual::None => panic!("when expr input not inited"),
                }
            })),
        )
    }

    /// Returns `true` an actual var is configured, trying to use the input var when this is `false` panics.
    pub fn can_use(&self) -> bool {
        self.var.lock().is_some()
    }

    /// Set the actual input var.
    ///
    /// After this call [`can_use`] is `true`. Note that if the input was already set and used that instance of the
    /// var will not be replaced, input vars pull the `var` on the first use of the instance, that means that a new
    /// clone of the input var must be made to
    ///
    /// [`can_use`]: Self::can_use
    /// [`new`]: Self::new
    pub fn set(&self, var: BoxedAnyVar) {
        self.var.lock().set(var);
    }
}

type WhenBuildActionData = Arc<dyn Any + Send + Sync>;
type WhenBuildDefaultAction = Arc<dyn Fn() -> Vec<Box<dyn AnyPropertyBuildAction>> + Send + Sync>;

/// Data for a custom when build action associated with an [`WhenInfo`].
#[derive(Clone)]
pub struct WhenBuildAction {
    /// Data for all inputs.
    pub data: WhenBuildActionData,
    /// Closure that generates the default build actions, used when the final widget has no build action instance.
    ///
    /// The closure must generate an action that behaves like it is not present and then activates when the condition data activates.
    ///
    /// If the final widget has no action and all when data for it has no default, the data is ignored.
    pub default_action: Option<WhenBuildDefaultAction>,
}
impl WhenBuildAction {
    /// New from strongly typed values.
    pub fn new<D, F>(data: D, default_action: F) -> Self
    where
        D: Any + Send + Sync + 'static,
        F: Fn() -> Vec<Box<dyn AnyPropertyBuildAction>> + Send + Sync + 'static,
    {
        Self {
            data: Arc::new(data),
            default_action: Some(Arc::new(default_action)),
        }
    }

    /// New from data, is only used if the action is provided by another data or the widget builder.
    pub fn new_no_default(data: impl Any + Send + Sync + 'static) -> Self {
        Self {
            data: Arc::new(data),
            default_action: None,
        }
    }
}

/// Represents a `when` block in a widget.
#[derive(Clone)]
pub struct WhenInfo {
    /// Properties referenced in the when condition expression.
    ///
    /// They are type erased `BoxedVar<T>` instances that are *late-inited*, other variable references (`*#{var}`) are imbedded in
    /// the build expression and cannot be modified. Note that the [`state`] sticks to the first *late-inited* vars that it uses,
    /// the variable only updates after clone, this cloning happens naturally when instantiating a widget more then once.
    ///
    /// [`state`]: Self::state
    pub inputs: Box<[WhenInput]>,

    /// Output of the when expression.
    ///
    /// # Panics
    ///
    /// If used when [`can_use`] is `false`.
    ///
    /// [`can_use`]: Self::can_use
    pub state: BoxedVar<bool>,

    /// Properties assigned in the when block, in the build widget they are joined with the default value and assigns
    /// from other when blocks into a single property instance set to `when_var!` inputs.
    pub assigns: Vec<Box<dyn PropertyArgs>>,

    /// Data associated with the when condition in the build action.
    pub build_action_data: Vec<((PropertyId, &'static str), WhenBuildAction)>,

    /// The condition expression code.
    pub expr: &'static str,

    /// When declaration location.
    pub location: SourceLocation,
}
impl WhenInfo {
    /// Returns `true` if the [`state`] var is valid because it does not depend of any property input or all
    /// property inputs are inited with a value or have a default.
    ///
    /// [`state`]: Self::state
    pub fn can_use(&self) -> bool {
        self.inputs.iter().all(|i| i.var.can_use())
    }
}
impl fmt::Debug for WhenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugBuildActions<'a>(&'a WhenInfo);
        impl<'a> fmt::Debug for DebugBuildActions<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.build_action_data.iter().map(|(k, _)| k)).finish()
            }
        }

        f.debug_struct("WhenInfo")
            .field("inputs", &self.inputs)
            .field("state", &self.state.debug())
            .field("assigns", &self.assigns)
            .field("build_action_data", &DebugBuildActions(self))
            .field("expr", &self.expr)
            .finish()
    }
}
impl Clone for Box<dyn PropertyArgs> {
    fn clone(&self) -> Self {
        PropertyArgs::clone_boxed(&**self)
    }
}
impl<'a> fmt::Debug for &'a dyn PropertyArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("dyn PropertyArgs")
            .field("property", &self.property())
            .field("instance", &self.instance())
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for Box<dyn PropertyArgs> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("dyn PropertyArgs")
            .field("property", &self.property())
            .field("instance", &self.instance())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct WidgetItemPositioned {
    position: NestPosition,
    insert_idx: u32,
    item: WidgetItem,
}
impl WidgetItemPositioned {
    fn sort_key(&self) -> (NestPosition, u32) {
        (self.position, self.insert_idx)
    }
}

#[derive(Clone, Debug)]
struct WhenItemPositioned {
    importance: Importance,
    insert_idx: u32,
    when: WhenInfo,
}
impl WhenItemPositioned {
    fn sort_key(&self) -> (Importance, u32) {
        (self.importance, self.insert_idx)
    }
}

enum WidgetItem {
    Property {
        importance: Importance,
        args: Box<dyn PropertyArgs>,
        captured: bool,
    },
    Intrinsic {
        name: &'static str,
        new: Box<dyn FnOnce(BoxedUiNode) -> BoxedUiNode + Send + Sync>,
    },
}
impl Clone for WidgetItem {
    fn clone(&self) -> Self {
        match self {
            Self::Property {
                importance,
                args,
                captured,
            } => Self::Property {
                importance: *importance,
                captured: *captured,
                args: args.clone(),
            },
            Self::Intrinsic { .. } => unreachable!("only WidgetBuilder clones, and it does not insert intrinsic"),
        }
    }
}

// [(PropertyId, "action-key") => (Importance, Vec<{action for each input}>)]
type PropertyBuildActionsMap = LinearMap<(PropertyId, &'static str), (Importance, Vec<Box<dyn AnyPropertyBuildAction>>)>;
type PropertyBuildActionsVec = Vec<((PropertyId, &'static str), (Importance, Vec<Box<dyn AnyPropertyBuildAction>>))>;

/// Widget instance builder.
pub struct WidgetBuilder {
    widget_mod: WidgetMod,

    insert_idx: u32,
    p: WidgetBuilderProperties,
    unset: LinearMap<PropertyId, Importance>,

    whens: Vec<WhenItemPositioned>,
    when_insert_idx: u32,

    p_build_actions: PropertyBuildActionsMap,
    p_build_actions_unset: LinearMap<(PropertyId, &'static str), Importance>,

    build_actions: Vec<Arc<Mutex<dyn FnMut(&mut WidgetBuilding) + Send>>>,

    custom_build: Option<Arc<Mutex<dyn FnMut(WidgetBuilder) -> BoxedUiNode + Send>>>,
}
impl Clone for WidgetBuilder {
    fn clone(&self) -> Self {
        Self {
            widget_mod: self.widget_mod,
            p: WidgetBuilderProperties { items: self.items.clone() },
            p_build_actions: self.p_build_actions.clone(),
            insert_idx: self.insert_idx,
            unset: self.unset.clone(),
            p_build_actions_unset: self.p_build_actions_unset.clone(),
            whens: self.whens.clone(),
            when_insert_idx: self.when_insert_idx,
            build_actions: self.build_actions.clone(),
            custom_build: self.custom_build.clone(),
        }
    }
}
impl fmt::Debug for WidgetBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct PropertiesDebug<'a>(&'a WidgetBuilderProperties);
        impl<'a> fmt::Debug for PropertiesDebug<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.properties()).finish()
            }
        }
        f.debug_struct("WidgetBuilder")
            .field("properties", &PropertiesDebug(&self.p))
            .field("unset", &self.unset)
            .field("whens", &self.whens)
            .field("build_actions.len", &self.build_actions.len())
            .field("is_custom_build", &self.is_custom_build())
            .finish()
    }
}
impl WidgetBuilder {
    /// New empty default.
    pub fn new(widget: WidgetMod) -> Self {
        Self {
            widget_mod: widget,
            p: WidgetBuilderProperties { items: Default::default() },
            insert_idx: 0,
            unset: Default::default(),
            whens: Default::default(),
            p_build_actions: Default::default(),
            p_build_actions_unset: Default::default(),
            when_insert_idx: 0,
            build_actions: Default::default(),
            custom_build: Default::default(),
        }
    }

    /// The widget that started this builder.
    pub fn widget_mod(&self) -> WidgetMod {
        self.widget_mod
    }

    /// Insert/override a property.
    ///
    /// You can use the [`property_args!`] macro to collect args for a property.
    pub fn push_property(&mut self, importance: Importance, args: Box<dyn PropertyArgs>) {
        let pos = NestPosition::property(args.property().group);
        self.push_property_positioned(importance, pos, args);
    }

    /// Insert property with custom nest position.
    pub fn push_property_positioned(&mut self, importance: Importance, position: NestPosition, args: Box<dyn PropertyArgs>) {
        let insert_idx = self.insert_idx;
        self.insert_idx = insert_idx.wrapping_add(1);

        let property_id = args.id();
        if let Some(i) = self.p.property_index(property_id) {
            match &self.p.items[i].item {
                WidgetItem::Property { importance: imp, .. } => {
                    if *imp <= importance {
                        // override
                        self.p.items[i] = WidgetItemPositioned {
                            position,
                            insert_idx,
                            item: WidgetItem::Property {
                                importance,
                                args,
                                captured: false,
                            },
                        };
                    }
                }
                WidgetItem::Intrinsic { .. } => unreachable!(),
            }
        } else {
            if let Some(imp) = self.unset.get(&property_id) {
                if *imp >= importance {
                    return; // unset blocks.
                }
            }
            self.p.items.push(WidgetItemPositioned {
                position,
                insert_idx,
                item: WidgetItem::Property {
                    importance,
                    args,
                    captured: false,
                },
            });
        }
    }

    /// Insert a `when` block.
    pub fn push_when(&mut self, importance: Importance, mut when: WhenInfo) {
        let insert_idx = self.when_insert_idx;
        self.when_insert_idx = insert_idx.wrapping_add(1);

        when.assigns.retain(|a| {
            if let Some(imp) = self.unset.get(&a.id()) {
                *imp < importance
            } else {
                true
            }
        });

        if !when.assigns.is_empty() {
            self.whens.push(WhenItemPositioned {
                importance,
                insert_idx,
                when,
            });
        }
    }

    /// Insert a `name = unset!;` property.
    pub fn push_unset(&mut self, importance: Importance, property_id: PropertyId) {
        let check;
        match self.unset.entry(property_id) {
            linear_map::Entry::Occupied(mut e) => {
                let i = e.get_mut();
                check = *i < importance;
                *i = importance;
            }
            linear_map::Entry::Vacant(e) => {
                check = true;
                e.insert(importance);
            }
        }

        if check {
            if let Some(i) = self.p.property_index(property_id) {
                match &self.p.items[i].item {
                    WidgetItem::Property { importance: imp, .. } => {
                        if *imp <= importance {
                            self.p.items.swap_remove(i);
                        }
                    }
                    WidgetItem::Intrinsic { .. } => unreachable!(),
                }
            }

            self.whens.retain_mut(|w| {
                if w.importance <= importance {
                    w.when.assigns.retain(|a| a.id() != property_id);
                    !w.when.assigns.is_empty()
                } else {
                    true
                }
            });
        }
    }

    /// Add or override custom builder actions that are called to finalize the inputs for a property.
    ///
    /// The `importance` overrides previous build action of the same name and property. The `input_actions` vec must
    /// contain one action for each property input.
    pub fn push_property_build_action(
        &mut self,
        property_id: PropertyId,
        action_name: &'static str,
        importance: Importance,
        input_actions: Vec<Box<dyn AnyPropertyBuildAction>>,
    ) {
        match self.p_build_actions.entry((property_id, action_name)) {
            linear_map::Entry::Occupied(mut e) => {
                if e.get().0 < importance {
                    e.insert((importance, input_actions));
                }
            }
            linear_map::Entry::Vacant(e) => {
                if let Some(imp) = self.p_build_actions_unset.get(&(property_id, action_name)) {
                    if *imp >= importance {
                        // blocked by unset
                        return;
                    }
                }
                e.insert((importance, input_actions));
            }
        }
    }

    /// Insert a [property build action] filter.
    ///
    /// [property build action]: Self::push_property_build_action
    pub fn push_unset_property_build_action(&mut self, property_id: PropertyId, action_name: &'static str, importance: Importance) {
        let mut check = false;
        match self.p_build_actions_unset.entry((property_id, action_name)) {
            linear_map::Entry::Occupied(mut e) => {
                if *e.get() < importance {
                    e.insert(importance);
                    check = true;
                }
            }
            linear_map::Entry::Vacant(e) => {
                e.insert(importance);
                check = true;
            }
        }
        if check {
            self.p_build_actions.retain(|_, (imp, _)| *imp > importance);
        }
    }

    /// Remove all registered property build actions.
    pub fn clear_property_build_actions(&mut self) {
        self.p_build_actions.clear();
    }

    /// Add an `action` closure that is called every time this builder or a clone of it builds a widget instance.
    pub fn push_build_action(&mut self, action: impl FnMut(&mut WidgetBuilding) + Send + 'static) {
        self.build_actions.push(Arc::new(Mutex::new(action)))
    }

    /// Remove all registered build actions.
    pub fn clear_build_actions(&mut self) {
        self.build_actions.clear();
    }

    /// Returns `true` if a custom build handler is registered.
    pub fn is_custom_build(&self) -> bool {
        self.custom_build.is_some()
    }

    /// Set a `build` closure to run instead of [`default_build`] when [`build`] is called.
    ///
    /// Overrides the previous custom build, if any was set.
    ///
    /// [`build`]: Self::build
    /// [`default_build`]: Self::default_build
    pub fn set_custom_build<R: UiNode>(&mut self, mut build: impl FnMut(WidgetBuilder) -> R + Send + 'static) {
        self.custom_build = Some(Arc::new(Mutex::new(move |b| build(b).boxed())));
    }

    /// Remove the custom build handler, if any was set.
    pub fn clear_custom_build(&mut self) {
        self.custom_build = None;
    }

    /// Apply `other` over `self`.
    pub fn extend(&mut self, other: WidgetBuilder) {
        for (id, imp) in other.unset {
            self.push_unset(imp, id);
        }

        for ((id, name), imp) in other.p_build_actions_unset {
            self.push_unset_property_build_action(id, name, imp);
        }

        for WidgetItemPositioned { position, item, .. } in other.p.items {
            match item {
                WidgetItem::Property {
                    importance,
                    args,
                    captured,
                } => {
                    debug_assert!(!captured);
                    self.push_property_positioned(importance, position, args);
                }
                WidgetItem::Intrinsic { .. } => unreachable!(),
            }
        }

        for w in other.whens {
            self.push_when(w.importance, w.when);
        }

        for ((id, name), (imp, action)) in other.p_build_actions {
            self.push_property_build_action(id, name, imp, action);
        }

        for act in other.build_actions {
            self.build_actions.push(act);
        }

        if let Some(c) = other.custom_build {
            self.custom_build = Some(c);
        }
    }

    /// If any property is present in the builder.
    pub fn has_properties(&self) -> bool {
        !self.p.items.is_empty()
    }

    /// If any unset filter is present in the builder.
    pub fn has_unsets(&self) -> bool {
        !self.unset.is_empty()
    }

    /// If any when block is present in the builder.
    pub fn has_whens(&self) -> bool {
        !self.whens.is_empty()
    }

    /// Move all `properties` to a new builder.
    ///
    /// The properties are removed from `self`, any `when` assign is also moved, properties used in [`WhenInput`] that
    /// affect the properties are cloned or moved into the new builder.
    ///
    /// Note that properties can depend on others in the widget contextually, this is not preserved on split-off.
    /// The canonical usage of split-off is the `style` property, that dynamically (re)builds widgets and is it-self a variable
    /// that can be affected by `when` blocks to a limited extent.
    pub fn split_off(&mut self, properties: impl IntoIterator<Item = PropertyId>, out: &mut WidgetBuilder) {
        self.split_off_impl(properties.into_iter().collect(), out)
    }
    fn split_off_impl(&mut self, properties: LinearSet<PropertyId>, out: &mut WidgetBuilder) {
        let mut found = 0;

        // move properties
        let mut i = 0;
        while i < self.items.len() && found < properties.len() {
            match &self.items[i].item {
                WidgetItem::Property { args, .. } if properties.contains(&args.id()) => match self.items.swap_remove(i) {
                    WidgetItemPositioned {
                        position,
                        item: WidgetItem::Property { importance, args, .. },
                        ..
                    } => {
                        out.push_property_positioned(importance, position, args);
                        found += 1;
                    }
                    _ => unreachable!(),
                },
                _ => {
                    i += 1;
                    continue;
                }
            }
        }

        i = 0;
        while i < self.whens.len() {
            // move when assigns
            let mut ai = 0;
            let mut moved_assigns = vec![];
            while ai < self.whens[i].when.assigns.len() {
                if properties.contains(&self.whens[i].when.assigns[ai].id()) {
                    let args = self.whens[i].when.assigns.remove(ai);
                    moved_assigns.push(args);
                } else {
                    ai += 1;
                }
            }

            if !moved_assigns.is_empty() {
                let out_imp;
                let out_when;
                if self.whens[i].when.assigns.is_empty() {
                    // moved all assigns from block, move block
                    let WhenItemPositioned { importance, mut when, .. } = self.whens.remove(i);
                    when.assigns = moved_assigns;

                    out_imp = importance;
                    out_when = when;
                } else {
                    // when block still used, clone block header for moved assigns.
                    let WhenItemPositioned { importance, when, .. } = &self.whens[i];
                    out_imp = *importance;
                    out_when = WhenInfo {
                        inputs: when.inputs.clone(),
                        state: when.state.clone(),
                        assigns: moved_assigns,
                        build_action_data: when.build_action_data.clone(),
                        expr: when.expr,
                        location: when.location,
                    };

                    i += 1;
                };

                // clone when input properties that are "manually" set.
                for input in out_when.inputs.iter() {
                    if let Some(i) = self.property_index(input.property) {
                        match &self.items[i] {
                            WidgetItemPositioned {
                                position,
                                item: WidgetItem::Property { importance, args, .. },
                                ..
                            } => {
                                out.push_property_positioned(*importance, *position, args.clone());
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                out.push_when(out_imp, out_when);
            } else {
                i += 1;
            }
        }

        // move unsets
        for id in properties {
            if let Some(imp) = self.unset.remove(&id) {
                out.push_unset(imp, id);
            }
        }
    }

    /// Instantiate the widget.
    ///
    /// If a custom build is set it is run, unless it is already running, otherwise the [`default_build`] is called.
    ///
    /// [`default_build`]: Self::default_build
    pub fn build(self) -> BoxedUiNode {
        if let Some(cust) = self.custom_build.clone() {
            match cust.try_lock() {
                Some(mut c) => c(self),
                None => self.default_build(),
            }
        } else {
            self.default_build()
        }
    }

    /// Instantiate the widget.
    ///
    /// Runs all build actions
    pub fn default_build(self) -> BoxedUiNode {
        #[cfg(inspector)]
        let builder = self.clone();

        let mut building = WidgetBuilding {
            #[cfg(inspector)]
            builder: Some(builder),
            #[cfg(trace_widget)]
            trace_widget: true,
            #[cfg(trace_wgt_item)]
            trace_wgt_item: true,

            widget_mod: self.widget_mod,
            p: self.p,
            child: None,
        };

        let mut p_build_actions = self.p_build_actions.into();

        if !self.whens.is_empty() {
            building.build_whens(self.whens, &mut p_build_actions);
        }

        if !p_build_actions.is_empty() {
            building.build_p_actions(p_build_actions);
        }

        for action in self.build_actions {
            (action.lock())(&mut building);
        }

        building.build()
    }
}
impl ops::Deref for WidgetBuilder {
    type Target = WidgetBuilderProperties;

    fn deref(&self) -> &Self::Target {
        &self.p
    }
}
impl ops::DerefMut for WidgetBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.p
    }
}

/// Represents a finalizing [`WidgetBuilder`].
///
/// Widgets can register a [build action] to get access to this, it provides an opportunity
/// to remove or capture the final properties of an widget, after they have all been resolved and `when` assigns generated.
/// Build actions can also define the child node, intrinsic nodes and a custom builder.
///
/// [build action]: WidgetBuilder::push_build_action
pub struct WidgetBuilding {
    #[cfg(inspector)]
    builder: Option<WidgetBuilder>,
    #[cfg(trace_widget)]
    trace_widget: bool,
    #[cfg(trace_wgt_item)]
    trace_wgt_item: bool,

    widget_mod: WidgetMod,
    p: WidgetBuilderProperties,
    child: Option<BoxedUiNode>,
}
impl WidgetBuilding {
    /// The widget that started this builder.
    pub fn widget_mod(&self) -> WidgetMod {
        self.widget_mod
    }

    /// If an innermost node is defined.
    ///
    /// If `false` by the end of build the [`NilUiNode`] is used as the innermost node.
    pub fn has_child(&self) -> bool {
        self.child.is_some()
    }

    /// Set/replace the innermost node of the widget.
    pub fn set_child(&mut self, node: impl UiNode) {
        self.child = Some(node.boxed());
    }

    /// Don't insert the inspector node and inspector metadata on build.
    ///
    /// The inspector metadata is inserted by default when `feature="inspector"` is active.
    #[cfg(inspector)]
    pub fn disable_inspector(&mut self) {
        self.builder = None;
    }

    /// Don't insert the widget trace node on build.
    ///
    /// The trace node is inserted by default when `feature="trace_widget"` is active.
    #[cfg(trace_widget)]
    pub fn disable_trace_widget(&mut self) {
        self.trace_widget = false;
    }

    /// Don't insert property/intrinsic trace nodes on build.
    ///
    /// The trace nodes is inserted by default when `feature="trace_wgt_item"` is active.
    #[cfg(trace_wgt_item)]
    pub fn disable_trace_wgt_item(&mut self) {
        self.trace_wgt_item = false;
    }

    /// Insert intrinsic node, that is a core functionality node of the widget that cannot be overridden.
    ///
    /// The `name` is used for inspector/trace only, intrinsic nodes are not deduplicated.
    pub fn push_intrinsic<I: UiNode>(
        &mut self,
        group: NestGroup,
        name: &'static str,
        intrinsic: impl FnOnce(BoxedUiNode) -> I + Send + Sync + 'static,
    ) {
        self.push_intrinsic_positioned(NestPosition::intrinsic(group), name, intrinsic)
    }

    /// Insert intrinsic node with custom nest position.
    ///
    /// The `name` is used for inspector/trace only, intrinsic nodes are not deduplicated.
    pub fn push_intrinsic_positioned<I: UiNode>(
        &mut self,
        position: NestPosition,
        name: &'static str,
        intrinsic: impl FnOnce(BoxedUiNode) -> I + Send + Sync + 'static,
    ) {
        self.items.push(WidgetItemPositioned {
            position,
            insert_idx: u32::MAX,
            item: WidgetItem::Intrinsic {
                name,
                new: Box::new(move |n| intrinsic(n).boxed()),
            },
        });
    }

    /// Removes the property, returns `(importance, position, args, captured)`.
    ///
    /// Note that if the property was captured a clone of the args is already in an intrinsic node and will still be used in the widget.
    pub fn remove_property(&mut self, property_id: PropertyId) -> Option<BuilderProperty> {
        if let Some(i) = self.property_index(property_id) {
            match self.items.swap_remove(i) {
                WidgetItemPositioned {
                    position,
                    item:
                        WidgetItem::Property {
                            importance,
                            args,
                            captured,
                        },
                    ..
                } => Some(BuilderProperty {
                    importance,
                    position,
                    args,
                    captured,
                }),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    /// Flags the property as captured and returns a reference to it.
    ///
    /// Note that captured properties are not instantiated in the final build, but they also are not removed like *unset*.
    /// A property can be "captured" more then once, and if the `inspector` feature is enabled they can be inspected.
    pub fn capture_property(&mut self, property_id: PropertyId) -> Option<BuilderPropertyRef> {
        self.capture_property_impl(property_id)
    }

    /// Flags the property as captured and downcast the input var.
    pub fn capture_var<T>(&mut self, property_id: PropertyId) -> Option<BoxedVar<T>>
    where
        T: VarValue,
    {
        let p = self.capture_property(property_id)?;
        let var = p.args.downcast_var::<T>(0).clone();
        Some(var)
    }

    /// Flags the property as captured and downcast the input var, or calls `or_else` to generate a fallback.
    pub fn capture_var_or_else<T>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> T) -> BoxedVar<T>
    where
        T: VarValue,
    {
        match self.capture_var::<T>(property_id) {
            Some(var) => var,
            None => or_else().into_var().boxed(),
        }
    }

    /// Flags the property as captured and downcast the input var, returns a new one with the default value.
    pub fn capture_var_or_default<T>(&mut self, property_id: PropertyId) -> BoxedVar<T>
    where
        T: VarValue + Default,
    {
        self.capture_var_or_else(property_id, T::default)
    }

    /// Flags the property as captured and get the input node.
    pub fn capture_ui_node(&mut self, property_id: PropertyId) -> Option<BoxedUiNode> {
        let p = self.capture_property(property_id)?;
        let node = p.args.ui_node(0).take_on_init().boxed();
        Some(node)
    }

    /// Flags the property as captured and get the input node, or calls `or_else` to generate a fallback node.
    pub fn capture_ui_node_or_else<F>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> F) -> BoxedUiNode
    where
        F: UiNode,
    {
        match self.capture_ui_node(property_id) {
            Some(u) => u,
            None => or_else().boxed(),
        }
    }

    /// Flags the property as captured and get the input list.
    pub fn capture_ui_node_list(&mut self, property_id: PropertyId) -> Option<BoxedUiNodeList> {
        let p = self.capture_property(property_id)?;
        let list = p.args.ui_node_list(0).take_on_init().boxed();
        Some(list)
    }

    /// Flags the property as captured and get the input list, or calls `or_else` to generate a fallback list.
    pub fn capture_ui_node_list_or_else<F>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> F) -> BoxedUiNodeList
    where
        F: UiNodeList,
    {
        match self.capture_ui_node_list(property_id) {
            Some(u) => u,
            None => or_else().boxed(),
        }
    }

    /// Flags the property as captured and get the input list, or returns an empty list.
    pub fn capture_ui_node_list_or_empty(&mut self, property_id: PropertyId) -> BoxedUiNodeList {
        self.capture_ui_node_list_or_else(property_id, Vec::<BoxedUiNode>::new)
    }

    /// Flags the property as captured and downcast the input handler.
    pub fn capture_widget_handler<A: Clone + 'static>(&mut self, property_id: PropertyId) -> Option<ArcWidgetHandler<A>> {
        let p = self.capture_property(property_id)?;
        let handler = p.args.downcast_handler::<A>(0).clone();
        Some(handler)
    }

    fn build_whens(&mut self, mut whens: Vec<WhenItemPositioned>, build_actions: &mut PropertyBuildActionsVec) {
        whens.sort_unstable_by_key(|w| w.sort_key());

        struct Input<'a> {
            input: &'a WhenInput,
            item_idx: usize,
        }
        let mut inputs = vec![];

        struct Assign {
            item_idx: usize,
            builder: Vec<Box<dyn Any>>,
            when_count: usize,
            /// map of key:action set in the property, in at least one when, and value:Vec of data for each when in order and
            /// Option of default action.
            actions_data: LinearMap<&'static str, (Vec<Option<WhenBuildActionData>>, Option<WhenBuildDefaultAction>)>,
        }
        let mut assigns = LinearMap::new();

        // rev so that the last when overrides others, the WhenVar returns the first true condition.
        'when: for WhenItemPositioned { when, .. } in whens.iter().rev() {
            // bind inputs.
            let valid_inputs = inputs.len();
            let valid_items = self.p.items.len();
            for input in when.inputs.iter() {
                if let Some(i) = self.property_index(input.property) {
                    inputs.push(Input { input, item_idx: i })
                } else if let Some(default) = input.property_default {
                    let args = default(PropertyInstInfo {
                        name: input.property.name,
                        location: when.location,
                    });
                    self.p.items.push(WidgetItemPositioned {
                        position: NestPosition::property(args.property().group),
                        insert_idx: u32::MAX,
                        item: WidgetItem::Property {
                            importance: Importance::WIDGET,
                            args,
                            captured: false,
                        },
                    });
                    inputs.push(Input {
                        input,
                        item_idx: self.p.items.len() - 1,
                    });
                } else {
                    inputs.truncate(valid_inputs);
                    self.p.items.truncate(valid_items);
                    continue 'when;
                }
            }

            let mut any_assign = false;
            // collect assigns.
            'assign: for assign in when.assigns.iter() {
                let id = assign.id();
                let assign_info;
                let i;
                if let Some(idx) = self.property_index(id) {
                    assign_info = assign.property();
                    i = idx;
                } else if let Some(default) = assign.property().default {
                    let args = default(assign.instance());
                    assign_info = args.property();
                    i = self.p.items.len();
                    self.p.items.push(WidgetItemPositioned {
                        position: NestPosition::property(args.property().group),
                        insert_idx: u32::MAX,
                        item: WidgetItem::Property {
                            importance: Importance::WIDGET,
                            args,
                            captured: false,
                        },
                    });
                } else {
                    continue;
                }

                any_assign = true;

                let default_args = match &self.items[i].item {
                    WidgetItem::Property { args, .. } => args,
                    WidgetItem::Intrinsic { .. } => unreachable!(),
                };
                let info = default_args.property();

                for (default_info, assign_info) in info.inputs.iter().zip(assign_info.inputs.iter()) {
                    if default_info.ty != assign_info.ty {
                        // can happen with generic properties.
                        continue 'assign;
                    }
                }

                let entry = match assigns.entry(id) {
                    linear_map::Entry::Occupied(e) => e.into_mut(),
                    linear_map::Entry::Vacant(e) => e.insert(Assign {
                        item_idx: i,
                        builder: info
                            .inputs
                            .iter()
                            .enumerate()
                            .map(|(i, input)| match input.kind {
                                InputKind::Var => Box::new(AnyWhenVarBuilder::new_any(default_args.var(i).clone_any())) as _,
                                InputKind::UiNode => Box::new(WhenUiNodeBuilder::new(default_args.ui_node(i).take_on_init())) as _,
                                InputKind::UiNodeList => {
                                    Box::new(WhenUiNodeListBuilder::new(default_args.ui_node_list(i).take_on_init())) as _
                                }
                                InputKind::WidgetHandler => {
                                    Box::new(AnyWhenArcWidgetHandlerBuilder::new(default_args.widget_handler(i).clone_boxed())) as _
                                }
                                InputKind::StateVar | InputKind::Value => panic!("can only assign vars in when blocks"),
                            })
                            .collect(),
                        when_count: 0,
                        actions_data: Default::default(),
                    }),
                };
                entry.when_count += 1;

                for (i, (input, entry)) in info.inputs.iter().zip(entry.builder.iter_mut()).enumerate() {
                    match input.kind {
                        InputKind::Var => {
                            let entry = entry.downcast_mut::<AnyWhenVarBuilder>().unwrap();
                            let value = assign.var(i).clone_any();
                            entry.push_any(when.state.clone(), value);
                        }
                        InputKind::UiNode => {
                            let entry = entry.downcast_mut::<WhenUiNodeBuilder>().unwrap();
                            let node = assign.ui_node(i).take_on_init();
                            entry.push(when.state.clone(), node);
                        }
                        InputKind::UiNodeList => {
                            let entry = entry.downcast_mut::<WhenUiNodeListBuilder>().unwrap();
                            let list = assign.ui_node_list(i).take_on_init();
                            entry.push(when.state.clone(), list);
                        }
                        InputKind::WidgetHandler => {
                            let entry = entry.downcast_mut::<AnyWhenArcWidgetHandlerBuilder>().unwrap();
                            let handler = assign.widget_handler(i).clone_boxed();
                            entry.push(when.state.clone(), handler);
                        }
                        InputKind::StateVar | InputKind::Value => panic!("cannot assign `StateVar`, `Value` in when blocks"),
                    }
                }

                for ((property_id, action_key), action) in &when.build_action_data {
                    if *property_id == id {
                        match entry.actions_data.entry(*action_key) {
                            linear_map::Entry::Occupied(mut e) => {
                                let e = e.get_mut();
                                for _ in e.0.len()..(entry.when_count - 1) {
                                    e.0.push(None);
                                }
                                e.0.push(Some(action.data.clone()));
                                if action.default_action.is_some() && e.1.is_none() {
                                    e.1 = action.default_action.clone();
                                }
                            }
                            linear_map::Entry::Vacant(e) => {
                                let mut a = Vec::with_capacity(entry.when_count);
                                for _ in 0..(entry.when_count - 1) {
                                    a.push(None);
                                }
                                a.push(Some(action.data.clone()));
                                e.insert((a, action.default_action.clone()));
                            }
                        }
                    }
                }
            }

            if !any_assign {
                inputs.truncate(valid_inputs);
                self.p.items.truncate(valid_items);
            }
        }

        for Input { input, item_idx } in inputs {
            let args = match &self.items[item_idx].item {
                WidgetItem::Property { args, .. } => args,
                WidgetItem::Intrinsic { .. } => unreachable!(),
            };
            let info = args.property();

            let member_i = match input.member {
                WhenInputMember::Named(name) => info.input_idx(name).expect("when ref named input not found"),
                WhenInputMember::Index(i) => i,
            };

            let actual = match info.inputs[member_i].kind {
                InputKind::Var => args.var(member_i).clone_any(),
                InputKind::StateVar => args.state_var(member_i).clone_any(),
                InputKind::Value => args.value(member_i).clone_boxed_var(),
                _ => panic!("can only ref var, state-var or values in when expr"),
            };
            input.var.set(actual);
        }

        for (
            _,
            Assign {
                item_idx,
                builder,
                when_count,
                mut actions_data,
            },
        ) in assigns
        {
            let args = match &mut self.items[item_idx].item {
                WidgetItem::Property { args, .. } => args,
                WidgetItem::Intrinsic { .. } => unreachable!(),
            };

            let mut actions = vec![];
            let mut b_actions_data = vec![];
            if !build_actions.is_empty() {
                let p_id = args.id();
                while let Some(i) = build_actions.iter().position(|((id, _), _)| *id == p_id) {
                    let ((_, action_key), (_, a)) = build_actions.swap_remove(i);
                    actions.push(a);

                    if let Some(data) = actions_data.remove(action_key) {
                        let mut data = data.clone();
                        for _ in data.0.len()..when_count {
                            data.0.push(None);
                        }
                        b_actions_data.push(data.0);
                    }
                }
            }

            for (_, (mut data, default)) in actions_data {
                if let Some(default) = default {
                    let action = default();
                    for _ in data.len()..when_count {
                        data.push(None);
                    }

                    actions.push(action);
                    b_actions_data.push(data);
                }
            }

            *args = (args.property().new)(PropertyNewArgs {
                inst_info: args.instance(),
                args: builder,
                build_actions: actions,
                build_actions_when_data: b_actions_data,
            });
        }
    }

    fn build_p_actions(&mut self, mut build_actions: PropertyBuildActionsVec) {
        while !build_actions.is_empty() {
            let ((p_id, _), (_, a)) = build_actions.swap_remove(0);
            let mut actions = vec![a];

            while let Some(i) = build_actions.iter().position(|((id, _), _)| *id == p_id) {
                let (_, (_, a)) = build_actions.swap_remove(i);
                actions.push(a);
            }

            if let Some(i) = self.property_index(p_id) {
                match &mut self.items[i].item {
                    WidgetItem::Property { args, .. } => *args = args.new_build(actions, vec![]),
                    WidgetItem::Intrinsic { .. } => unreachable!(),
                }
            }
        }
    }

    fn build(mut self) -> BoxedUiNode {
        // sort by group, index and insert index.
        self.items.sort_unstable_by_key(|b| b.sort_key());

        #[cfg(inspector)]
        let mut inspector_items = Vec::with_capacity(self.p.items.len());

        let mut node = self.child.take().unwrap_or_else(|| NilUiNode.boxed());
        for WidgetItemPositioned { position, item, .. } in self.p.items.into_iter().rev() {
            match item {
                WidgetItem::Property { args, captured, .. } => {
                    if !captured {
                        node = args.instantiate(node);

                        #[cfg(trace_wgt_item)]
                        if self.trace_wgt_item {
                            let name = args.instance().name;
                            node = node.trace(|_, mtd| crate::context::UpdatesTrace::property_span(name, mtd));
                        }
                    }

                    #[cfg(inspector)]
                    inspector_items.push(crate::inspector::InstanceItem::Property { args, captured });
                }
                #[allow(unused_variables)]
                WidgetItem::Intrinsic { new, name } => {
                    node = new(node);
                    #[cfg(trace_wgt_item)]
                    if self.trace_wgt_item {
                        node = node.trace(|_, mtd| ctate::context::UpdatesTrace::intrinsic_span(name, mtd));
                    }

                    #[cfg(inspector)]
                    inspector_items.push(crate::inspector::InstanceItem::Intrinsic {
                        group: position.group,
                        name,
                    });

                    #[cfg(not(inspector))]
                    let _ = position;
                }
            }
        }

        #[cfg(inspector)]
        if let Some(builder) = self.builder {
            node = crate::inspector::insert_widget_builder_info(
                node,
                crate::inspector::InspectorInfo {
                    builder,
                    items: inspector_items.into_boxed_slice(),
                },
            )
            .boxed();
        }

        #[cfg(trace_widget)]
        if self.trace_widget {
            let name = self.widget_mod.name();
            node = node
                .trace(move |ctx, mtd| crate::context::UpdatesTrace::widget_span(ctx.path.widget_id(), name, mtd))
                .boxed();
        }

        // ensure `when` reuse works, by forcing input refresh on (re)init.
        node = types::with_new_context_init_id(node).boxed();

        node
    }
}
impl ops::Deref for WidgetBuilding {
    type Target = WidgetBuilderProperties;

    fn deref(&self) -> &Self::Target {
        &self.p
    }
}
impl ops::DerefMut for WidgetBuilding {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.p
    }
}

/// Represents a property removed from [`WidgetBuilding`].
#[derive(Debug)]
pub struct BuilderProperty {
    /// Property importance at the time of remove.
    pub importance: Importance,
    /// Property group and index at the time of remove.
    pub position: NestPosition,
    /// Property args.
    pub args: Box<dyn PropertyArgs>,
    /// If the property was *captured* before remove.
    pub captured: bool,
}

/// Represents a property in [`WidgetBuilder`] or [`WidgetBuilding`].
#[derive(Debug)]
pub struct BuilderPropertyRef<'a> {
    /// Property current importance.
    pub importance: Importance,
    /// Property current group and index.
    pub position: NestPosition,
    /// Property args.
    pub args: &'a dyn PropertyArgs,
    /// If the property was *captured*.
    ///
    /// This can only be `true` in [`WidgetBuilding`].
    pub captured: bool,
}

/// Represents a mutable reference to property in [`WidgetBuilder`] or [`WidgetBuilding`].
#[derive(Debug)]
pub struct BuilderPropertyMut<'a> {
    /// Property current importance.
    pub importance: &'a mut Importance,
    /// Property current group and index.
    pub position: &'a mut NestPosition,
    /// Property args.
    pub args: &'a mut Box<dyn PropertyArgs>,
    /// If the property was *captured*.
    ///
    /// This can only be `true` in [`WidgetBuilding`].
    pub captured: &'a mut bool,
}

/// Direct property access in [`WidgetBuilder`] and [`WidgetBuilding`].
pub struct WidgetBuilderProperties {
    items: Vec<WidgetItemPositioned>,
}
impl WidgetBuilderProperties {
    /// Reference the property, if it is present.
    pub fn property(&self, property_id: PropertyId) -> Option<BuilderPropertyRef> {
        match self.property_index(property_id) {
            Some(i) => match &self.items[i].item {
                WidgetItem::Property {
                    importance,
                    args,
                    captured,
                } => Some(BuilderPropertyRef {
                    importance: *importance,
                    position: self.items[i].position,
                    args: &**args,
                    captured: *captured,
                }),
                WidgetItem::Intrinsic { .. } => unreachable!(),
            },
            None => None,
        }
    }

    /// Modify the property, if it is present.
    pub fn property_mut(&mut self, property_id: PropertyId) -> Option<BuilderPropertyMut> {
        match self.property_index(property_id) {
            Some(i) => match &mut self.items[i] {
                WidgetItemPositioned {
                    position,
                    item:
                        WidgetItem::Property {
                            importance,
                            args,
                            captured,
                        },
                    ..
                } => Some(BuilderPropertyMut {
                    importance,
                    position,
                    args,
                    captured,
                }),
                _ => unreachable!(),
            },
            None => None,
        }
    }

    /// Iterate over the current properties.
    ///
    /// The properties may not be sorted in the correct order if the builder has never built.
    pub fn properties(&self) -> impl Iterator<Item = BuilderPropertyRef> {
        self.items.iter().filter_map(|it| match &it.item {
            WidgetItem::Intrinsic { .. } => None,
            WidgetItem::Property {
                importance,
                args,
                captured,
            } => Some(BuilderPropertyRef {
                importance: *importance,
                position: it.position,
                args: &**args,
                captured: *captured,
            }),
        })
    }

    /// iterate over mutable references to the current properties.
    pub fn properties_mut(&mut self) -> impl Iterator<Item = BuilderPropertyMut> {
        self.items.iter_mut().filter_map(|it| match &mut it.item {
            WidgetItem::Intrinsic { .. } => None,
            WidgetItem::Property {
                importance,
                args,
                captured,
            } => Some(BuilderPropertyMut {
                importance,
                position: &mut it.position,
                args,
                captured,
            }),
        })
    }

    /// Flags the property as captured and downcast the input value.
    ///
    /// Unlike other property kinds you can capture values in the [`WidgetBuilder`], note that the value may not
    /// the final value, unless you are capturing on build. Other properties kinds can only be captured in [`WidgetBuilding`] as
    /// their values strongly depend on the final `when` blocks that are only applied after building starts.
    pub fn capture_value<T>(&mut self, property_id: PropertyId) -> Option<T>
    where
        T: VarValue,
    {
        let p = self.capture_property_impl(property_id)?;
        let value = p.args.downcast_value::<T>(0).clone();
        Some(value)
    }

    /// Flags the property as captured and downcast the input value, or calls `or_else` to generate the value.
    pub fn capture_value_or_else<T>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> T) -> T
    where
        T: VarValue,
    {
        match self.capture_value(property_id) {
            Some(v) => v,
            None => or_else(),
        }
    }

    /// Flags the property as captured and downcast the input value, or returns the default value.
    pub fn capture_value_or_default<T>(&mut self, property_id: PropertyId) -> T
    where
        T: VarValue + Default,
    {
        self.capture_value_or_else(property_id, T::default)
    }

    fn capture_property_impl(&mut self, property_id: PropertyId) -> Option<BuilderPropertyRef> {
        if let Some(i) = self.property_index(property_id) {
            match &mut self.items[i] {
                WidgetItemPositioned {
                    position,
                    item:
                        WidgetItem::Property {
                            importance,
                            args,
                            captured,
                        },
                    ..
                } => {
                    *captured = true;
                    Some(BuilderPropertyRef {
                        importance: *importance,
                        position: *position,
                        args: &**args,
                        captured: *captured,
                    })
                }
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    fn property_index(&self, property_id: PropertyId) -> Option<usize> {
        self.items.iter().position(|it| match &it.item {
            WidgetItem::Property { args, .. } => args.id() == property_id,
            WidgetItem::Intrinsic { .. } => false,
        })
    }
}

/// Represents any [`PropertyBuildAction<I>`].
pub trait AnyPropertyBuildAction: crate::private::Sealed + Any + Send + Sync {
    /// As any.
    fn as_any(&self) -> &dyn Any;

    /// Clone the action into a new box.
    fn clone_boxed(&self) -> Box<dyn AnyPropertyBuildAction>;
}

/// Arguments for [`PropertyBuildAction<I>`].
pub struct PropertyBuildActionArgs<'a, I: Any + Send> {
    /// The property input value.
    pub input: I,
    /// The [`WhenBuildAction::data`] for each when assign that affects `input` in the order that `input` was generated.
    ///
    /// Items are `None` for when assigns that do not have associated build action data.
    pub when_conditions_data: &'a [Option<WhenBuildActionData>],
}

/// Represents a custom build action targeting a property input that is applied after `when` is build.
///
/// The type `I` depends on the input kind:
///
/// The expected types for each [`InputKind`] are:
///
/// | Kind                | Expected Type
/// |---------------------|-------------------------------------------------
/// | [`Var`]             | `BoxedVar<T>`
/// | [`StateVar`]        | `StateVar`
/// | [`Value`]           | `T`
/// | [`UiNode`]          | `ArcNode<BoxedUiNode>`
/// | [`UiNodeList`]      | `ArcNodeList<BoxedUiNodeList>`
/// | [`WidgetHandler`]   | `ArcWidgetHandler<A>`
///
/// [`Var`]: InputKind::Var
/// [`StateVar`]: InputKind::StateVar
/// [`Value`]: InputKind::Value
/// [`UiNode`]: InputKind::UiNode
/// [`UiNodeList`]: InputKind::UiNodeList
/// [`WidgetHandler`]: InputKind::WidgetHandler
pub struct PropertyBuildAction<I: Any + Send>(Arc<Mutex<dyn FnMut(PropertyBuildActionArgs<I>) -> I + Send>>);
impl<I: Any + Send> crate::private::Sealed for PropertyBuildAction<I> {}
impl<I: Any + Send> Clone for PropertyBuildAction<I> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<I: Any + Send> AnyPropertyBuildAction for PropertyBuildAction<I> {
    fn clone_boxed(&self) -> Box<dyn AnyPropertyBuildAction> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl<I: Any + Send> PropertyBuildAction<I> {
    /// New build action.
    pub fn new(build: impl FnMut(PropertyBuildActionArgs<I>) -> I + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(build)))
    }

    /// New build action that just pass the input.
    pub fn no_op() -> Self {
        Self::new(|i| i.input)
    }

    /// Run the build action on a input.
    pub fn build(&self, args: PropertyBuildActionArgs<I>) -> I {
        (self.0.lock())(args)
    }
}
impl Clone for Box<dyn AnyPropertyBuildAction> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

/// Represents the strong types of each input of a property.
///
/// # Examples
///
/// The example uses [`property_input_types!`] to collect the types and compares it to a manually generated types. Note
/// that the type is a tuple even if there is only one input.
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, widget_builder::*, var::*};
/// # use std::any::Any;
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<bool>) -> impl UiNode {
/// #    child
/// }
///
/// assert_eq!(
///     property_input_types!(foo).type_id(),
///     PropertyInputTypes::<(BoxedVar<bool>,)>::unit().type_id(),
/// );
/// ```
///
/// You can use the collected types in advanced code generation, such as attribute proc-macros targeting property assigns in widgets.
/// The next example demonstrates a trait that uses auto-deref to convert a trait bound to a `bool`:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, widget_builder::*, var::*};
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<bool>) -> impl UiNode {
/// #    child
/// }
///
/// trait SingleBoolVar {
///     fn is_single_bool_var(self) -> bool;
/// }
///
/// // match
/// impl<'a, V: Var<bool>> SingleBoolVar for &'a PropertyInputTypes<(V,)> {
///     fn is_single_bool_var(self) -> bool {
///         true
///     }
/// }
///
/// // fallback impl
/// impl<T: Send + 'static> SingleBoolVar for PropertyInputTypes<T> {
///     fn is_single_bool_var(self) -> bool {
///         false
///     }
/// }
///
/// assert!((&property_input_types!(foo)).is_single_bool_var());
/// ```
///
/// Learn more about how this trick works and limitations
/// [here](https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md).
pub struct PropertyInputTypes<Tuple>(std::marker::PhantomData<Tuple>);
impl<Tuple> PropertyInputTypes<Tuple> {
    /// Unit value.
    pub const fn unit() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<Tuple> Clone for PropertyInputTypes<Tuple> {
    fn clone(&self) -> Self {
        Self::unit()
    }
}
impl<Tuple> Copy for PropertyInputTypes<Tuple> {}
// SAFETY: PhantomData
unsafe impl<Tuple> Send for PropertyInputTypes<Tuple> {}
unsafe impl<Tuple> Sync for PropertyInputTypes<Tuple> {}
