//! Widget and property builder types.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt, ops,
    rc::Rc,
};

use linear_map::LinearMap;

use crate::{
    handler::WidgetHandler,
    impl_from_and_into_var,
    text::{formatx, Text},
    var::*,
    widget_instance::{BoxedUiNode, BoxedUiNodeList, NilUiNode, RcNode, RcNodeList, UiNode, UiNodeList},
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
/// * `property::path`: Gets the ID for the standalone property function.
/// * `property::path as rename`: Gets the ID, but with the new name.
/// * `widget::path.property`: Gets the ID for the property re-exported by the widget.
/// * `widget::path.property as rename`: Gets the ID for widget property, but with the new name.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{property, widget_builder::property_id};
/// # pub mod path {
/// #   #[property(context)]
/// #   pub fn foo(child: impl UiNode, bar: impl IntoValue<bool>) -> impl UiNode {
/// #     child
/// #   }
/// # }
/// # fn main() {
/// let foo_id = property_id!(path::foo);
/// let renamed_id = property_id!(path::foo as bar);
///
/// assert_ne!(foo_id, renamed_id);
/// assert_eq!(foo_id.unique_id, renamed_id.unique_id);
/// assert_ne!(foo_id.name, renamed_id.name);
/// # }
/// ```
#[macro_export]
macro_rules! property_id {
    ($($property:ident)::+) => {{
        // Rust does not expand the macro if we remove the braces.
        #[rustfmt::skip] use $($property)::+::{property_id};
        property_id($crate::widget_builder::property_id_name(stringify!($($property)::+)))
    }};
    ($($property:ident)::+ as $rename:ident) => {{
        // Rust does not expand the macro if we remove the braces.
        #[rustfmt::skip] use $($property)::+::{property_id};
        property_id($crate::widget_builder::property_id_name(stringify!($rename)))
    }};
    ($($widget:ident)::+ . $property:ident) => {{
        #[rustfmt::skip] use $($widget)::+::{__properties__::{$property::{property_id}}};
        property_id($crate::widget_builder::property_id_name(stringify!($property)))
    }};
    ($($widget:ident)::+ . $property:ident as $rename:ident) => {{
        #[rustfmt::skip] use $($widget)::+::{__properties__::{$property::{property_id}}};
        property_id($crate::widget_builder::property_id_name(stringify!($rename)))
    }};
}
#[doc(inline)]
pub use crate::property_id;

#[doc(hidden)]
pub fn property_id_name(path: &'static str) -> &'static str {
    path.rsplit(':').last().unwrap_or("").trim()
}

///<span data-del-macro-root></span> New [`PropertyArgs`] box from a property and value.
///
/// # Syntax
///
/// The syntax is similar to a property assign in a widget, with some extra means to reference widget properties.
///
/// * `property::path = <value>;`: Args for the standalone property function.
/// * `property::path as rename = <value>;`: Args for the standalone property, but the ID is renamed.
/// * `widget::path.property = <value>;`: Args for a property re-exported by the widget.
/// * `widget::path.property as rename = <value>;`: Args for the widget property, but the ID renamed.
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
    ($($widget:ident)::+ . $property:ident $(as $rename:ident)? = $($value:tt)*) => {
        {
            $crate::widget_builder::property_args_getter! {
                $($widget)::+::__properties__::$property $(as $rename)? = $($value)*
            }
        }
    };
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
        match wgt.p.items.remove(0).1 {
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
    /// The priority, all items of the same priority are "grouped" together.
    pub priority: Priority,
    /// Extra sorting within items of the same priority.
    pub index: u16,
}
impl NestPosition {
    /// Default index used for intrinsic nodes, is `u16::MAX / 3`.
    pub const INTRINSIC_INDEX: u16 = u16::MAX / 3;

    /// Default index used for properties, is `INTRINSIC_INDEX * 2`.
    pub const PROPERTY_INDEX: u16 = Self::INTRINSIC_INDEX * 2;

    /// New position for property.
    pub fn property(priority: Priority) -> Self {
        NestPosition {
            priority,
            index: Self::PROPERTY_INDEX,
        }
    }

    /// New position for intrinsic node.
    pub fn intrinsic(priority: Priority) -> Self {
        NestPosition {
            priority,
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
            .field("priority", &self.priority)
            .field("index", &IndexName(self.index))
            .finish()
    }
}

/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    /// [Context](crate::property#context) property.
    Context,
    /// [Event](crate::property#event) property.
    Event,
    /// [Layout](crate::property#layout) property.
    Layout,
    /// [Size](crate::property#size) property.
    Size,
    /// [Border](crate::property#border) property.
    Border,
    /// [Fill](crate::property#fill) property.
    Fill,
    /// [Child Context](crate::property#child-context) property.
    ChildContext,
    /// [Child Layout](crate::property#child-layout) property.
    ChildLayout,
}
impl Priority {
    /// All priorities, from outermost([`Context`]) to innermost([`ChildLayout`]).
    ///
    /// [`Context`]: Priority::Context
    /// [`ChildLayout`]: Priority::ChildLayout
    pub const ITEMS: [Priority; 8] = [
        Priority::Context,
        Priority::Event,
        Priority::Layout,
        Priority::Size,
        Priority::Border,
        Priority::Fill,
        Priority::ChildContext,
        Priority::ChildLayout,
    ];

    /// Priority name in the style written in the `#[property(..)]` attribute.
    pub fn name(self) -> &'static str {
        match self {
            Priority::Context => "context",
            Priority::Event => "event",
            Priority::Layout => "layout",
            Priority::Size => "size",
            Priority::Border => "border",
            Priority::Fill => "fill",
            Priority::ChildContext => "child_context",
            Priority::ChildLayout => "child_layout",
        }
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
    /// Input is `impl UiNode`, build value is `RcNode<BoxedUiNode>`.
    UiNode,
    /// Input is `impl UiNodeList`, build value is `RcNodeList<BoxedUiNodeList>`.
    UiNodeList,
    /// Input is `impl WidgetHandler<A>`, build value is `RcWidgetHandler<A>`.
    WidgetHandler,
}

/// Represents a [`WidgetHandler<A>`] that can be reused.
///
/// Note that [`once_hn!`] will still only be used once, and [`async_hn!`] tasks are bound to the specific widget
/// context that spawned then. This `struct` is cloneable to support handler properties in styleable widgets, but the
/// general expectation is that the handler will be used on one property instance at a time.
#[derive(Clone)]
pub struct RcWidgetHandler<A: Clone + 'static>(Rc<RefCell<dyn WidgetHandler<A>>>);
impl<A: Clone + 'static> RcWidgetHandler<A> {
    /// New from `handler`.
    pub fn new(handler: impl WidgetHandler<A>) -> Self {
        Self(Rc::new(RefCell::new(handler)))
    }
}
impl<A: Clone + 'static> WidgetHandler<A> for RcWidgetHandler<A> {
    fn event(&mut self, ctx: &mut crate::context::WidgetContext, args: &A) -> bool {
        self.0.borrow_mut().event(ctx, args)
    }

    fn update(&mut self, ctx: &mut crate::context::WidgetContext) -> bool {
        self.0.borrow_mut().update(ctx)
    }
}
/// Property info.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property insert order.
    pub priority: Priority,
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

    /// Function that constructs the default args for the property.
    pub default: Option<fn(PropertyInstInfo) -> Box<dyn PropertyArgs>>,

    /// Property inputs info, always at least one.
    pub inputs: Box<[PropertyInput]>,
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

/// Represents a property instantiation request.
pub trait PropertyArgs {
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
    fn ui_node(&self, i: usize) -> &RcNode<BoxedUiNode> {
        panic_input(&self.property(), i, InputKind::UiNode)
    }

    /// Gets a [`InputKind::UiNodeList`].
    fn ui_node_list(&self, i: usize) -> &RcNodeList<BoxedUiNodeList> {
        panic_input(&self.property(), i, InputKind::UiNodeList)
    }

    /// Gets a [`InputKind::WidgetHandler`].
    ///
    /// Is a `RcWidgetHandler<A>`.
    fn widget_handler(&self, i: usize) -> &dyn Any {
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
    pub fn downcast_handler<A>(&self, i: usize) -> &RcWidgetHandler<A>
    where
        A: 'static + Clone,
    {
        self.widget_handler(i)
            .downcast_ref::<RcWidgetHandler<A>>()
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
            InputKind::WidgetHandler => LocalVar(formatx!("<impl WidgetHandler<{}>>", p.inputs[i].ty_name)).boxed(),
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
            InputKind::WidgetHandler => formatx!("<impl WidgetHandler<{}>>", p.inputs[i].ty_name),
        }
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
pub fn ui_node_to_args(node: impl UiNode) -> RcNode<BoxedUiNode> {
    RcNode::new(node.boxed())
}

#[doc(hidden)]
pub fn ui_node_list_to_args(node_list: impl UiNodeList) -> RcNodeList<BoxedUiNodeList> {
    RcNodeList::new(node_list.boxed())
}

#[doc(hidden)]
pub fn widget_handler_to_args<A: Clone + 'static>(handler: impl WidgetHandler<A>) -> RcWidgetHandler<A> {
    RcWidgetHandler::new(handler)
}

/*

 WIDGET

*/

/// Value that indicates the override importance of a property instance, higher overrides lower.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Importance(pub usize);
impl Importance {
    /// Importance of default values defined in the widget declaration.
    pub const WIDGET: Importance = Importance(1000);
    /// Importance of values defined in the widget instantiation.
    pub const INSTANCE: Importance = Importance(1000 * 10);
}
impl_from_and_into_var! {
    fn from(imp: usize) -> Importance {
        Importance(imp)
    }
}

unique_id_32! {
    /// Unique ID of a widget implementation.
    ///
    /// This ID identifies a widget implementation. Widgets are identified
    /// by this ID and a module path, see [`WidgetMod`].
    ///
    /// [`name`]: WidgetId::name
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
    /// by this ID and a name string, see [`PropertyType`].
    ///
    /// [`name`]: WidgetId::name
    pub struct PropertyImplId;
}
impl fmt::Debug for PropertyImplId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PropertyImplId").field(&self.get()).finish()
    }
}

/// Unique identifier of a property, properties with the same id override each other in a widget and are joined
/// into a single instance is assigned in when blocks.
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
    Some { var: RcVar<T>, handle: VarHandle },
}
impl<T: VarValue> WhenInputVarActual<T> {
    fn bind_init(&mut self, vars: &Vars, other: &impl Var<T>) {
        match self {
            WhenInputVarActual::None => {
                let var = var(other.get());
                *self = Self::Some {
                    handle: other.bind(&var),
                    var,
                }
            }
            WhenInputVarActual::Some { var, handle } => {
                var.set(vars, other.get());
                *handle = other.bind(var);
            }
        }
    }

    fn bind_init_value(&mut self, vars: &Vars, value: T) {
        match self {
            WhenInputVarActual::None => {
                *self = Self::Some {
                    var: var(value),
                    handle: VarHandle::dummy(),
                }
            }
            WhenInputVarActual::Some { var, handle } => {
                *handle = VarHandle::dummy();
                var.set(vars, value);
            }
        }
    }
}
trait AnyWhenInputVarActual: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn is_some(&self) -> bool;
}
impl<T: VarValue> AnyWhenInputVarActual for WhenInputVarActual<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn is_some(&self) -> bool {
        matches!(self, Self::Some { .. })
    }
}

/// Represents a [`WhenInput`] variable that can be rebound.
#[derive(Clone)]
pub struct WhenInputVar {
    var: Rc<RefCell<dyn AnyWhenInputVarActual>>,
}
impl WhenInputVar {
    /// New for property without default value.
    pub fn new<T: VarValue>() -> (Self, impl Var<T>) {
        let rc: Rc<RefCell<dyn AnyWhenInputVarActual>> = Rc::new(RefCell::new(WhenInputVarActual::<T>::None));
        (
            WhenInputVar { var: rc.clone() },
            crate::var::types::ContextualizedVar::new(Rc::new(move || {
                match rc.borrow().as_any().downcast_ref::<WhenInputVarActual<T>>().unwrap() {
                    WhenInputVarActual::Some { var, .. } => var.read_only(),
                    WhenInputVarActual::None => panic!("when var input not inited"),
                }
            })),
        )
    }

    /// Returns `true` if a default or bound value has inited the variable and it is of type `T`.
    ///
    /// Note that attempting to use the [`WhenInfo::state`] when this is `false` will cause a panic.
    pub fn can_use(&self) -> bool {
        self.var.borrow().is_some()
    }

    /// Assign and bind the input var from `other`, after this call [`can_use`] is `true`.
    ///
    /// # Panics
    ///
    /// If `T` is not the same that was used to create the input var.
    pub fn bind<T: VarValue>(&self, vars: impl WithVars, other: &impl Var<T>) {
        vars.with_vars(|vars| self.validate_borrow_mut::<T>().bind_init(vars, other))
    }

    /// Assigns the input var to `value` and removes any previous binding, after this call [`can_use`] is `true`.
    ///
    /// # Panics
    ///
    /// If `T` is not the same that was used to create the input var.
    pub fn bind_value<T: VarValue>(&self, vars: impl WithVars, value: T) {
        vars.with_vars(|vars| self.validate_borrow_mut::<T>().bind_init_value(vars, value))
    }

    fn validate_borrow_mut<T: VarValue>(&self) -> std::cell::RefMut<WhenInputVarActual<T>> {
        std::cell::RefMut::map(self.var.borrow_mut(), |var| {
            match var.as_mut_any().downcast_mut::<WhenInputVarActual<T>>() {
                Some(a) => a,
                None => panic!("incorrect when input var type"),
            }
        })
    }
}

/// Represents a `when` block in a widget.
#[derive(Clone)]
pub struct WhenInfo {
    /// Properties referenced in the when condition expression.
    ///
    /// They are type erased `RcVar<T>` instances and can be rebound, other variable references (`*#{var}`) are imbedded in
    /// the build expression and cannot be modified.
    pub inputs: Box<[WhenInput]>,

    /// Output of the when expression.
    ///
    /// # Panics
    ///
    /// If used when [`can_use`] is `false`.
    pub state: BoxedVar<bool>,

    /// Properties assigned in the when block, in the build widget they are joined with the default value and assigns
    /// from other when blocks into a single property instance set to `when_var!` inputs.
    pub assigns: Box<[Box<dyn PropertyArgs>]>,

    /// The condition expression code.
    pub expr: &'static str,
}
impl WhenInfo {
    /// Returns `true` if the [`state`] var is valid because it does not depend of any property input or all
    /// property inputs are inited with a value or have a default.
    pub fn can_use(&self) -> bool {
        self.inputs.iter().all(|i| i.var.can_use())
    }
}
impl fmt::Debug for WhenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhenInfo")
            .field("inputs", &self.inputs)
            .field("state", &self.state.debug())
            .field("assigns", &self.assigns)
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

enum WidgetItem {
    Property {
        importance: Importance,
        args: Box<dyn PropertyArgs>,
    },
    Intrinsic {
        new: Box<dyn FnOnce(BoxedUiNode) -> BoxedUiNode>,
    },
}
impl Clone for WidgetItem {
    fn clone(&self) -> Self {
        match self {
            Self::Property { importance, args } => Self::Property {
                importance: *importance,
                args: args.clone(),
            },
            Self::Intrinsic { .. } => unreachable!("only WidgetBuilder clones, and it does not insert intrinsic"),
        }
    }
}

/// Widget instance builder.
pub struct WidgetBuilder {
    widget_mod: WidgetMod,
    p: WidgetBuilderProperties,
    unset: LinearMap<PropertyId, Importance>,
    whens: Vec<(Importance, WhenInfo)>,
    build_actions: Vec<Rc<RefCell<dyn FnMut(&mut WidgetBuilding)>>>,
    custom_build: Option<Rc<RefCell<dyn FnMut(WidgetBuilder) -> BoxedUiNode>>>,
}
impl Clone for WidgetBuilder {
    fn clone(&self) -> Self {
        Self {
            widget_mod: self.widget_mod,
            p: WidgetBuilderProperties { items: self.items.clone() },
            unset: self.unset.clone(),
            whens: self.whens.clone(),
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
            unset: Default::default(),
            whens: Default::default(),
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
        let pos = NestPosition::property(args.property().priority);
        self.push_property_positioned(importance, args, pos);
    }

    /// Insert property with custom nest position.
    pub fn push_property_positioned(&mut self, importance: Importance, args: Box<dyn PropertyArgs>, position: NestPosition) {
        let property_id = args.id();
        if let Some(i) = self.p.property_index(property_id) {
            match &self.p.items[i].1 {
                WidgetItem::Property { importance: imp, .. } => {
                    if *imp <= importance {
                        // override
                        self.p.items[i] = (position, WidgetItem::Property { importance, args });
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
            self.p.items.push((position, WidgetItem::Property { importance, args }));
        }
    }

    /// Insert a `when` block.
    pub fn push_when(&mut self, importance: Importance, when: WhenInfo) {
        self.whens.push((importance, when));
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
                match &self.p.items[i].1 {
                    WidgetItem::Property { importance: imp, .. } => {
                        if *imp <= importance {
                            self.p.items.remove(i);
                        }
                    }
                    WidgetItem::Intrinsic { .. } => unreachable!(),
                }
            }
        }
    }

    /// Add an `action` closure that is called every time this builder or a clone of it builds a widget instance.
    pub fn push_build_action(&mut self, action: impl FnMut(&mut WidgetBuilding) + 'static) {
        self.build_actions.push(Rc::new(RefCell::new(action)))
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
    pub fn set_custom_build(&mut self, build: impl FnMut(WidgetBuilder) -> BoxedUiNode + 'static) {
        self.custom_build = Some(Rc::new(RefCell::new(build)));
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

        for (pos, p) in other.p.items {
            match p {
                WidgetItem::Property { importance, args } => {
                    self.push_property_positioned(importance, args, pos);
                }
                WidgetItem::Intrinsic { .. } => unreachable!(),
            }
        }

        for (imp, when) in other.whens {
            self.push_when(imp, when);
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

    /// Instantiate the widget.
    ///
    /// If a custom build is set it is run, unless it is already running, otherwise the [`default_build`] is called.
    ///
    /// [`default_build`]: Self::default_build
    pub fn build(self) -> BoxedUiNode {
        if let Some(cust) = self.custom_build.clone() {
            match cust.try_borrow_mut() {
                Ok(mut c) => c(self),
                Err(_) => self.default_build(),
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
            p: self.p,
            whens: self.whens,
            child: None,
        };

        #[cfg(inspector)]
        building.push_intrinsic(Priority::Context, move |child| {
            crate::inspector::insert_widget_builder_info(child, builder)
        });

        for action in self.build_actions {
            (action.borrow_mut())(&mut building);
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
/// Widgets can register a [`build_action`] to get access to this, it provides an opportunity
/// to remove or capture the final properties of an widget, after they have all been resolved as well as define
/// the child node, intrinsic nodes and a custom builder.
pub struct WidgetBuilding {
    p: WidgetBuilderProperties,
    whens: Vec<(Importance, WhenInfo)>,
    child: Option<BoxedUiNode>,
}
impl WidgetBuilding {
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

    /// Insert intrinsic node, that is a core functionality node of the widget that cannot be overridden.
    pub fn push_intrinsic<I: UiNode>(&mut self, priority: Priority, intrinsic: impl FnOnce(BoxedUiNode) -> I + 'static) {
        self.push_intrinsic_positioned(intrinsic, NestPosition::intrinsic(priority))
    }

    /// Insert intrinsic node with custom nest position.
    pub fn push_intrinsic_positioned<I: UiNode>(&mut self, intrinsic: impl FnOnce(BoxedUiNode) -> I + 'static, position: NestPosition) {
        self.items.push((
            position,
            WidgetItem::Intrinsic {
                new: Box::new(move |n| intrinsic(n).boxed()),
            },
        ));
    }

    fn build(mut self) -> BoxedUiNode {
        self.items.sort_by_key(|(k, _)| *k);

        // TODO !!: when

        let mut node = self.child.take().unwrap_or_else(|| NilUiNode.boxed());
        for (_, item) in self.p.items {
            match item {
                WidgetItem::Property { args, .. } => {
                    node = args.instantiate(node);
                }
                WidgetItem::Intrinsic { new } => {
                    node = new(node);
                }
            }
        }

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

/// Direct property access in [`WidgetBuilder`] and [`WidgetBuilding`].
pub struct WidgetBuilderProperties {
    items: Vec<(NestPosition, WidgetItem)>,
}
impl WidgetBuilderProperties {
    /// Remove the property that matches the `property_id!(..)`.
    pub fn remove_property(&mut self, property_id: PropertyId) -> Option<(Importance, NestPosition, Box<dyn PropertyArgs>)> {
        if let Some(i) = self.property_index(property_id) {
            match self.items.remove(i) {
                // can't be swap remove for ordering of equal priority.
                (pos, WidgetItem::Property { importance, args, .. }) => Some((importance, pos, args)),
                (_, WidgetItem::Intrinsic { .. }) => unreachable!(),
            }
        } else {
            None
        }

        // this method is used to remove "captures", that means we need to remove `when` assigns and a clone of the conditions too?
    }

    /// Remove the property and downcast the input value.
    pub fn capture_value<T>(&mut self, property_id: PropertyId) -> Option<T>
    where
        T: VarValue,
    {
        let (_, _, args) = self.remove_property(property_id)?;
        let value = args.downcast_value::<T>(0).clone();
        Some(value)
    }

    /// Remove the property and downcast the input value, or calls `or_else` to generate the value.
    pub fn capture_value_or_else<T>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> T) -> T
    where
        T: VarValue,
    {
        match self.capture_value(property_id) {
            Some(v) => v,
            None => or_else(),
        }
    }

    /// Remove the property and downcast the input value, or returns the default value.
    pub fn capture_value_or_default<T>(&mut self, property_id: PropertyId) -> T
    where
        T: VarValue + Default,
    {
        self.capture_value_or_else(property_id, T::default)
    }

    /// Remove the property and downcast the input var.
    pub fn capture_var<T>(&mut self, property_id: PropertyId) -> Option<BoxedVar<T>>
    where
        T: VarValue,
    {
        let (_, _, args) = self.remove_property(property_id)?;
        let var = args.downcast_var::<T>(0).clone();
        Some(var)
    }

    /// Remove the property and downcast the input var, or calls `or_else` to generate a fallback.
    pub fn capture_var_or_else<T>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> T) -> BoxedVar<T>
    where
        T: VarValue,
    {
        match self.capture_var::<T>(property_id) {
            Some(var) => var,
            None => or_else().into_var().boxed(),
        }
    }

    /// Remove the property and downcast the input var, returns a new one with the default value.
    pub fn capture_var_or_default<T>(&mut self, property_id: PropertyId) -> BoxedVar<T>
    where
        T: VarValue + Default,
    {
        self.capture_var_or_else(property_id, T::default)
    }

    /// Remove the property and get the input node.
    pub fn capture_ui_node(&mut self, property_id: PropertyId) -> Option<BoxedUiNode> {
        let (_, _, args) = self.remove_property(property_id)?;
        let node = args.ui_node(0).take_on_init().boxed();
        Some(node)
    }

    /// Remove the property and get the input node, or calls `or_else` to generate a fallback node.
    pub fn capture_ui_node_or_else<F>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> F) -> BoxedUiNode
    where
        F: UiNode,
    {
        match self.capture_ui_node(property_id) {
            Some(u) => u,
            None => or_else().boxed(),
        }
    }

    /// Remove the property and get the input list.
    pub fn capture_ui_node_list(&mut self, property_id: PropertyId) -> Option<BoxedUiNodeList> {
        let (_, _, args) = self.remove_property(property_id)?;
        let list = args.ui_node_list(0).take_on_init().boxed();
        Some(list)
    }

    /// Remove the property and get the input list, or calls `or_else` to generate a fallback list.
    pub fn capture_ui_node_list_or_else<F>(&mut self, property_id: PropertyId, or_else: impl FnOnce() -> F) -> BoxedUiNodeList
    where
        F: UiNodeList,
    {
        match self.capture_ui_node_list(property_id) {
            Some(u) => u,
            None => or_else().boxed(),
        }
    }

    /// Remove the property and get the input list, or returns an empty list.
    pub fn capture_ui_node_list_or_empty(&mut self, property_id: PropertyId) -> BoxedUiNodeList {
        self.capture_ui_node_list_or_else(property_id, Vec::<BoxedUiNode>::new)
    }

    /// Remove the property and downcast the input handler.
    pub fn capture_widget_handler<A: Clone + 'static>(&mut self, property_id: PropertyId) -> Option<RcWidgetHandler<A>> {
        let (_, _, args) = self.remove_property(property_id)?;
        let handler = args.downcast_handler::<A>(0).clone();
        Some(handler)
    }

    /// Reference the property, if it is present.
    pub fn property(&self, property_id: PropertyId) -> Option<(Importance, NestPosition, &dyn PropertyArgs)> {
        match self.property_index(property_id) {
            Some(i) => match &self.items[i].1 {
                WidgetItem::Property { importance, args } => Some((*importance, self.items[i].0, &**args)),
                WidgetItem::Intrinsic { .. } => unreachable!(),
            },
            None => None,
        }
    }

    /// Modify the property, if it is present.
    pub fn property_mut(&mut self, property_id: PropertyId) -> Option<(&mut Importance, &mut NestPosition, &mut Box<dyn PropertyArgs>)> {
        match self.property_index(property_id) {
            Some(i) => match &mut self.items[i] {
                (pos, WidgetItem::Property { importance, args }) => Some((importance, pos, args)),
                _ => unreachable!(),
            },
            None => None,
        }
    }

    /// Iterate over the current properties.
    ///
    /// The properties may not be sorted in the correct order if the builder has never built.
    pub fn properties(&self) -> impl Iterator<Item = (Importance, NestPosition, &dyn PropertyArgs)> {
        self.items.iter().filter_map(|(pos, it)| match it {
            WidgetItem::Intrinsic { .. } => None,
            WidgetItem::Property { importance, args } => Some((*importance, *pos, &**args)),
        })
    }

    /// iterate over mutable references to the current properties.
    pub fn properties_mut(&mut self) -> impl Iterator<Item = (&mut Importance, &mut NestPosition, &mut Box<dyn PropertyArgs>)> {
        self.items.iter_mut().filter_map(|(pos, it)| match it {
            WidgetItem::Intrinsic { .. } => None,
            WidgetItem::Property { importance, args } => Some((importance, pos, args)),
        })
    }

    fn property_index(&self, property_id: PropertyId) -> Option<usize> {
        self.items.iter().position(|(_, item)| match item {
            WidgetItem::Property { args, .. } => args.id() == property_id,
            WidgetItem::Intrinsic { .. } => false,
        })
    }
}
