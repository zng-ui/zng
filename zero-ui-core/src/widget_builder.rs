//! Widget and property builder types.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt, mem,
    rc::Rc,
};

use linear_map::LinearMap;

use crate::{
    handler::WidgetHandler,
    impl_from_and_into_var,
    var::*,
    widget_instance::{AdoptiveNode, BoxedUiNode, BoxedUiNodeList, NilUiNode, RcNode, RcNodeList, UiNode, UiNodeList},
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[macro_export]
macro_rules! property_id {
    ($property:path) => {{
        // Rust does not expand the macro if we remove the braces.
        #[rustfmt::skip] use $property::{property as p};

        p::__id__($crate::widget_builder::property_id_name(stringify!($property)))
    }};
    ($property:path as $rename:ident) => {{
        // Rust does not expand the macro if we remove the braces.
        #[rustfmt::skip] use $property::{property as p};

        p::__id__($crate::widget_builder::property_id_name(stringify!($rename)))
    }};
}
#[doc(inline)]
pub use crate::property_id;

#[doc(hidden)]
pub fn property_id_name(path: &'static str) -> &'static str {
    path.rsplit(':').last().unwrap_or("").trim()
}

///<span data-del-macro-root></span> New [`PropertyArgs`] box from a property and value.
#[macro_export]
macro_rules! property_args {
    ($property:path $(as $rename:ident)? = $($value:tt)*) => {
        {
            $crate::widget_builder::property_args_getter! {
                $property $(as $rename)? = $($value)*
            }
        }
    }
}
#[doc(inline)]
pub use crate::property_args;

#[doc(hidden)]
#[crate::widget($crate::widget_builder::property_args_getter)]
pub mod property_args_getter {
    use super::*;

    fn build(mut wgt: WidgetBuilder) -> Box<dyn PropertyArgs> {
        let id = wgt.properties().next().unwrap().2.id();
        wgt.remove_property(id).unwrap().2
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
    /// Input is `impl UiNode`, build value is `RcNode<BoxedWidget>`.
    Widget,
    /// Input is `impl UiNodeList`, build value is `RcNodeList<BoxedUiNodeList>`.
    UiNodeList,
    /// Input is `impl UiNodeList`, build value is `RcNodeList<BoxedWidgetList>`.
    WidgetList,
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

    /// Unique type ID that identifies the property.
    pub unique_id: TypeId,
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

    /// Unique ID.
    fn id(&self) -> PropertyId {
        PropertyId {
            unique_id: self.property().unique_id,
            name: self.instance().name,
        }
    }

    /// Gets a [`InputKind::Var`].
    ///
    /// Is a `BoxedVar<T>`.
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
    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode;
}

/// Extension methods for `Box<dyn PropertyArgs>`
pub trait PropertyArgsExt {
    /// Gets a strongly typed [`value`].
    ///
    /// [`value`]: PropertyArgs::value
    fn downcast_value<T: VarValue>(&self, i: usize) -> &T;
    /// Gets a strongly typed [`var`].
    ///
    /// [`var`]: PropertyArgs::var
    fn downcast_var<T: VarValue>(&self, i: usize) -> &BoxedVar<T>;

    /// Gets a strongly typed [`widget_handler`].
    ///
    /// [`widget_handler`]: PropertyArgs::widget_handler
    fn downcast_handler<A: Clone + 'static>(&self, i: usize) -> &RcWidgetHandler<A>;
}

impl PropertyArgsExt for Box<dyn PropertyArgs> {
    fn downcast_value<T: VarValue>(&self, i: usize) -> &T {
        self.value(i).as_any().downcast_ref::<T>().expect("cannot downcast value to type")
    }

    fn downcast_var<T: VarValue>(&self, i: usize) -> &BoxedVar<T> {
        self.var(i)
            .as_any()
            .downcast_ref::<BoxedVar<T>>()
            .expect("cannot downcast var to type")
    }

    fn downcast_handler<A: Clone + 'static>(&self, i: usize) -> &RcWidgetHandler<A> {
        self.widget_handler(i)
            .downcast_ref::<RcWidgetHandler<A>>()
            .expect("cannot downcast handler to type")
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

#[derive(Clone)]
enum WidgetItem {
    Instrinsic {
        child: Rc<RefCell<BoxedUiNode>>,
        node: RcNode<BoxedUiNode>,
    },
    Property {
        importance: Importance,
        args: Box<dyn PropertyArgs>,
    },
}

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

/// Unique identifier of a property, properties with the same id override each other in a widget and are joined
/// into a single instance is assigned in when blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId {
    /// The [`PropertyInfo::unique_id`].
    pub unique_id: TypeId,
    /// The [`PropertyInstInfo::name`].
    pub name: &'static str,
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

impl Clone for Box<dyn PropertyArgs> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

/// Widget instance builder.
#[derive(Default, Clone)]
pub struct WidgetBuilder {
    child: Option<RcNode<BoxedUiNode>>,
    items: Vec<(NestPosition, WidgetItem)>,
    unset: LinearMap<PropertyId, Importance>,
    whens: Vec<(Importance, WhenInfo)>,
    custom_build: Option<Box<dyn CustomWidgetBuild>>,

    items_sorted: bool,
    whens_sorted: bool,
}
impl WidgetBuilder {
    /// New empty default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert intrinsic node, that is a core functionality node of the widget that cannot be overridden.
    pub fn insert_intrinsic(&mut self, priority: Priority, node: AdoptiveNode<BoxedUiNode>) {
        self.insert_intrinsic_positioned(node, NestPosition::intrinsic(priority));
    }

    /// Insert/override a property.
    ///
    /// You can use the [`property_args!`] macro to collect args for a property.
    pub fn insert_property(&mut self, importance: Importance, args: Box<dyn PropertyArgs>) {
        let pos = NestPosition::property(args.property().priority);
        self.insert_property_positioned(importance, args, pos);
    }

    /// Insert intrinsic node with custom nest position.
    pub fn insert_intrinsic_positioned(&mut self, node: AdoptiveNode<BoxedUiNode>, position: NestPosition) {
        self.items_sorted = false;
        let (child, node) = node.into_parts();
        let node = RcNode::new(node);
        self.items.push((position, WidgetItem::Instrinsic { child, node }));
    }

    /// Insert property with custom nest position.
    pub fn insert_property_positioned(&mut self, importance: Importance, args: Box<dyn PropertyArgs>, position: NestPosition) {
        let property_id = args.id();
        if let Some(i) = self.property_index(property_id) {
            match &self.items[i].1 {
                WidgetItem::Property { importance: imp, .. } => {
                    if *imp <= importance {
                        // override

                        if self.items[i].0 != position {
                            self.items_sorted = false;
                        }

                        self.items[i] = (position, WidgetItem::Property { importance, args });
                    }
                }
                WidgetItem::Instrinsic { .. } => unreachable!(),
            }
        } else {
            if let Some(imp) = self.unset.get(&property_id) {
                if *imp >= importance {
                    return; // unset blocks.
                }
            }
            self.items.push((position, WidgetItem::Property { importance, args }));
            self.items_sorted = false;
        }
    }

    /// Insert a `when` block.
    pub fn insert_when(&mut self, importance: Importance, when: WhenInfo) {
        self.whens.push((importance, when));
        self.whens_sorted = false;
    }

    /// Insert a `name = unset!;` property.
    pub fn insert_unset(&mut self, importance: Importance, property_id: PropertyId) {
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
            self.items.retain(|(_, it)| match it {
                WidgetItem::Property { importance: imp, args } => args.id() != property_id || *imp > importance,
                WidgetItem::Instrinsic { .. } => true,
            });
        }
    }

    /// Set/replace the inner most node of the widget.
    pub fn set_child(&mut self, node: BoxedUiNode) -> Option<RcNode<BoxedUiNode>> {
        self.child.replace(RcNode::new(node))
    }

    /// Apply `other` over `self`.
    pub fn extend(&mut self, other: WidgetBuilder) {
        if let Some(child) = other.child {
            self.child = Some(child);
        }

        for (id, imp) in other.unset {
            self.insert_unset(imp, id);
        }

        for (position, item) in other.items {
            match item {
                WidgetItem::Instrinsic { child, node } => {
                    self.items.push((position, WidgetItem::Instrinsic { child, node }));
                }
                WidgetItem::Property { importance, args } => {
                    self.insert_property_positioned(importance, args, position);
                }
            }
        }

        for (imp, when) in other.whens {
            self.insert_when(imp, when);
        }
    }

    /// Remove the property that matches the `property_id!(..)`.
    pub fn remove_property(&mut self, property_id: PropertyId) -> Option<(Importance, NestPosition, Box<dyn PropertyArgs>)> {
        if let Some(i) = self.property_index(property_id) {
            match self.items.remove(i) {
                // can't be swap remove for ordering of equal priority.
                (pos, WidgetItem::Property { importance, args, .. }) => Some((importance, pos, args)),
                (_, WidgetItem::Instrinsic { .. }) => unreachable!(),
            }
        } else {
            None
        }

        // this method is used to remove "captures", that means we need to remove `when` assigns and a clone of the conditions too?
    }

    /// Remove the property and downcast the input value.
    pub fn capture_value<T: VarValue>(&mut self, property_id: PropertyId) -> Option<T> {
        let (_, _, args) = self.remove_property(property_id)?;
        let value = args.downcast_value::<T>(0).clone();
        Some(value)
    }

    /// Remove the property and downcast the input value.
    pub fn capture_var<T: VarValue>(&mut self, property_id: PropertyId) -> Option<BoxedVar<T>> {
        let (_, _, args) = self.remove_property(property_id)?;
        let var = args.downcast_var::<T>(0).clone();
        Some(var)
    }

    /// Remove the property and get the input n.
    pub fn capture_ui_node(&mut self, property_id: PropertyId) -> Option<BoxedUiNode> {
        let (_, _, args) = self.remove_property(property_id)?;
        let node = args.ui_node(0).take_on_init().boxed();
        Some(node)
    }

    /// Remove the property and get the input list.
    pub fn capture_ui_node_list(&mut self, property_id: PropertyId) -> Option<BoxedUiNodeList> {
        let (_, _, args) = self.remove_property(property_id)?;
        let list = args.ui_node_list(0).take_on_init().boxed();
        Some(list)
    }

    /// Remove the property and downcast the input handler.
    pub fn capture_widget_handler<A: Clone + 'static>(&mut self, property_id: PropertyId) -> Option<RcWidgetHandler<A>> {
        let (_, _, args) = self.remove_property(property_id)?;
        let handler = args.downcast_handler::<A>(0).clone();
        Some(handler)
    }

    /// Take the child node, leaving the builder without child.
    pub fn remove_child(&mut self) -> Option<RcNode<BoxedUiNode>> {
        self.child.take()
    }

    /// Reference the property, if it is present.
    pub fn property(&self, property_id: PropertyId) -> Option<(Importance, NestPosition, &dyn PropertyArgs)> {
        match self.property_index(property_id) {
            Some(i) => match &self.items[i].1 {
                WidgetItem::Property { importance, args } => Some((*importance, self.items[i].0, &**args)),
                WidgetItem::Instrinsic { .. } => unreachable!(),
            },
            None => None,
        }
    }

    /// Modify the property, if it is present.
    pub fn property_mut(&mut self, property_id: PropertyId) -> Option<(&mut Importance, &mut NestPosition, &mut Box<dyn PropertyArgs>)> {
        match self.property_index(property_id) {
            Some(i) => match &mut self.items[i] {
                (pos, WidgetItem::Property { importance, args }) => {
                    self.items_sorted = false;
                    Some((importance, pos, args))
                }
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
            WidgetItem::Instrinsic { .. } => None,
            WidgetItem::Property { importance, args } => Some((*importance, *pos, &**args)),
        })
    }

    /// iterate over mutable references to the current properties.
    pub fn properties_mut(&mut self) -> impl Iterator<Item = (&mut Importance, &mut NestPosition, &mut Box<dyn PropertyArgs>)> {
        self.items_sorted = false;
        self.items.iter_mut().filter_map(|(pos, it)| match it {
            WidgetItem::Instrinsic { .. } => None,
            WidgetItem::Property { importance, args } => Some((importance, pos, args)),
        })
    }

    /// If a child not is already set in the builder.
    ///
    /// If build without child the [`NilUiNode`] is used as the innermost node.
    pub fn has_child(&self) -> bool {
        self.child.is_some()
    }

    /// If the builder contains any properties.
    pub fn has_properties(&self) -> bool {
        self.items.iter().any(|(_, it)| matches!(it, WidgetItem::Property { .. }))
    }

    /// If the builder contains any intrinsic nodes.
    pub fn has_intrinsics(&self) -> bool {
        self.items.iter().any(|(_, it)| matches!(it, WidgetItem::Instrinsic { .. }))
    }

    /// If the builder contains any *unset* filters.
    pub fn has_unsets(&self) -> bool {
        !self.unset.is_empty()
    }

    /// If the builder contains any *when* filters.
    pub fn has_whens(&self) -> bool {
        !self.whens.is_empty()
    }

    /// Remove all properties.
    pub fn clear_properties(&mut self) {
        self.items.retain(|(_, it)| !matches!(it, WidgetItem::Property { .. }))
    }

    /// Remove all intrinsic nodes.
    pub fn clear_intrinsics(&mut self) {
        self.items.retain(|(_, it)| !matches!(it, WidgetItem::Instrinsic { .. }))
    }

    /// Remove all *unset* filters.
    ///
    /// Note that unset filters are applied on insert and on property insert, this is not undone by this clear.
    pub fn clear_unsets(&mut self) {
        self.unset.clear()
    }

    /// Remove all *when* directives.
    pub fn clear_whens(&mut self) {
        self.whens.clear();
    }

    /// Remove the custom build.
    pub fn clear_custom_build(&mut self) {
        self.custom_build = None;
    }

    /// Full clear.
    pub fn clear(&mut self) {
        self.items.clear();
        self.unset.clear();
        self.whens.clear();
        self.child = None;
        self.custom_build = None;
    }

    /// If a custom builder closure is set.
    pub fn is_custom_build(&self) -> bool {
        self.custom_build.is_some()
    }

    /// Set a custom `build` closure that gets called on [`build`] instead of the default implementation.
    ///
    /// The [`build`] is not recursive, if called again inside `build` it will use the default implementation,
    /// unless another custom build was set during build. If another custom build is set after `build` it is discarded.
    ///
    /// [`build`]: Self::build
    /// [`build_default`]: Self::build_default
    pub fn set_custom_build(&mut self, build: impl CustomWidgetBuild) {
        self.custom_build = Some(Box::new(build))
    }

    /// Instantiate and link all property and intrinsic nodes, returns the outermost node.
    ///
    /// Note that you can reuse the builder, but only after the previous build is deinited and dropped.
    ///
    /// Note that the build process can be replaced with a custom implementation using [`set_custom_build`], if set
    /// the custom implementation is called instead.
    ///
    /// [`set_custom_build`]: Self::set_custom_build
    pub fn build(&mut self) -> BoxedUiNode {
        if let Some(mut custom) = self.custom_build.take() {
            let r = custom.build(self);
            if self.custom_build.is_none() {
                self.custom_build = Some(custom);
            }
            r
        } else {
            self.build_default()
        }
    }

    /// Default [`build`] action.
    ///
    /// [`build`]: Self::build
    pub fn build_default(&mut self) -> BoxedUiNode {
        self.sort_items();

        let mut node = self
            .child
            .as_ref()
            .map(|c| c.take_on_init().boxed())
            .unwrap_or_else(|| NilUiNode.boxed());

        for (_, item) in &self.items {
            match item {
                WidgetItem::Instrinsic { child, node: n } => {
                    *child.borrow_mut() = mem::replace(&mut node, n.take_on_init().boxed());
                }
                WidgetItem::Property { args, .. } => {
                    node = args.instantiate(node);
                }
            }
        }

        node
    }

    fn property_index(&self, property_id: PropertyId) -> Option<usize> {
        self.items.iter().position(|(_, item)| match item {
            WidgetItem::Property { args, .. } => args.id() == property_id,
            WidgetItem::Instrinsic { .. } => false,
        })
    }

    fn sort_items(&mut self) {
        if !self.items_sorted {
            self.items.sort_by_key(|(pos, _)| *pos);
            self.items_sorted = true;
        }

        if !self.whens_sorted {
            self.whens.sort_by_key(|(imp, _)| *imp);
            self.whens_sorted = true;
        }
    }
}

/// Represents a custom widget build process.
pub trait CustomWidgetBuild: Any {
    /// Clone the build.
    ///
    /// This is called every time the [`WidgetBuilder`] is cloned.
    fn clone_boxed(&self) -> Box<dyn CustomWidgetBuild>;
    /// Apply the custom build.
    ///
    /// This is called every time the [`WidgetBuilder::build`] is called.
    fn build(&mut self, wgt: &mut WidgetBuilder) -> BoxedUiNode;
}
impl Clone for Box<dyn CustomWidgetBuild> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}
