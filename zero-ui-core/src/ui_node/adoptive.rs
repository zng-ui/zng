use std::fmt;
use std::{cell::RefCell, mem, rc::Rc};

use crate::crate_util::TakeByRefIterExt;
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
    /// path::to::foo = true;
    /// bar as foo = true;
    /// foo = true;
    /// ```
    pub name: &'static str,

    /// Who assigned the property.
    pub source: DynPropertySource,

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
            .field("source", &self.source)
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
            source: if user_assigned {
                DynPropertySource::Instance
            } else {
                DynPropertySource::Widget
            },
            is_when_condition,
        }
    }
}

#[doc(hidden)]
pub type DynPropertySourceV1 = DynPropertySource;

/// Represents who assigned the property that caused the [`DynProperty`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynPropertySource {
    /// Property assigned in the widget declaration and not overwritten in the instance.
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
    Widget,
    /// Property assigned in the widget instance.
    ///
    /// ```
    /// # foo! { ($($tt:tt)*) => { } }
    /// foo! {
    ///     bar = true;// assign in the instance.
    /// }
    /// ```
    Instance,
}
impl std::cmp::PartialOrd for DynPropertySource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for DynPropertySource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            std::cmp::Ordering::Equal
        } else if let DynPropertySource::Instance = self {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
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
    source: DynPropertySource,
    is_when_condition: bool,
}
impl PropertyItem {
    fn new(property: DynProperty) -> Self {
        let (child, node) = property.node.into_parts();
        PropertyItem {
            child,
            node: Rc::new(RefCell::new(node)),
            name: property.name,
            source: property.source,
            is_when_condition: property.is_when_condition,
        }
    }

    /// Set `self` as the child of `other`.
    fn set_parent(&mut self, other: &PropertyItem) {
        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
        self.node = other.node.clone();
    }

    /// Unset `self` as the child of `other`.
    fn unset_parent(&mut self, other: &PropertyItem) {
        debug_assert!(Rc::ptr_eq(&self.node, &other.child));
        self.node = Rc::new(RefCell::new(NilUiNode.boxed()));
        mem::swap(&mut *other.child.borrow_mut(), &mut *self.node.borrow_mut());
    }
}
impl fmt::Debug for PropertyItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyItem")
            .field("name", &self.name)
            .field("source", &self.source)
            .field("is_when_condition", &self.is_when_condition)
            .finish_non_exhaustive()
    }
}

/// Property priority of dynamic properties.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynPropertyPriority {
    ChildLayout = 0,
    ChildContext,
    Fill,
    Border,
    Size,
    Layout,
    Event,
    Context,
}
impl DynPropertyPriority {
    /// Number of priority items.
    pub const LEN: usize = DynPropertyPriority::Context as usize + 1;
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
    child: Rc<RefCell<BoxedUiNode>>,
    // outermost node.
    node: Rc<RefCell<BoxedUiNode>>,

    is_inited: bool,
    is_bound: bool,

    properties: Vec<PropertyItem>,
    // exclusive end of each priority range in `properties`
    priority_ranges: [usize; DynPropertyPriority::LEN],
}
impl fmt::Debug for DynProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynProperties")
            .field("id", &self.id)
            .field("properties", &self.properties)
            .field("is_inited", &self.is_inited)
            .finish_non_exhaustive()
    }
}
impl Default for DynProperties {
    fn default() -> Self {
        Self::new(DynPropertyPriority::ChildLayout, vec![])
    }
}
impl DynProperties {
    /// New from properties of a priority.
    ///
    /// Panics if `properties` is inited.
    pub fn new(priority: DynPropertyPriority, properties: Vec<DynProperty>) -> DynProperties {
        let node = Rc::new(RefCell::new(NilUiNode.boxed()));
        if properties.is_empty() {
            DynProperties {
                id: DynPropertiesId::new_unique(),
                child: node.clone(),
                node,
                is_inited: false,
                is_bound: false,
                properties: vec![],
                priority_ranges: [0; DynPropertyPriority::LEN],
            }
        } else {
            assert!(!properties[0].node.is_inited());

            let mut priority_ranges = [0; DynPropertyPriority::LEN];
            for e in &mut priority_ranges[(priority as usize)..DynPropertyPriority::LEN] {
                *e = properties.len();
            }

            DynProperties {
                id: DynPropertiesId::new_unique(),
                child: node.clone(),
                node,
                is_inited: false,
                is_bound: false,
                properties: properties.into_iter().map(PropertyItem::new).collect(),
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
            // TODO !!: child is lost?
            self.child = Rc::new(RefCell::new(NilUiNode.boxed()));
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
        debug_assert!(self.is_bound);

        if !self.properties.is_empty() {
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

    /// Insert `properties` in the chain, overrides properties with the same name, priority and with source
    /// less than or equal to `override_level`.
    ///
    /// Panics if `self` or `properties` are inited.
    pub fn insert(&mut self, priority: DynPropertyPriority, properties: Vec<DynProperty>, override_level: DynPropertySource) {
        assert!(!self.is_inited);

        if properties.is_empty() {
            return;
        }

        assert!(!properties[0].node.is_inited());

        if self.is_bound {
            // will rebind next init.
            self.unbind_all();
        }

        self.insert_impl(
            priority as usize,
            properties.len(),
            properties.into_iter().map(PropertyItem::new),
            override_level,
        );
    }

    /// Insert `properties` in the chain, overrides properties with the same name, priority and with source
    /// less than or equal to `override_level`.
    ///
    /// Panics if `self` or `properties` are inited.
    pub fn insert_all(&mut self, properties: DynProperties, override_level: DynPropertySource) {
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
        for p in 0..DynPropertyPriority::LEN {
            let e = other.priority_ranges[p];

            let n = e - s;
            if n > 0 {
                self.insert_impl(p, n, properties.take_by_ref(n), override_level);
            }

            s = e;
        }
    }

    fn insert_impl(
        &mut self,
        priority: usize,
        properties_len: usize,
        properties: impl Iterator<Item = PropertyItem>,
        override_level: DynPropertySource,
    ) {
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

            let properties: Vec<_> = properties.collect();

            // collect overrides
            let mut removes = vec![];
            for (i, p) in self.properties[priority.clone()].iter().enumerate() {
                if p.is_when_condition {
                    continue; // never remove when condition properties
                }

                if let Some(same_name) = properties.iter().find(|n| n.name == p.name) {
                    if same_name.source <= override_level {
                        removes.push(priority.start + i);
                    }
                }
            }
            // remove overrides
            let remove_len = removes.len();
            for i in removes.into_iter().rev() {
                self.properties.remove(i);
            }

            // insert new
            let insert_len = properties_len;

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
    pub fn split_priority(self) -> [DynProperties; DynPropertyPriority::LEN] {
        todo!()
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
    priority_ranges: [usize; DynPropertyPriority::LEN],
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
