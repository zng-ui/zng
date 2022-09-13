use std::any::TypeId;
use std::fmt;
use std::{cell::RefCell, mem, rc::Rc};

use crate::var::types::AnyWhenVarBuilder;
use crate::var::{AnyVar, *};
use crate::NilUiNode;
use crate::{context::WidgetContext, impl_ui_node, BoxedUiNode, UiNode};

/// Represents a node setup to dynamically swap child.
///
/// Any property node can be made adoptive by wrapping it with this node.
pub struct AdoptiveNode<U> {
    child: Rc<RefCell<BoxedUiNode>>,
    node: U,
    is_inited: bool,
}
impl<U: UiNode> AdoptiveNode<U> {
    /// Create the adoptive node, the [`AdoptiveChildNode`] must be used as the *property child*.
    pub fn new(create: impl FnOnce(AdoptiveChildNode) -> U) -> Self {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child);
        Self {
            child,
            node,
            is_inited: false,
        }
    }

    /// Create the adoptive node with a constructor that can fail.
    pub fn try_new<E>(create: impl FnOnce(AdoptiveChildNode) -> Result<U, E>) -> Result<Self, E> {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child)?;
        Ok(Self {
            child,
            node,
            is_inited: false,
        })
    }

    /// Replaces the child node, panics if the node is inited.
    ///
    /// Returns the previous child, the initial child is a [`NilUiNode`].
    pub fn replace_child(&mut self, new_child: impl UiNode) -> BoxedUiNode {
        assert!(!self.is_inited);
        mem::replace(&mut *self.child.borrow_mut(), new_child.boxed())
    }

    /// Returns `true` if this node is initialized in a UI tree.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Into child reference, node and if it is inited.
    pub fn into_parts(self) -> (Rc<RefCell<BoxedUiNode>>, U) {
        assert!(!self.is_inited);
        (self.child, self.node)
    }
}
#[impl_ui_node(
    delegate = &self.node,
    delegate_mut = &mut self.node,
)]
impl<U: UiNode> UiNode for AdoptiveNode<U> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = true;
        self.node.init(ctx);
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = false;
        self.node.deinit(ctx);
    }
}

/// Placeholder for the dynamic child of an [`AdoptiveNode`].
///
/// This node must be used as the property child of the adoptive node.
pub struct AdoptiveChildNode {
    child: Rc<RefCell<BoxedUiNode>>,
}
impl AdoptiveChildNode {
    fn nil() -> Self {
        Self {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
        }
    }
}
#[impl_ui_node(
    delegate = self.child.borrow(),
    delegate_mut = self.child.borrow_mut(),
)]
impl UiNode for AdoptiveChildNode {}

/// Property priority of dynamic properties.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynPropPriority {
    /// `child_layout`.
    ChildLayout = 0,
    /// `child_context`.
    ChildContext,
    /// `fill`.
    Fill,
    /// `border`.
    Border,
    /// `size`.
    Size,
    /// `layout`.
    Layout,
    /// `event`.
    Event,
    /// `context`.
    Context,
}
impl DynPropPriority {
    /// Number of priority items.
    pub const LEN: usize = DynPropPriority::Context as usize + 1;

    /// Cast index to variant.
    pub fn from_index(i: usize) -> Result<DynPropPriority, usize> {
        use DynPropPriority::*;
        match i {
            0 => Ok(ChildLayout),
            1 => Ok(ChildContext),
            2 => Ok(Fill),
            3 => Ok(Border),
            4 => Ok(Size),
            5 => Ok(Layout),
            6 => Ok(Event),
            7 => Ok(Context),
            n => Err(n),
        }
    }

    fn intrinsic_name(self) -> &'static str {
        match self {
            DynPropPriority::ChildLayout => "<new_child_layout>",
            DynPropPriority::ChildContext => "<new_child_context>",
            DynPropPriority::Fill => "<new_fill>",
            DynPropPriority::Border => "<new_border>",
            DynPropPriority::Size => "<new_size>",
            DynPropPriority::Layout => "<new_layout>",
            DynPropPriority::Event => "<new_event>",
            DynPropPriority::Context => "<new_context>",
        }
    }

    fn intrinsic_id(self) -> TypeId {
        match self {
            DynPropPriority::ChildLayout => {
                enum ChildLayoutType {}
                TypeId::of::<ChildLayoutType>()
            }
            DynPropPriority::ChildContext => {
                enum ChildContextType {}
                TypeId::of::<ChildContextType>()
            }
            DynPropPriority::Fill => {
                enum FillType {}
                TypeId::of::<FillType>()
            }
            DynPropPriority::Border => {
                enum BorderType {}
                TypeId::of::<BorderType>()
            }
            DynPropPriority::Size => {
                enum SizeType {}
                TypeId::of::<SizeType>()
            }
            DynPropPriority::Layout => {
                enum LayoutType {}
                TypeId::of::<LayoutType>()
            }
            DynPropPriority::Event => {
                enum EventType {}
                TypeId::of::<EventType>()
            }
            DynPropPriority::Context => {
                enum ContextType {}
                TypeId::of::<ContextType>()
            }
        }
    }
}

/// Error in call to [`DynPropertyFn`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynPropError {
    /// Property is `allowed_in_when = false`, it does not have variable inputs.
    NotAllowedInWhen,
    /// Property input does not match the expected number and/or type or variables expected.
    ArgsMismatch,
}
impl fmt::Display for DynPropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynPropError::NotAllowedInWhen => write!(f, "property is not `allowed_in_when`, it does not have var inputs"),
            DynPropError::ArgsMismatch => write!(f, "property expected different input variables"),
        }
    }
}
impl std::error::Error for DynPropError {}

/// Represents a dynamic arguments to a property set by `when` condition.
pub enum DynPropertyArgs {
    /// Arguments for a property with `impl IntoVar<T>` args, must match a [`BoxedVar<T>`] for each [`IntoVar<T>`] exactly.
    Args(Vec<Box<dyn AnyVar>>),
    /// Argument for a state (`is_`) property.
    State(StateVar),
    /// Arguments for a property with `impl IntoVar<T>` args that setups when conditions with the default value,
    /// must match a [`BoxedVar<T>`] for each [`IntoVar<T>`] exactly.
    When(Vec<AnyWhenVarBuilder>),
    /// Similar to `When`, but the user did not explicitly set a default value, only when condition values, a default
    /// value may be available anyway if the property has a default, if no default is available the property node is a
    /// no-op passthrough.
    WhenNoDefault(Vec<AnyWhenVarBuilder>),
}
impl DynPropertyArgs {
    /// Get a clone of the property argument, builds if it is when.
    pub fn get<T: VarValue>(&self, i: usize) -> Result<impl IntoVar<T>, DynPropError> {
        match self {
            Self::Args(a) => match a.get(i).and_then(|a| a.as_any().downcast_ref::<BoxedVar<T>>()) {
                Some(v) => Ok(v.clone()),
                None => Err(DynPropError::ArgsMismatch),
            },
            Self::State(s) => {
                if i == 0 && TypeId::of::<T>() == TypeId::of::<bool>() {
                    let cast = s.clone().boxed().into_any().as_box_any().downcast::<BoxedVar<T>>().unwrap();
                    Ok(cast)
                } else {
                    Err(DynPropError::ArgsMismatch)
                }
            }
            Self::When(w) => {
                if let Some(w) = w.get(i).and_then(|w| w.build()) {
                    Ok(w.boxed())
                } else {
                    Err(DynPropError::ArgsMismatch)
                }
            }
            Self::WhenNoDefault(w) => {
                if let Some(w) = w.get(i).and_then(|w| w.build()) {
                    Ok(w.boxed())
                } else {
                    Err(DynPropError::ArgsMismatch)
                }
            }
        }
    }

    /// Get a clone of the single argument for state properties.
    pub fn get_state(&self) -> Result<StateVar, DynPropError> {
        match self {
            Self::State(s) => Ok(s.clone()),
            _ => Err(DynPropError::ArgsMismatch),
        }
    }
}
impl fmt::Debug for DynPropertyArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Args(_) => write!(f, "Args(_)"),
            Self::State(_) => write!(f, "State(_)"),
            Self::When(_) => write!(f, "When(_)"),
            Self::WhenNoDefault(_) => write!(f, "WhenNoDefault(_)"),
        }
    }
}
impl Clone for DynPropertyArgs {
    fn clone(&self) -> Self {
        match self {
            Self::Args(a) => Self::Args(a.iter().map(|a| a.clone_any()).collect()),
            Self::State(a) => Self::State(a.clone()),
            Self::When(a) => Self::When(a.clone()),
            Self::WhenNoDefault(a) => Self::WhenNoDefault(a.clone()),
        }
    }
}

/// Represents a property constructor function activated with dynamically defined input variables.
///
/// You can use the [`property_dyn_fn!`] macro to get the constructor for a property.
pub type DynPropertyFn = fn(BoxedUiNode, &DynPropertyArgs) -> Result<BoxedUiNode, (BoxedUiNode, DynPropError)>;

#[doc(hidden)]
pub fn not_allowed_in_when_dyn_ctor(child: BoxedUiNode, _: &DynPropertyArgs) -> Result<BoxedUiNode, (BoxedUiNode, DynPropError)> {
    Err((child, DynPropError::NotAllowedInWhen))
}

///<span data-del-macro-root></span> Gets the [`DynPropertyFn`] of a property.
///
/// If the property is not `allowed_in_when` returns a constructor that always returns [`DynPropError::NotAllowedInWhen`].
#[macro_export]
macro_rules! property_dyn_fn {
    ($property:path) => {{
        use $property as __property;

        __property::code_gen! {if allowed_in_when=>
            __property::dyn_ctor
        }
        __property::code_gen! {if !allowed_in_when=>
            $crate::not_allowed_in_when_dyn_ctor
        }
    }};
}
#[doc(inline)]
pub use crate::property_dyn_fn;

///<span data-del-macro-root></span> Gets a [`TypeId`] that uniquely identifies a property function.
#[macro_export]
macro_rules! property_type_id {
    ($property_path:path) => {{
        use $property_path::PropertyType as __Type;
        std::any::TypeId::of::<__Type>()
    }};
}
#[doc(inline)]
pub use crate::property_type_id;

/// Information about how a dynamic property can be configured to update by `when` conditions.
#[derive(Clone)]
pub enum DynPropWhenInfo {
    /// Property is `allowed_in_when = false`.
    NotAllowed,

    /// Property is `allowed_in_when = true`.
    Allowed {
        /// Dynamic constructor, can generate another property instance with newly configured when conditions.
        new_fn: DynPropertyFn,
        /// Clone of the args used to instantiate the property.
        args: DynPropertyArgs,
    },
}
impl fmt::Debug for DynPropWhenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAllowed => write!(f, "NotAllowed"),
            Self::Allowed { args, .. } => f.debug_struct("Allowed").field("args", args).finish_non_exhaustive(),
        }
    }
}

/// Represents an widget property use in dynamic initialization.
///
/// See [`DynWidgetPart`] for details.
pub struct DynProperty {
    /// The property node, setup as an adoptive node that allows swapping the child node, the input variables
    /// and whens are already setup, a clone of then is available in `when_info`.
    pub node: AdoptiveNode<BoxedUiNode>,

    /// The property constructor function and initial args, if it is `allowed_in_when = true`.
    pub when_info: DynPropWhenInfo,

    /// Name of the property as it was set in the widget.
    ///
    /// All of these assigns have the same name `foo`:
    ///
    /// ```
    /// # macro_rules! _demo { () => {
    /// path::to::foo = true;
    /// bar as foo = true;
    /// foo = true;
    /// # }}
    /// ```
    pub name: &'static str,

    /// Type ID that uniquely identify the property.
    ///
    /// The [`property_type_id!`] macro can be used to extract the ID of a property function.
    pub id: TypeId,

    /// The *importance* of the property, that is, if it is set in the widget default or the widget instance.
    ///
    /// Defines what instance of the same property replaces the other.
    pub importance: DynPropImportance,

    /// Defines the property *position* within the same priority group, larger numbers means more likely to be inside
    /// the other properties.
    ///
    /// Note that the property priority it self is recorded, but it is the same priority of the widget constructor function
    /// that received this property instance.
    pub priority_index: i16,
}
impl fmt::Debug for DynProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynProperty")
            .field("name", &self.name)
            .field("id", &self.id)
            .field("when_info", &self.when_info)
            .field("importance", &self.importance)
            .field("priority_index", &self.priority_index)
            .finish_non_exhaustive()
    }
}
impl DynProperty {
    /// Merge or replace `self` by `other`, returns the replacement property, or `(self, other)` in case the
    /// properties are not compatible.
    ///
    /// What property *replaces* the other depends on the [`importance`] of both, if the importance is the same the
    /// `other` property *replaces* `self`. The replacement can be a input property or it can be a new instance that
    /// merges the when conditions of both input properties.
    ///
    /// [`importance`]: DynProperty.importance
    pub fn merge_replace(self, other: DynProperty) -> Result<DynProperty, (DynProperty, DynProperty)> {
        if self.name == other.name {
            Ok(MergeReplaceSolver::merge_replace(self, other))
        } else {
            Err((self, other))
        }
    }
}
impl MergeReplaceSolver for DynProperty {
    fn name(&self) -> &'static str {
        self.name
    }

    fn id(&self) -> TypeId {
        self.id
    }

    fn importance(&self) -> DynPropImportance {
        self.importance
    }

    fn when_info(&self) -> &DynPropWhenInfo {
        &self.when_info
    }

    fn replace_node(mut self, node: AdoptiveNode<BoxedUiNode>, when_info: DynPropWhenInfo) -> Self {
        self.node = node;
        self.when_info = when_info;
        self
    }
}

/// Represents the properties and intrinsic node of a priority in a [`DynWidget`].
pub struct DynWidgetPart {
    /// Properties set for the priority.
    pub properties: Vec<DynProperty>,

    /// Return node of the constructor for the priority.
    pub intrinsic: AdoptiveNode<BoxedUiNode>,
}
impl DynWidgetPart {
    /// Modify/replace the `intrinsic` node.
    ///
    /// Panics is called in an inited context.
    pub fn modify_intrinsic(&mut self, build: impl FnOnce(BoxedUiNode) -> BoxedUiNode) {
        assert!(!self.intrinsic.is_inited());

        let node = mem::replace(&mut self.intrinsic.node, NilUiNode.boxed());
        self.intrinsic.node = build(node);
    }
}
impl fmt::Debug for DynWidgetPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidgetPart")
            .field("properties", &self.properties)
            .finish_non_exhaustive()
    }
}

/// Represents a dynamic widget final part, available in `new_dyn`.
pub struct DynWidget {
    /// Innermost node, returned by the `new_child` constructor function.
    pub child: BoxedUiNode,

    /// Parts for each priority, from `child_layout` to `context`.
    pub parts: [DynWidgetPart; DynPropPriority::LEN],
}
impl fmt::Debug for DynWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidget").field("parts", &self.parts).finish_non_exhaustive()
    }
}
impl DynWidget {
    /// Convert the widget to an editable [`UiNode`].
    ///
    /// If `include_intrinsic` is `true` the widget constructor nodes are also include, otherwise only the property nodes are included.
    pub fn into_node(self, include_intrinsic: bool) -> DynWidgetNode {
        DynWidgetNode::new(self, include_intrinsic)
    }

    #[doc(hidden)]
    pub fn modify_context_intrinsic_v1(&mut self, build: impl FnOnce(BoxedUiNode) -> BoxedUiNode) {
        self.parts[DynPropPriority::Context as usize].modify_intrinsic(build);
    }
}

/// Importance index of a property in the group of properties of the same priority in the same widget.
///
/// Properties of a widget are grouped by [`DynPropPriority`], within these groups properties of the same name
/// override by importance, zero is the least important, `u32::MAX` is the most.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynPropImportance(pub u32);
impl DynPropImportance {
    /// Least important, is overridden by all others.
    ///
    /// Is `0`.
    pub const LEAST: DynPropImportance = DynPropImportance(0);

    /// Property assigned by the widget declaration.
    ///
    /// Is `u16::MAX`.
    ///
    /// ```
    /// # macro_rules! _demo { () => {
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     properties! {
    ///         bar = true;
    ///     }
    /// }
    ///
    /// foo!() // `bar` assigned in the widget.
    /// # }}
    /// ```
    pub const WIDGET: DynPropImportance = DynPropImportance(u16::MAX as u32);

    ///  Property assigned by the widget instance.
    ///
    /// Is `u32::MAX - u16::MAX as u32`.
    ///
    /// ```
    /// # fn main() { }
    /// # macro_rules! foo { ($($tt:tt)*) => { } }
    /// foo! {
    ///     bar = true;// assign in the instance.
    /// }
    /// ```
    pub const INSTANCE: DynPropImportance = DynPropImportance(u32::MAX - u16::MAX as u32);

    /// Most important, overrides all others.
    ///
    /// Is `u32::MAX`.
    pub const MOST: DynPropImportance = DynPropImportance(u32::MAX);
}
impl fmt::Debug for DynPropImportance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DynPropImportance::LEAST => write!(f, "LEAST"),
            DynPropImportance::WIDGET => write!(f, "WIDGET"),
            DynPropImportance::INSTANCE => write!(f, "INSTANCE"),
            DynPropImportance::MOST => write!(f, "MOST"),
            DynPropImportance(n) => write!(f, "{n}"),
        }
    }
}

#[doc(hidden)]
pub struct DynWidgetBuilderV1 {
    child: BoxedUiNode,
    parts: Vec<DynWidgetPart>,
}
impl DynWidgetBuilderV1 {
    pub fn begin(child: impl UiNode) -> Self {
        DynWidgetBuilderV1 {
            child: child.boxed(),
            parts: Vec::with_capacity(DynPropPriority::LEN),
        }
    }

    pub fn begin_part(&self) -> (AdoptiveChildNode, DynWidgetPartBuilderV1) {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();

        (ad_child, DynWidgetPartBuilderV1 { child, properties: vec![] })
    }

    pub fn finish_part(&mut self, part: DynWidgetPartBuilderV1, intrinsic_node: impl UiNode) {
        let node = AdoptiveNode {
            child: part.child,
            node: intrinsic_node.boxed(),
            is_inited: false,
        };

        self.parts.push(DynWidgetPart {
            properties: part.properties,
            intrinsic: node,
        })
    }

    pub fn finish(self) -> DynWidget {
        debug_assert_eq!(self.parts.len(), DynPropPriority::LEN);
        DynWidget {
            child: self.child,
            parts: self.parts.try_into().unwrap(),
        }
    }
}
#[doc(hidden)]
pub struct DynWidgetPartBuilderV1 {
    child: Rc<RefCell<BoxedUiNode>>,
    properties: Vec<DynProperty>,
}
impl DynWidgetPartBuilderV1 {
    pub fn begin_property(&self) -> (AdoptiveChildNode, DynPropertyBuilderV1) {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        (ad_child, DynPropertyBuilderV1 { child })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_property_state(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        id: TypeId,
        user_assigned: bool,
        priority_index: i16,

        new_fn: DynPropertyFn,
        state: StateVar,
    ) {
        self.finish(
            property,
            property_node,
            name,
            id,
            user_assigned,
            priority_index,
            DynPropWhenInfo::Allowed {
                new_fn,
                args: DynPropertyArgs::State(state),
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_property_allowed_in_when(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        id: TypeId,
        user_assigned: bool,
        priority_index: i16,

        new_fn: DynPropertyFn,
        args: Vec<Box<dyn AnyVar>>,
    ) {
        self.finish(
            property,
            property_node,
            name,
            id,
            user_assigned,
            priority_index,
            DynPropWhenInfo::Allowed {
                new_fn,
                args: DynPropertyArgs::Args(args),
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_property_with_when(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        id: TypeId,
        user_assigned: bool,
        priority_index: i16,

        new_fn: DynPropertyFn,
        args: Vec<AnyWhenVarBuilder>,
        default_set: bool,
    ) {
        self.finish(
            property,
            property_node,
            name,
            id,
            user_assigned,
            priority_index,
            DynPropWhenInfo::Allowed {
                new_fn,
                args: if default_set {
                    DynPropertyArgs::When(args)
                } else {
                    DynPropertyArgs::WhenNoDefault(args)
                },
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_property_not_allowed_in_when(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        id: TypeId,
        user_assigned: bool,
        priority_index: i16,
    ) {
        self.finish(
            property,
            property_node,
            name,
            id,
            user_assigned,
            priority_index,
            DynPropWhenInfo::NotAllowed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        id: TypeId,
        user_assigned: bool,
        priority_index: i16,
        when_info: DynPropWhenInfo,
    ) {
        let node = AdoptiveNode {
            child: property.child,
            node: property_node.boxed(),
            is_inited: false,
        };

        self.properties.push(DynProperty {
            node,
            when_info,
            name,
            id,
            importance: if user_assigned {
                DynPropImportance::INSTANCE
            } else {
                DynPropImportance::WIDGET
            },
            priority_index,
        })
    }
}

#[doc(hidden)]
pub struct DynPropertyBuilderV1 {
    child: Rc<RefCell<BoxedUiNode>>,
}

/// Represents a [`DynWidget`] that can be used as the *outermost* node of a widget and *edited*
/// with property overrides and new `when` blocks.
pub struct DynWidgetNode {
    // Unique ID used to validate snapshots.
    id: DynWidgetNodeId,

    // innermost child.
    //
    // The Rc changes to the `child` of the innermost property when bound and a new Rc when unbound,
    // the interior only changes when `replace_child` is used.
    child: Rc<RefCell<BoxedUiNode>>,

    // property and intrinsic nodes from innermost to outermost.
    items: Vec<DynWidgetItem>,
    // exclusive end of each priority range in `properties`
    priority_ranges: [usize; DynPropPriority::LEN],

    // outermost node.
    //
    // The Rc changes to the `node` of the outermost property, the interior is not modified from here.
    node: Rc<RefCell<BoxedUiNode>>,

    is_inited: bool,
    is_bound: bool,
}
impl Default for DynWidgetNode {
    fn default() -> Self {
        let nil = Rc::new(RefCell::new(NilUiNode.boxed()));
        Self {
            id: DynWidgetNodeId::new_unique(),
            child: nil.clone(),
            items: vec![],
            priority_ranges: [0; DynPropPriority::LEN],
            node: nil,
            is_inited: false,
            is_bound: true,
        }
    }
}
impl fmt::Debug for DynWidgetNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidgetNode")
            .field("id", &self.id)
            .field("items", &self.items)
            .field("is_inited", &self.is_inited)
            .field("is_bound", &self.is_bound)
            .finish_non_exhaustive()
    }
}
impl DynWidgetNode {
    fn new(wgt: DynWidget, include_intrinsic: bool) -> Self {
        let mut priority_ranges = [0; DynPropPriority::LEN];
        let mut items = vec![];
        for (i, part) in wgt.parts.into_iter().enumerate() {
            items.extend(part.properties.into_iter().map(DynWidgetItem::new));

            if include_intrinsic {
                items.push(DynWidgetItem::new_instrinsic(
                    DynPropPriority::from_index(i).unwrap(),
                    part.intrinsic,
                ));
            }

            priority_ranges[i] = items.len();
        }

        let node = Rc::new(RefCell::new(wgt.child));

        DynWidgetNode {
            id: DynWidgetNodeId::new_unique(),
            child: node.clone(),
            node,
            items,
            priority_ranges,
            is_inited: false,
            is_bound: false,
        }
    }

    /// Returns `true` if this node is initialized in a UI tree.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Returns `true` if this node contains no property and intrinsic nodes, note that it can still contain a child node.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Replaces the inner child node, panics if the node is inited.
    ///
    /// Returns the previous child, the initial child is the [`DynWidget::child`] or a [`NilUiNode`] for the default.
    pub fn replace_child(&mut self, new_child: impl UiNode) -> BoxedUiNode {
        assert!(!self.is_inited);
        mem::replace(&mut *self.child.borrow_mut(), new_child.boxed())
    }

    /// Insert `properties` in the chain, overrides properties with the same name, priority and with less or equal importance.
    ///
    /// Assumes the `properties` are in the same order as received in a widget's dynamic constructor, that is,
    /// sorted by priority index and reversed, innermost first.
    ///
    /// Panics if `self` or any of the `properties` are inited.
    pub fn insert(&mut self, priority: DynPropPriority, properties: Vec<DynProperty>) {
        assert!(!self.is_inited);

        if properties.is_empty() {
            return;
        }

        assert!(!properties[0].node.is_inited());

        if self.is_bound {
            // will rebind next init.
            self.unbind_all();
        }

        let priority = priority as usize;
        for p in properties {
            self.insert_merge_replace(priority, DynWidgetItem::new(p));
        }
    }

    /// Insert `other` in the properties chain, overrides properties with the same name, priority and with less or equal importance.
    ///
    /// Panics if `self` or `other` are inited, ignores the `other` child.
    pub fn insert_all(&mut self, other: DynWidgetNode) {
        let mut other = other;

        assert!(!self.is_inited);
        assert!(!other.is_inited);

        if self.is_bound {
            // will rebind next init.
            self.unbind_all();
        }
        if other.is_bound {
            other.unbind_all();
        }

        let mut s = 0;
        let mut items = other.items.into_iter();
        for (p, e) in other.priority_ranges.into_iter().enumerate() {
            while s < e {
                self.insert_merge_replace(p, items.next().unwrap());
                s += 1;
            }
        }
    }

    /// Create an snapshot of the current properties.
    ///
    /// The snapshot can be used to [`restore`] the properties to a state before it was overridden by an insert.
    ///
    /// Panics if the node is inited.
    ///
    /// [`restore`]: DynWidgetNode::restore
    pub fn snapshot(&mut self) -> DynWidgetSnapshot {
        assert!(!self.is_inited);

        if self.is_bound {
            self.unbind_all();
        }

        DynWidgetSnapshot {
            id: self.id,
            items: self.items.iter().map(DynWidgetItem::snapshot).collect(),
            priority_ranges: self.priority_ranges,
        }
    }

    /// Restores the properties to the snapshot, if it was taken from the same properties.
    ///
    /// Panics if the node is inited.
    pub fn restore(&mut self, snapshot: DynWidgetSnapshot) -> Result<(), DynWidgetSnapshot> {
        assert!(!self.is_inited);

        if self.id == snapshot.id {
            if self.is_bound {
                self.unbind_all();
            }

            self.items.clear();
            self.items.extend(snapshot.items.into_iter().map(DynWidgetItem::restore));
            self.priority_ranges = snapshot.priority_ranges;

            Ok(())
        } else {
            Err(snapshot)
        }
    }

    fn bind_all(&mut self) {
        debug_assert!(!self.is_bound);

        if !self.items.is_empty() {
            // move the child to the innermost property child.

            debug_assert_eq!(
                self.items[0].child.borrow().actual_type_id(),
                std::any::TypeId::of::<NilUiNode>(),
                "`{}` already has a child",
                self.items[0].name
            );
            mem::swap(&mut *self.child.borrow_mut(), &mut *self.items[0].child.borrow_mut());
            // save the new child address.
            self.child = self.items[0].child.clone();

            // chain properties.

            for i in 0..self.items.len() {
                let (a, b) = self.items.split_at_mut(i + 1);
                if let (Some(inner), Some(outer)) = (a.last_mut(), b.first()) {
                    inner.set_parent(outer);
                }
            }

            // save the new outermost node address.
            self.node = self.items[self.items.len() - 1].node.clone();
        }

        self.is_bound = true;
    }

    fn unbind_all(&mut self) {
        debug_assert!(self.is_bound);

        if !self.items.is_empty() {
            let child = mem::replace(&mut *self.child.borrow_mut(), NilUiNode.boxed());

            self.child = Rc::new(RefCell::new(child));

            for i in 0..self.items.len() {
                let (a, b) = self.items.split_at_mut(i + 1);
                if let (Some(inner), Some(outer)) = (a.last_mut(), b.first()) {
                    inner.unset_parent(outer);
                }
            }

            self.node = self.child.clone();
        }

        self.is_bound = false;
    }

    fn insert_merge_replace(&mut self, priority: usize, item: DynWidgetItem) {
        let item = if let Some(i) = self.items.iter().position(|b| b.name == item.name) {
            // merge or replace

            let base_item = self.items.remove(i);
            for p in &mut self.priority_ranges[priority..] {
                *p -= 1;
            }

            base_item.merge_replace(item)
        } else {
            item
        };

        // insert
        let range = self.priority_range(priority);
        let mut insert = range.end;
        for i in range {
            if self.items[i].priority_index <= item.priority_index {
                insert = i; // insert *inside* other items with the same priority or less.
                break;
            }
        }
        self.items.insert(insert, item);

        for p in &mut self.priority_ranges[priority..] {
            *p += 1;
        }
    }

    fn priority_range(&self, priority: usize) -> std::ops::Range<usize> {
        let ps = if priority == 0 { 0 } else { self.priority_ranges[priority - 1] };
        ps..self.priority_ranges[priority]
    }
}
#[impl_ui_node(
    delegate = self.node.borrow(),
    delegate_mut = self.node.borrow_mut(),
)]
impl UiNode for DynWidgetNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = true;
        if !self.is_bound {
            self.bind_all();
        }
        self.node.borrow_mut().init(ctx);
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = false;
        self.node.borrow_mut().deinit(ctx);
    }
}

/// Represents a snapshot of a [`DynWidgetNode`] value.
///
/// The snapshot can be used to [`restore`] the properties to a state before it was overridden by an insert.
///
/// [`restore`]: DynWidgetNode::restore
pub struct DynWidgetSnapshot {
    id: DynWidgetNodeId,
    items: Vec<PropertyItemSnapshot>,
    priority_ranges: [usize; DynPropPriority::LEN],
}
impl fmt::Debug for DynWidgetSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidgetSnapshot")
            .field("id", &self.id)
            .field("items", &self.items)
            .finish_non_exhaustive()
    }
}

/// Widget property or intrinsic node.
struct DynWidgetItem {
    // item child, the `Rc` does not change, only the interior.
    child: Rc<RefCell<BoxedUiNode>>,
    // item node, the `Rc` changes, but it always points to the same node.
    node: Rc<RefCell<BoxedUiNode>>,
    // original `node`, preserved when parent is set, reused when unset.
    snapshot_node: Option<Rc<RefCell<BoxedUiNode>>>,

    // property name or "<new_*>" intrinsic.
    name: &'static str,
    // property id or "intrinsic_id"
    id: TypeId,
    // property index cast to `i32` or `i32::MIN+1` for intrinsic.
    priority_index: i32,
    // property importance, or `WIDGET` for intrinsic.
    importance: DynPropImportance,
    // initialization mode, constructor and args.
    when_info: DynPropWhenInfo,
}
impl DynWidgetItem {
    fn new(property: DynProperty) -> Self {
        assert!(!property.node.is_inited());

        let (child, node) = property.node.into_parts();
        DynWidgetItem {
            child,
            node: Rc::new(RefCell::new(node)),
            snapshot_node: None,
            name: property.name,
            id: property.id,
            priority_index: property.priority_index as i32,
            importance: property.importance,
            when_info: property.when_info,
        }
    }

    fn new_instrinsic(priority: DynPropPriority, node: AdoptiveNode<BoxedUiNode>) -> Self {
        assert!(!node.is_inited());

        let (child, node) = node.into_parts();

        DynWidgetItem {
            child,
            node: Rc::new(RefCell::new(node)),
            snapshot_node: None,
            name: priority.intrinsic_name(),
            id: priority.intrinsic_id(),
            priority_index: i32::MIN + 1,
            importance: DynPropImportance::WIDGET,
            when_info: DynPropWhenInfo::NotAllowed,
        }
    }

    /// Set `self` as the child of `other`.
    fn set_parent(&mut self, other: &DynWidgetItem) {
        debug_assert_eq!(
            other.child.borrow().actual_type_id(),
            std::any::TypeId::of::<NilUiNode>(),
            "`{}` already has a child",
            other.name
        );

        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
        self.snapshot_node = Some(self.node.clone());
        self.node = other.child.clone();
    }

    /// Unset `self` as the child of `other`.
    fn unset_parent(&mut self, other: &DynWidgetItem) {
        debug_assert!(
            Rc::ptr_eq(&self.node, &other.child),
            "`{}` is not the parent of `{}`",
            other.name,
            self.name
        );

        self.node = self.snapshot_node.take().unwrap();
        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
    }
}
impl fmt::Debug for DynWidgetItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItem")
            .field("name", &self.name)
            .field("id", &self.id)
            .field("priority_index", &self.priority_index)
            .field("importance", &self.importance)
            .field("when_info", &self.when_info)
            .finish_non_exhaustive()
    }
}
impl MergeReplaceSolver for DynWidgetItem {
    fn name(&self) -> &'static str {
        self.name
    }

    fn id(&self) -> TypeId {
        self.id
    }

    fn importance(&self) -> DynPropImportance {
        self.importance
    }

    fn when_info(&self) -> &DynPropWhenInfo {
        &self.when_info
    }

    fn replace_node(mut self, node: AdoptiveNode<BoxedUiNode>, when_info: DynPropWhenInfo) -> Self {
        let (child, node) = node.into_parts();
        self.child = child;
        self.node = Rc::new(RefCell::new(node));
        self.when_info = when_info;
        self
    }
}

unique_id_32! {
    struct DynWidgetNodeId;
}
impl fmt::Debug for DynWidgetNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DynWidgetNodeId").field(&self.sequential()).finish()
    }
}

struct PropertyItemSnapshot {
    child: Rc<RefCell<BoxedUiNode>>,
    node: Rc<RefCell<BoxedUiNode>>,
    name: &'static str,
    id: TypeId,
    priority_index: i32,
    importance: DynPropImportance,
    when_info: DynPropWhenInfo,
}
impl fmt::Debug for PropertyItemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItemSnapshot")
            .field("name", &self.name)
            .field("id", &self.name)
            .field("priority_index", &self.priority_index)
            .field("importance", &self.importance)
            .field("when_info", &self.when_info)
            .finish_non_exhaustive()
    }
}

impl DynWidgetItem {
    fn snapshot(&self) -> PropertyItemSnapshot {
        PropertyItemSnapshot {
            child: self.child.clone(),
            node: self.node.clone(),
            name: self.name,
            id: self.id,
            priority_index: self.priority_index as i32,
            importance: self.importance,
            when_info: self.when_info.clone(),
        }
    }

    fn restore(snapshot: PropertyItemSnapshot) -> Self {
        DynWidgetItem {
            child: snapshot.child,
            node: snapshot.node,
            snapshot_node: None,
            name: snapshot.name,
            id: snapshot.id,
            priority_index: snapshot.priority_index as i32,
            importance: snapshot.importance,
            when_info: snapshot.when_info,
        }
    }
}

trait MergeReplaceSolver {
    fn name(&self) -> &'static str;
    fn id(&self) -> TypeId;
    fn importance(&self) -> DynPropImportance;
    fn when_info(&self) -> &DynPropWhenInfo;

    fn replace_node(self, node: AdoptiveNode<BoxedUiNode>, when_info: DynPropWhenInfo) -> Self;

    fn merge_replace(self, other: Self) -> Self
    where
        Self: Sized,
    {
        let (base, over) = if self.importance() <= other.importance() {
            (self, other)
        } else {
            (other, self)
        };
        if base.id() == over.id() {
            match (&base.when_info(), &over.when_info()) {
                (DynPropWhenInfo::NotAllowed, _) | (_, DynPropWhenInfo::NotAllowed) => over,
                (DynPropWhenInfo::Allowed { args: base_args, .. }, DynPropWhenInfo::Allowed { new_fn, args }) => {
                    use DynPropertyArgs::*;
                    match (base_args, args) {
                        // can't merge state reads.
                        (State(_), _) | (_, State(_)) => over,
                        // base has no when parts, over has a complete "default".
                        (Args(_), Args(_) | When(_)) => over,
                        // merge in the default.
                        (Args(default), WhenNoDefault(when)) | (WhenNoDefault(when), Args(default)) | (When(when), Args(default)) => {
                            let mut when = when.clone();

                            for (w, d) in when.iter_mut().zip(default) {
                                w.set_default_any(d.clone_any());
                            }

                            let new_fn = *new_fn;
                            over.merge_replace_finish_merge(new_fn, When(when))
                        }
                        // merge, keep base default.
                        (When(base_when), WhenNoDefault(when)) => {
                            let mut new_when = base_when.clone();

                            for (w, c) in new_when.iter_mut().zip(when) {
                                w.extend(c);
                            }

                            let new_fn = *new_fn;
                            over.merge_replace_finish_merge(new_fn, When(new_when))
                        }
                        // merge, replace default.
                        (When(base_when) | WhenNoDefault(base_when), When(when) | WhenNoDefault(when)) => {
                            let mut new_when = base_when.clone();

                            for (w, c) in new_when.iter_mut().zip(when) {
                                w.replace_extend(c);
                            }

                            let new_fn = *new_fn;
                            over.merge_replace_finish_merge(new_fn, When(new_when))
                        }
                    }
                }
            }
        } else {
            over
        }
    }
    fn merge_replace_finish_merge(self, new_fn: DynPropertyFn, args: DynPropertyArgs) -> Self
    where
        Self: Sized,
    {
        match AdoptiveNode::try_new(|child| new_fn(child.boxed(), &args)) {
            Ok(node) => self.replace_node(node, DynPropWhenInfo::Allowed { new_fn, args }),
            Err((_, e)) => {
                tracing::error!("failed `{:?}` when merge, will fully replace, {}", (self.name(), self.id()), e);
                self
            }
        }
    }
}
