use std::fmt;
use std::{cell::RefCell, mem, rc::Rc};

use crate::var::BoxedVar;
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
}

/// Error in call to [`DynProperty::new`].
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

/// Represents a dynamic input to a property set by `when` condition.
pub struct DynPropertyInput {
    args: Vec<Box<dyn crate::var::AnyVar>>,
}
impl fmt::Debug for DynPropertyInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DynPropertyInput {{ args: <{}> }}", self.args.len())
    }
}

/// Represents a property constructor function activated with dynamically defined input variables.
pub type DynPropertyFn = fn(BoxedUiNode, &DynPropertyInput) -> Result<BoxedUiNode, DynPropError>;

/// Represents the dynamic constructor and default input of a property that can update due to when condition.
pub struct DynPropertyWhenInfo {
    /// Dynamic constructor.
    pub new_fn: DynPropertyFn,
    /// Default input.
    pub input: DynPropertyInput,
}

/// Information about how a dynamic property can be configured to update by `when` conditions.
pub enum DynPropWhenInfo {
    /// Property is `allowed_in_when = false`.
    NotAllowedInWhen,

    /// Property is `allowed_in_when = true`.
    Assignable {
        /// Dynamic constructor, can generate another property instance with newly configured when conditions.
        new_fn: DynPropertyFn,
        /// Default input, can be used in when conditions.
        defaults: DynPropertyInput,
    },

    /// Property is read in a `when` expression.
    Condition {
        /// Index of the condition in the [`DynWidget`].
        index: usize,
    },
}
impl fmt::Debug for DynPropWhenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAllowedInWhen => write!(f, "NotAllowedInWhen"),
            Self::Assignable { defaults, .. } => f.debug_struct("Assignable").field("defaults", defaults).finish_non_exhaustive(),
            Self::Condition { index } => f.debug_struct("Condition").field("index", index).finish(),
        }
    }
}

/// Represents an widget property use in dynamic initialization.
///
/// See [`DynWidgetPart`] for details.
pub struct DynProperty {
    /// The property node, setup as an adoptive node that allows swapping the child node, the input variables
    /// is not setup for `when` condition, it is only the default value directly.
    pub node: AdoptiveNode<BoxedUiNode>,

    /// The property constructor function and default input vars, if it is `allowed_in_when = true`.
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
            .field("when_info", &self.when_info)
            .field("importance", &self.importance)
            .field("priority_index", &self.priority_index)
            .finish_non_exhaustive()
    }
}

/// Represents an widget `when` condition in dynamic initialization.
pub struct DynWhenCondition {
    /// The condition result.
    pub condition: BoxedVar<bool>,
    /// Inputs assigned for each property if when the condition is `true`.
    pub assigns: Vec<(&'static str, DynPropertyInput)>,
}
impl fmt::Debug for DynWhenCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWhenCondition")
            .field("assigns", &self.assigns)
            .finish_non_exhaustive()
    }
}

/// Represents the properties and intrinsic node of a priority in a [`DynWidget`].
pub struct DynWidgetPart {
    /// Properties set for the priority.
    pub properties: Vec<DynProperty>,

    /// Intrinsic node constructed for the priority.
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
    /// Innermost node, returned by `new_child`.
    pub child: BoxedUiNode,

    /// Parts for each priority, from `child_layout` to `context`.
    pub parts: [DynWidgetPart; DynPropPriority::LEN],

    /// When conditions set in the widget.
    pub whens: Vec<DynWhenCondition>,
}
impl fmt::Debug for DynWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidget")
            .field("parts", &self.parts)
            .field("whens", &self.whens)
            .finish_non_exhaustive()
    }
}
impl DynWidget {
    /// Convert the widget to an editable [`UiNode`].
    pub fn into_node(self) -> DynWidgetNode {
        DynWidgetNode::new(self)
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
            whens: vec![],
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

    pub fn finish_property(
        &mut self,
        property: DynPropertyBuilderV1,
        property_node: impl UiNode,
        name: &'static str,
        user_assigned: bool,
        priority_index: i16,
        is_when_condition: bool,
    ) {
        let node = AdoptiveNode {
            child: property.child,
            node: property_node.boxed(),
            is_inited: false,
        };

        self.properties.push(DynProperty {
            node,
            name,
            when_info: DynPropWhenInfo::NotAllowedInWhen,
            priority_index,
            importance: if user_assigned {
                DynPropImportance::INSTANCE
            } else {
                DynPropImportance::WIDGET
            },
        });
    }
}

#[doc(hidden)]
pub struct DynPropertyBuilderV1 {
    child: Rc<RefCell<BoxedUiNode>>,
}

/// Represents a [`DynamicWidget`] that can be used as the *outermost* node of a widget and *edited*
/// with property overrides and new `when` blocks.
pub struct DynWidgetNode {
    // Unique ID used to validate snapshots.
    id: DynWidgetNodeId,

    // innermost child.
    //
    // The Rc changes to the `child` of the innermost property when bound and a new Rc when unbound,
    // the interior only changes when `replace_child` is used.
    child: Rc<RefCell<BoxedUiNode>>,

    // properties from innermost to outermost.
    properties: Vec<PropertyItem>,
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
            properties: vec![],
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
            .field("properties", &self.properties)
            .field("is_inited", &self.is_inited)
            .field("is_bound", &self.is_bound)
            .finish_non_exhaustive()
    }
}
impl DynWidgetNode {
    fn new(wgt: DynWidget) -> Self {
        let mut priority_ranges = [0; DynPropPriority::LEN];
        let mut properties = vec![];
        for (i, part) in wgt.parts.into_iter().enumerate() {
            properties.extend(part.properties.into_iter().map(PropertyItem::new));
            priority_ranges[i] = properties.len();
        }

        let node = Rc::new(RefCell::new(wgt.child));

        DynWidgetNode {
            id: DynWidgetNodeId::new_unique(),
            child: node.clone(),
            node,
            properties,
            priority_ranges,
            is_inited: false,
            is_bound: false,
        }
    }

    /// Returns `true` if this node is initialized in a UI tree.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Returns `true` if this node contains no properties, note that it can still contain a child node.
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
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

        self.insert_impl(priority as usize, properties.len(), properties.into_iter().map(PropertyItem::new));
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
        let mut properties = other.properties.into_iter();
        for p in 0..DynPropPriority::LEN {
            let e = other.priority_ranges[p];

            let n = e - s;
            if n > 0 {
                self.insert_impl(p, n, properties.by_ref().take(n));
            }

            s = e;
        }
    }

    /// Create an snapshot of the current properties.
    ///
    /// The snapshot can be used to [`restore`] the properties to a state before it was overridden by an insert.
    ///
    /// Panics if the node is inited.
    ///
    /// [`restore`]: DynProperties::restore
    pub fn snapshot(&mut self) -> DynWidgetSnapshot {
        assert!(!self.is_inited);

        if self.is_bound {
            self.unbind_all();
        }

        DynWidgetSnapshot {
            id: self.id,
            properties: self.properties.iter().map(PropertyItem::snapshot).collect(),
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

            self.properties.clear();
            self.properties.extend(snapshot.properties.into_iter().map(PropertyItem::restore));
            self.priority_ranges = snapshot.priority_ranges;

            Ok(())
        } else {
            Err(snapshot)
        }
    }

    fn bind_all(&mut self) {
        debug_assert!(!self.is_bound);

        if !self.properties.is_empty() {
            // move the child to the innermost property child.

            debug_assert_eq!(
                self.properties[0].child.borrow().actual_type_id(),
                std::any::TypeId::of::<NilUiNode>(),
                "`{}` already has a child",
                self.properties[0].name
            );
            mem::swap(&mut *self.child.borrow_mut(), &mut *self.properties[0].child.borrow_mut());
            // save the new child address.
            self.child = self.properties[0].child.clone();

            // chain properties.

            for i in 0..self.properties.len() {
                let (a, b) = self.properties.split_at_mut(i + 1);
                if let (Some(inner), Some(outer)) = (a.last_mut(), b.first()) {
                    inner.set_parent(outer);
                }
            }

            // save the new outermost node address.
            self.node = self.properties[self.properties.len() - 1].node.clone();
        }

        self.is_bound = true;
    }

    fn unbind_all(&mut self) {
        debug_assert!(self.is_bound);

        if !self.properties.is_empty() {
            let child = mem::replace(&mut *self.child.borrow_mut(), NilUiNode.boxed());

            self.child = Rc::new(RefCell::new(child));

            for i in 0..self.properties.len() {
                let (a, b) = self.properties.split_at_mut(i + 1);
                if let (Some(inner), Some(outer)) = (a.last_mut(), b.first()) {
                    inner.unset_parent(outer);
                }
            }

            self.node = self.child.clone();
        }

        self.is_bound = false;
    }

    fn insert_impl(&mut self, priority: usize, properties_len: usize, properties: impl Iterator<Item = PropertyItem>) {
        let priority_range = self.priority_range(priority);

        if priority_range.is_empty() {
            // no properties of the priority, can just append or override.

            let properties: Vec<_> = properties.collect();

            if priority_range.start == self.properties.len() {
                // append
                self.properties.extend(properties);

                // update ranges.
                for p in &mut self.priority_ranges[priority..] {
                    *p = self.properties.len();
                }
            } else {
                // insert

                let insert_len = properties_len;

                let _rmv = self.properties.splice(priority_range, properties).next();
                debug_assert!(_rmv.is_none());

                // update ranges.
                for p in &mut self.priority_ranges[priority..] {
                    *p += insert_len;
                }
            }
        } else {
            // already has properties of the priority, compute overrides and resort.

            let mut new_properties: Vec<_> = properties.collect();

            // collect overrides
            let mut rmv_existing = vec![];
            let mut rmv_new = vec![];

            for (i, existing) in self.properties[priority_range.clone()].iter().enumerate() {
                if let Some(new_i) = new_properties.iter().position(|n| n.name == existing.name) {
                    let new = &new_properties[new_i];

                    if new.importance >= existing.importance {
                        rmv_existing.push(priority_range.start + i);
                    } else {
                        rmv_new.push(new_i);
                    }
                }
            }
            // remove overridden
            let remove_len = rmv_existing.len();
            for i in rmv_existing.into_iter().rev() {
                self.properties.remove(i);
            }
            for p in &mut self.priority_ranges[priority..] {
                *p -= remove_len;
            }
            let priority_range = self.priority_range(priority);

            // remove override attempts of less importance
            if !rmv_new.is_empty() {
                rmv_new.sort_unstable();

                for i in rmv_new.into_iter().rev() {
                    new_properties.remove(i);
                }

                if new_properties.is_empty() {
                    return;
                }
            }

            // insert new
            let insert_len = new_properties.len();
            let insert_i = priority_range.start;
            let _rmv = self.properties.splice(insert_i..insert_i, new_properties).next();
            debug_assert!(_rmv.is_none());
            for p in &mut self.priority_ranges[priority..] {
                *p += insert_len;
            }

            // resort priority.
            let priority_range = self.priority_range(priority);
            self.properties[priority_range].sort_by_key(|p| -p.priority_index);
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
    properties: Vec<PropertyItemSnapshot>,
    priority_ranges: [usize; DynPropPriority::LEN],
}
impl fmt::Debug for DynWidgetSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynWidgetSnapshot")
            .field("id", &self.id)
            .field("properties", &self.properties)
            .finish_non_exhaustive()
    }
}

struct PropertyItem {
    // property child, the `Rc` does not change, only the interior.
    child: Rc<RefCell<BoxedUiNode>>,
    // property node, the `Rc` changes, but it always points to the same node.
    node: Rc<RefCell<BoxedUiNode>>,
    // original `node`, preserved when parent is set, reused when unset.
    snapshot_node: Option<Rc<RefCell<BoxedUiNode>>>,

    name: &'static str,
    priority_index: i16,
    importance: DynPropImportance,
}
impl PropertyItem {
    fn new(property: DynProperty) -> Self {
        assert!(!property.node.is_inited());

        let (child, node) = property.node.into_parts();
        PropertyItem {
            child,
            node: Rc::new(RefCell::new(node)),
            snapshot_node: None,
            name: property.name,
            priority_index: property.priority_index,
            importance: property.importance,
        }
    }

    /// Set `self` as the child of `other`.
    fn set_parent(&mut self, other: &PropertyItem) {
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
    fn unset_parent(&mut self, other: &PropertyItem) {
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
impl fmt::Debug for PropertyItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItem")
            .field("name", &self.name)
            .field("priority_index", &self.priority_index)
            .field("importance", &self.importance)
            .finish_non_exhaustive()
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
    priority_index: i16,
    importance: DynPropImportance,
}
impl fmt::Debug for PropertyItemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItemSnapshot")
            .field("name", &self.name)
            .field("priority_index", &self.priority_index)
            .field("importance", &self.importance)
            .finish_non_exhaustive()
    }
}

impl PropertyItem {
    fn snapshot(&self) -> PropertyItemSnapshot {
        PropertyItemSnapshot {
            child: self.child.clone(),
            node: self.node.clone(),
            name: self.name,
            priority_index: self.priority_index,
            importance: self.importance,
        }
    }

    fn restore(snapshot: PropertyItemSnapshot) -> Self {
        PropertyItem {
            child: snapshot.child,
            node: snapshot.node,
            snapshot_node: None,
            name: snapshot.name,
            priority_index: snapshot.priority_index,
            importance: snapshot.importance,
        }
    }
}
