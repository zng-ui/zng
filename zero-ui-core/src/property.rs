//! Property helper types.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc, mem,
};

use linear_map::LinearMap;

use crate::{
    handler::WidgetHandler,
    inspector::SourceLocation,
    var::{AnyVar, AnyVarValue, BoxedVar, IntoVar, StateVar, Var, VarValue},
    AdoptiveNode, BoxedUiNode, BoxedWidget, UiNode, UiNodeList, Widget, WidgetList, NilUiNode, impl_from_and_into_var,
};

pub use crate::inspector::source_location;

/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// Kind of property input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InputKind {
    /// Input is `impl IntoVar<T>`, build value is `BoxedVar<T>`.
    Var,
    /// Input and build value is `StateVar`.
    StateVar,
    /// Input is `impl IntoValue<T>`, build value is `T`.
    Value,
    /// Input is `impl UiNode`, `impl Widget`, `impl WidgetHandler<A>`, ``, build value is `InputTakeout`.
    Takeout,
}

/// Represents a value that cannot be cloned and can only be used in one instance.
pub struct InputTakeout {
    val: Rc<RefCell<Option<Box<dyn Any>>>>,
}
impl InputTakeout {
    fn new(val: Box<dyn Any>) -> Self {
        InputTakeout {
            val: Rc::new(RefCell::new(Some(val))),
        }
    }

    /// New from `impl UiNode` input.
    pub fn new_ui_node(node: impl UiNode) -> Self {
        Self::new(Box::new(node.boxed()))
    }

    /// New from `impl Widget` input.
    pub fn new_widget(wgt: impl Widget) -> Self {
        Self::new(Box::new(wgt.boxed_wgt()))
    }

    /// New from `impl WidgetHandler<A>` input.
    pub fn new_widget_handler<A>(handler: impl WidgetHandler<A>) -> Self
    where
        A: Clone + 'static,
    {
        Self::new(Box::new(handler.boxed()))
    }

    /// New from `impl UiNodeList` input.
    pub fn new_ui_node_list(list: impl UiNodeList) -> Self {
        todo!("Boxed version")
    }

    /// New from `impl WidgetList` input.
    pub fn new_widget_list(list: impl WidgetList) -> Self {
        todo!("Boxed version")
    }

    /// If the args was not spend yet.
    pub fn is_available(&self) -> bool {
        self.val.borrow().is_some()
    }

    fn take<T: Any>(&self) -> T {
        *self
            .val
            .borrow_mut()
            .take()
            .expect("input takeout already used")
            .downcast::<T>()
            .expect("input takeout was of the requested type")
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_ui_node(&self) -> BoxedUiNode {
        self.take()
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_widget(&self) -> BoxedWidget {
        self.take()
    }

    /// Takes the value for an `impl WidgetHandler<A>` input.
    pub fn take_widget_handler<A: Clone + 'static>(&self) -> Box<dyn WidgetHandler<A>> {
        self.take()
    }

    // UiNodeList, WidgetList, don't have a boxed version.
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

    /// Gets a [`InputKind::Value`].
    fn value(&self, i: usize) -> &dyn AnyVarValue {
        panic_input(&self.property(), i, InputKind::Value)
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

    /// Gets a [`InputKind::Takeout`].
    fn takeout(&self, i: usize) -> &InputTakeout {
        panic_input(&self.property(), i, InputKind::Takeout)
    }

    /// Create a property instance with args clone or taken.
    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode;
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
pub fn read_var<T: VarValue>(args: &dyn PropertyArgs, i: usize) -> BoxedVar<T> {
    args.var(i)
        .as_any()
        .downcast_ref::<BoxedVar<T>>()
        .expect("expected different arg type")
        .clone()
}

#[doc(hidden)]
pub fn read_value<T: VarValue>(args: &dyn PropertyArgs, i: usize) -> BoxedVar<T> {
    args.value(i)
        .as_any()
        .downcast_ref::<T>()
        .expect("expected diffent arg type")
        .clone()
        .into_var()
        .boxed()
}

/*

 WIDGET

*/

enum WidgetItem {
    Instrinsic(AdoptiveNode<BoxedUiNode>),
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
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId {
    /// The [`PropertyInfo::unique_id`].
    pub unique_id: TypeId,
    /// The [`PropertyInstInfo::name`].
    pub name: &'static str,
}

/// Input var read in a `when` condition expression.
pub struct WhenInput {
    /// Property name.
    pub property_name: &'static str,
    /// Id of the named property when the expression was created.
    pub property_id: TypeId,
    /// Property input name.
    pub member_name: &'static str,
    /// Property input index.
    pub member_i: usize,
    /// Property input value.
    pub var: Box<dyn AnyVar>,
}

/// Represents a `when` block in a widget.
pub struct When {
    /// Properties referenced in the when condition expression.
    /// 
    /// They are type erased `RcVar<T>` instances and can be rebound, other variable references (`*#{var}`) are imbedded in
    /// the build expression and cannot be modified.
    pub inputs: Box<[WhenInput]>,

    /// Output of the when expression.
    pub state: BoxedVar<bool>,

    /// Properties assigned in the when block, in the build widget they are joined with the default value and assigns
    /// from other when blocks into a single property instance set to `when_var!` inputs.
    pub assigns: Box<[Box<dyn PropertyArgs>]>,
}

/// Widget instance builder.
#[derive(Default)]
pub struct WidgetBuilder {
    child: Option<BoxedUiNode>,
    items: Vec<(Priority, WidgetItem)>,
    unset: LinearMap<PropertyId, Importance>,
    whens: Vec<(Importance, When)>,
}
impl WidgetBuilder {
    /// Insert intrinsic node, that is a core functionality node of the widget that cannot be overridden.
    pub fn insert_intrinsic(&mut self, priority: Priority, node: AdoptiveNode<BoxedUiNode>) {
        self.items.push((priority, WidgetItem::Instrinsic(node)));
    }

    /// Insert/override a property.
    pub fn insert_property(&mut self, importance: Importance, args: Box<dyn PropertyArgs>) {
        let property_id = args.id();
        let info = args.property();
        if let Some(i) = self.property_position(property_id) {
            match &self.items[i].1 {
                WidgetItem::Property { importance: imp, .. } => {
                    if *imp <= importance {
                        self.items[i] = (
                            info.priority,
                            WidgetItem::Property {
                                importance,
                                args,
                            },
                        )
                    }
                    // else already overridden
                }
                WidgetItem::Instrinsic(_) => unreachable!(),
            }
        } else {
            if let Some(imp) = self.unset.get(&property_id) {
                if *imp >= importance {
                    return; // unset overrides.
                }
            }
            self.items.push((
                info.priority,
                WidgetItem::Property {
                    importance,
                    args,
                },
            ))
        }
    }

    fn property_position(&self, property_id: PropertyId) -> Option<usize> {
        self.items.iter().position(|(_, item)| match item {
            WidgetItem::Property { args, .. } => args.id() == property_id,
            WidgetItem::Instrinsic(_) => false,
        })
    }

    /// Insert a `name = unset!;` property.
    pub fn insert_unset(&mut self, importance: Importance, property_id: PropertyId) {
        match self.unset.entry(property_id) {
            linear_map::Entry::Occupied(mut e) => {
                let i = e.get_mut();
                *i = (*i).max(importance);
            }
            linear_map::Entry::Vacant(e) => {
                let mut rmv = None;
                for (i, (_, item)) in self.items.iter().enumerate() {
                    match item {
                        WidgetItem::Property { importance: imp, args, .. } => {
                            if args.id() == property_id {
                                if *imp <= importance {
                                    rmv = Some(i);
                                    break;
                                } else {
                                    return;
                                }
                            }
                        }
                        WidgetItem::Instrinsic(_) => {}
                    }
                }

                e.insert(importance);
                if let Some(i) = rmv {
                    self.items.remove(i);
                }
            }
        }
    }

    /// Remove the property that matches the `property_id!(..)`.
    pub fn remove_property(&mut self, property_id: PropertyId) -> Option<(Importance, Box<dyn PropertyArgs>)> {
        if let Some(i) = self.property_position(property_id) {
            match self.items.remove(i).1 {
                // can't be swap remove for ordering of equal priority.
                WidgetItem::Property { importance, args, .. } => Some((importance, args)),
                WidgetItem::Instrinsic(_) => unreachable!(),
            }
        } else {
            None
        }

        // this method is used to remove "captures", that means we need to remove `when` assigns and a clone of the conditions too?
    }

    /// Insert a `when` block.
    pub fn insert_when(&mut self, importance: Importance, when: When) {
        self.whens.push((importance, when))
    }

    /// If a child not is already set in the builder.
    ///
    /// If build without child the [`NilUiNode`] is used as the innermost node.
    pub fn has_child(&self) -> bool {
        self.child.is_some()
    }

    /// Set/replace the inner most node of the widget.
    pub fn set_child(&mut self, node: BoxedUiNode) -> Option<BoxedUiNode> {
        self.child.replace(node)
    }

    fn sort_items(&mut self) {
        self.items.sort_by(|(a_pri, a_item), (b_pri, b_item)| match a_pri.cmp(b_pri) {
            std::cmp::Ordering::Equal => match (a_item, b_item) {
                // INSTANCE importance is innermost of DEFAULT.
                (WidgetItem::Property { importance: a_imp, .. }, WidgetItem::Property { importance: b_imp, .. }) => a_imp.cmp(b_imp),
                // Intrinsic is outermost of priority items.
                (WidgetItem::Property { .. }, WidgetItem::Instrinsic(_)) => std::cmp::Ordering::Greater,
                (WidgetItem::Instrinsic(_), WidgetItem::Property { .. }) => std::cmp::Ordering::Less,
                (WidgetItem::Instrinsic(_), WidgetItem::Instrinsic(_)) => std::cmp::Ordering::Equal,
            },
            ord => ord,
        });

        self.whens.sort_by_key(|(imp, _)| *imp);
    }

    /// Instantiate and link all property and intrinsic nodes, returns the outermost node.
    pub fn build(mut self) -> BoxedUiNode {
        self.sort_items();

        let mut child = self.child.unwrap_or_else(|| NilUiNode.boxed());

        for (_, item) in self.items {
            match item {
                WidgetItem::Instrinsic(node) => {
                    let (c, n) = node.into_parts();
                    *c.borrow_mut() = mem::replace(&mut child, n);
                },
                WidgetItem::Property { args, .. } => {
                    child = args.instantiate(child);
                },
            }
        }

        child
    }

    /// Build to a node type that can still be modified to an extent.
    pub fn build_dyn(mut self) -> DynUiNode {
        self.sort_items();

        todo!()
    }
}

struct DynUiNodeItem {
    child: Rc<RefCell<BoxedUiNode>>,
    node: Rc<RefCell<BoxedUiNode>>,
    when: Option<()>,
}

/// Represents a built [`WidgetBuilder`] node that can still be modified to an extent when deinited.
pub struct DynUiNode {
    is_inited: bool,
    is_linked: bool,
}
impl DynUiNode {
    /// If the node is inited in a context, if `true` the node cannot be restored into a builder.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    fn delink(&mut self) {
        assert!(!self.is_inited);
        
        if !mem::take(&mut self.is_linked) {
            return;
        }

        todo!()
    }

    fn link(&mut self) {
        assert!(!self.is_inited);

        if mem::replace(&mut self.is_linked, true) {
            return;
        }

        todo!()
    }

    /// Take a snapshot that can be used to restore the node to a pre-injection state.
    pub fn snapshot(&self) -> DynUiNodeSnapshot{
        assert!(!self.is_inited);
        todo!()
    }

    /// Restore the node properties.
    pub fn restore(&mut self, snapshot: DynUiNodeSnapshot) {
        self.delink();
        todo!()
    }

    /// Insert/override nodes from `other` onto `self`.
    /// 
    /// Intrinsic nodes are moved in, property nodes of the same name, id and >= importance replace self, when conditions and assigns
    /// are rebuild.
    pub fn inject(&mut self, other: DynUiNode) {
        self.delink();
        todo!()
    }
}

pub struct DynUiNodeSnapshot {

}

mod expand{
    use super::*;

    #[zero_ui_proc_macros::property2(context, default(true, None))]
    pub fn boo<T: VarValue>(child: impl UiNode, boo: impl IntoVar<bool>, too: impl IntoVar<Option<T>>) -> impl UiNode {
        let _ = (boo, too);
        tracing::error!("boo must be captured by the widget");
        child
    }

    #[zero_ui_proc_macros::widget2($crate::property::expand::bar)]
    pub mod bar {
        use super::*;

        properties! {
            foo;
        }

        fn build(_: WidgetBuilder) -> NilUiNode {
            NilUiNode
        }
    }
}
