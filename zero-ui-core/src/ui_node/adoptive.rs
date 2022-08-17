use std::fmt;
use std::{cell::RefCell, mem, rc::Rc};

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

/// Represents an widget property use in dynamic initialization.
///
/// See the [`#[widget]`] documentation for more details.
///
/// [`#[widget]`]: macro@crate::widget
pub struct DynProperty {
    /// The property node, setup as an adoptive node that allows swapping the child node.
    pub node: AdoptiveNode<BoxedUiNode>,

    /// Name of the property that was set.
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

    /// Who assigned the property.
    pub importance: DynPropImportance,

    /// If this property is read in `when` conditions.
    ///
    /// If this is `true` removing the property instance will cause other properties set
    /// in the `when` condition to run the "when false" behavior even if a property with
    /// the same name is re-inserted.
    pub is_when_condition: bool, // TODO turns this into some sort of reference counter
                                 /*
                                 /// Unique ID of the property *type*.
                                 pub id: PropertyId,
                                 */
}
impl DynProperty {
    #[doc(hidden)]
    pub fn start_v1() -> (AdoptiveChildNode, DynPropertyBuilderV1) {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        (ad_child, DynPropertyBuilderV1 { child })
    }
}
impl fmt::Debug for DynProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynProperty")
            .field("name", &self.name)
            .field("importance", &self.importance)
            .field("is_when_condition", &self.is_when_condition)
            .finish_non_exhaustive()
    }
}

#[doc(hidden)]
pub struct DynPropertyBuilderV1 {
    child: Rc<RefCell<BoxedUiNode>>,
}
impl DynPropertyBuilderV1 {
    #[doc(hidden)]
    pub fn build(self, property: impl UiNode, name: &'static str, user_assigned: bool, is_when_condition: bool) -> DynProperty {
        let node = AdoptiveNode {
            child: self.child,
            node: property.boxed(),
            is_inited: false,
        };

        DynProperty {
            node,
            name,
            importance: if user_assigned {
                DynPropImportance::INSTANCE
            } else {
                DynPropImportance::WIDGET
            },
            is_when_condition,
        }
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

/*
unique_id_64! {
    /// Unique ID of a `#[property]` declaration.
    pub struct PropertyId;
}
*/

#[derive(Clone)]
struct PropertyItem {
    // property child, the `Rc` does not change, only the interior.
    child: Rc<RefCell<BoxedUiNode>>,
    // property node, the `Rc` changes, but it always points to the same node.
    node: Rc<RefCell<BoxedUiNode>>,

    name: &'static str,
    importance: DynPropImportance,
    is_when_condition: bool,
}
impl PropertyItem {
    fn new(property: DynProperty) -> Self {
        assert!(!property.node.is_inited());

        let (child, node) = property.node.into_parts();
        PropertyItem {
            child,
            node: Rc::new(RefCell::new(node)),
            name: property.name,
            importance: property.importance,
            is_when_condition: property.is_when_condition,
        }
    }

    /// Set `self` as the child of `other`.
    fn set_parent(&mut self, other: &PropertyItem) {
        debug_assert!(
            {
                let unset_placeholder = mem::replace(&mut *other.child.borrow_mut(), NilUiNode.boxed());
                unset_placeholder.downcast_unbox::<NilUiNode>().is_ok()
            },
            "`{}` already has a child",
            other.name
        );

        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
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

        self.node = Rc::new(RefCell::new(NilUiNode.boxed()));
        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
    }
}
impl fmt::Debug for PropertyItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItem")
            .field("name", &self.name)
            .field("importance", &self.importance)
            .field("is_when_condition", &self.is_when_condition)
            .finish_non_exhaustive()
    }
}

/// Property priority of dynamic properties.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynPropPriority {
    ChildLayout = 0,
    ChildContext,
    Fill,
    Border,
    Size,
    Layout,
    Event,
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

unique_id_32! {
    struct DynPropertiesId;
}
impl fmt::Debug for DynPropertiesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DynPropertiesId").field(&self.sequential()).finish()
    }
}

/// Represents an editable chain of dynamic properties.
///
/// This struct is a composite [`AdoptiveNode`].
pub struct DynProperties {
    id: DynPropertiesId,

    // innermost child.
    //
    // The Rc changes to the `child` of the innermost property when bound and a new Rc when unbound,
    // the interior only changes when `replace_child` is used.
    child: Rc<RefCell<BoxedUiNode>>,

    // outermost node.
    //
    // The Rc changes to the `node` of the outermost property, the interior is not modified from here.
    node: Rc<RefCell<BoxedUiNode>>,

    is_inited: bool,
    is_bound: bool,

    properties: Vec<PropertyItem>,
    // exclusive end of each priority range in `properties`
    priority_ranges: [usize; DynPropPriority::LEN],
}
impl fmt::Debug for DynProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        #[allow(unused)] // used for debug print
        struct DebugProperties<'a> {
            priority: DynPropPriority,
            entries: &'a [PropertyItem],
        }
        let mut properties = vec![];
        let mut s = 0;
        for (i, &e) in self.priority_ranges.iter().enumerate() {
            if s < e {
                properties.push(DebugProperties {
                    priority: DynPropPriority::from_index(i).unwrap(),
                    entries: &self.properties[s..e],
                });
            }
            s = e;
        }

        f.debug_struct("DynProperties")
            .field("id", &self.id)
            .field("properties", &properties)
            .field("is_inited", &self.is_inited)
            .field("is_bound", &self.is_inited)
            .finish_non_exhaustive()
    }
}
impl Default for DynProperties {
    fn default() -> Self {
        Self::new(DynPropPriority::ChildLayout, vec![])
    }
}
impl DynProperties {
    /// New from properties of a priority.
    ///
    /// Panics if `properties` is inited.
    pub fn new(priority: DynPropPriority, properties: Vec<DynProperty>) -> DynProperties {
        Self::new_impl(priority, properties.into_iter().map(PropertyItem::new).collect())
    }

    fn new_impl(priority: DynPropPriority, properties: Vec<PropertyItem>) -> DynProperties {
        let node = Rc::new(RefCell::new(NilUiNode.boxed()));
        if properties.is_empty() {
            DynProperties {
                id: DynPropertiesId::new_unique(),
                child: node.clone(),
                node,
                is_inited: false,
                is_bound: false,
                properties: vec![],
                priority_ranges: [0; DynPropPriority::LEN],
            }
        } else {
            let mut priority_ranges = [0; DynPropPriority::LEN];
            for e in &mut priority_ranges[(priority as usize)..DynPropPriority::LEN] {
                *e = properties.len();
            }

            DynProperties {
                id: DynPropertiesId::new_unique(),
                child: node.clone(),
                node,
                is_inited: false,
                is_bound: false,
                properties,
                priority_ranges,
            }
        }
    }

    /// Returns `true` if this node is initialized in a UI tree.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Returns `true` if this collection contains no properties.
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Replaces the inner child node, panics if the node is inited.
    ///
    /// Returns the previous child, the initial child is a [`NilUiNode`].
    pub fn replace_child(&mut self, new_child: impl UiNode) -> BoxedUiNode {
        assert!(!self.is_inited);
        mem::replace(&mut *self.child.borrow_mut(), new_child.boxed())
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

    fn bind_all(&mut self) {
        debug_assert!(!self.is_bound);

        if !self.properties.is_empty() {
            mem::swap(&mut *self.child.borrow_mut(), &mut *self.properties[0].child.borrow_mut());
            self.child = self.properties[0].child.clone();

            for i in 0..self.properties.len() {
                let (a, b) = self.properties.split_at_mut(i + 1);
                if let (Some(inner), Some(outer)) = (a.last_mut(), b.first()) {
                    inner.set_parent(outer);
                }
            }

            self.node = self.properties[self.properties.len() - 1].node.clone();
        }

        self.is_bound = true;
    }

    /// Insert `properties` in the chain, overrides properties with the same name, priority and with less or equal importance.
    /// less than or equal to `override_level`.
    ///
    /// Panics if `self` or `properties` are inited.
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

    /// Insert `properties` in the chain, overrides properties with the same name, priority and with less or equal importance.
    ///
    /// Panics if `self` or `properties` are inited.
    pub fn insert_all(&mut self, properties: DynProperties) {
        let mut other = properties;

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

    fn insert_impl(&mut self, priority: usize, properties_len: usize, properties: impl Iterator<Item = PropertyItem>) {
        let pe = priority as usize;
        let ps = pe.saturating_sub(1);
        let priority = self.priority_ranges[ps]..self.priority_ranges[pe];

        if priority.is_empty() {
            // no properties of the priority, can just append or override.

            if priority.start == self.properties.len() {
                // append
                self.properties.extend(properties);

                // update ranges.
                for p in &mut self.priority_ranges[pe..] {
                    *p = self.properties.len();
                }
            } else {
                // insert

                let insert_len = properties_len;

                let _rmv = self.properties.splice(priority, properties).next();
                debug_assert!(_rmv.is_none());

                // update ranges.
                for p in &mut self.priority_ranges[pe..] {
                    *p += insert_len;
                }
            }
        } else {
            // already has properties of the priority, compute overrides.

            let mut properties: Vec<_> = properties.collect();

            // collect overrides
            let mut removes = vec![];
            let mut insert_removes = vec![];

            for (i, p) in self.properties[priority.clone()].iter().enumerate() {
                if p.is_when_condition {
                    continue; // never remove when condition properties
                }

                if let Some(insert_i) = properties.iter().position(|n| n.name == p.name) {
                    let same_name = &properties[insert_i];
                    if same_name.importance <= p.importance {
                        removes.push(priority.start + i);
                    } else {
                        insert_removes.push(insert_i);
                    }
                }
            }
            // remove overrides
            let remove_len = removes.len();
            for i in removes.into_iter().rev() {
                self.properties.remove(i);
            }
            if !insert_removes.is_empty() {
                insert_removes.sort();

                for i in insert_removes.into_iter().rev() {
                    properties.remove(i);
                }

                if properties.is_empty() {
                    return;
                }
            }

            // insert new
            let insert_len = properties.len();

            let insert_i = priority.end - remove_len;
            let _rmv = self.properties.splice(insert_i..insert_i, properties).next();
            debug_assert!(_rmv.is_none());

            // update ranges.
            for p in &mut self.priority_ranges[pe..] {
                *p -= remove_len;
                *p += insert_len;
            }
        }
    }

    /// Create an snapshot of the current properties.
    ///
    /// The snapshot can be used to [`restore`] the properties to a state before it was overridden by an insert.
    ///
    /// [`restore`]: DynProperties::restore
    pub fn snapshot(&self) -> DynPropertiesSnapshot {
        DynPropertiesSnapshot {
            id: self.id,
            properties: self.properties.clone(),
            priority_ranges: self.priority_ranges,
        }
    }

    /// Restores the properties to the snapshot, if it was taken from the same properties.
    ///
    /// Panics if the node is inited.
    pub fn restore(&mut self, snapshot: DynPropertiesSnapshot) -> Result<(), DynPropertiesSnapshot> {
        assert!(!self.is_inited);

        if self.id == snapshot.id {
            if self.is_bound {
                self.unbind_all();
            }

            self.properties = snapshot.properties;
            self.priority_ranges = snapshot.priority_ranges;

            Ok(())
        } else {
            Err(snapshot)
        }
    }

    /// Split the properties in a separate collection for each property priority.
    pub fn split_priority(mut self) -> [DynProperties; DynPropPriority::LEN] {
        assert!(!self.is_inited);

        if self.is_bound {
            self.unbind_all();
        }

        let mut properties = self.properties.into_iter();

        let mut r = Vec::with_capacity(DynPropPriority::LEN);
        let mut start = 0;
        for (i, end) in self.priority_ranges.iter().enumerate() {
            r.push(DynProperties::new_impl(
                DynPropPriority::from_index(i).unwrap(),
                properties.by_ref().take(end - start).collect(),
            ));
            start = *end;
        }

        r.try_into().unwrap()
    }
}
#[impl_ui_node(
    delegate = self.node.borrow(),
    delegate_mut = self.node.borrow_mut(),
)]
impl UiNode for DynProperties {
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

/// Represents a snapshot of a [`DynProperties`] value.
///
/// The snapshot can be used to [`restore`] the properties to a state before it was overridden by an insert.
///
/// [`restore`]: DynProperties::restore
#[derive(Debug)]
pub struct DynPropertiesSnapshot {
    id: DynPropertiesId,
    properties: Vec<PropertyItem>,
    priority_ranges: [usize; DynPropPriority::LEN],
}
