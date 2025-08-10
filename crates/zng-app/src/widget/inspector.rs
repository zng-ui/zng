//! Helper types for inspecting an UI tree.
//!
//! When compiled with the `"inspector"` feature all widget instances are instrumented with inspection node
//! that shares a clone of the [`WidgetBuilder`] in the [`WidgetInfo`].

#[cfg(feature = "inspector")]
mod inspector_only {
    use std::sync::Arc;

    use crate::widget::{
        builder::{InputKind, PropertyId},
        node::{UiNode, UiNodeOp, match_node},
    };

    pub(crate) fn insert_widget_builder_info(child: UiNode, info: super::InspectorInfo) -> UiNode {
        let insp_info = Arc::new(info);
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op {
                info.set_meta(*super::INSPECTOR_INFO_ID, insp_info.clone());
            }
        })
    }

    pub(crate) fn actualize_var_info(child: UiNode, property: PropertyId) -> UiNode {
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op {
                info.with_meta(|mut m| {
                    let info = m.get_mut(*super::INSPECTOR_INFO_ID).unwrap();
                    let prop = info.properties().find(|p| p.0.id() == property).unwrap().0;
                    for (i, input) in prop.property().inputs.iter().enumerate() {
                        if matches!(input.kind, InputKind::Var) {
                            let var = prop.var(i);
                            if var.capabilities().is_contextual() {
                                let var = var.current_context();
                                info.actual_vars.insert(property, i, var);
                            }
                        }
                    }
                });
            }
        })
    }
}
#[cfg(feature = "inspector")]
pub(crate) use inspector_only::*;

use parking_lot::RwLock;
use zng_state_map::StateId;
use zng_txt::Txt;
use zng_unique_id::static_id;
use zng_var::{AnyVar, Var, VarValue};

use std::{any::TypeId, collections::HashMap, sync::Arc};

use super::{
    builder::{InputKind, NestGroup, PropertyArgs, PropertyId, WidgetBuilder, WidgetType},
    info::WidgetInfo,
};

static_id! {
    pub(super) static ref INSPECTOR_INFO_ID: StateId<Arc<InspectorInfo>>;
}

/// Widget instance item.
///
/// See [`InspectorInfo::items`].
#[derive(Debug)]
pub enum InstanceItem {
    /// Property instance.
    Property {
        /// Final property args.
        ///
        /// Unlike the same property in the builder, these args are affected by `when` assigns.
        args: Box<dyn PropertyArgs>,
        /// If the property was captured by the widget.
        ///
        /// If this is `true` the property is not instantiated in the widget, but its args are used in intrinsic nodes.
        captured: bool,
    },
    /// Marks an intrinsic node instance inserted by the widget.
    Intrinsic {
        /// Intrinsic node nest group.
        group: NestGroup,
        /// Name given to this intrinsic by the widget.
        name: &'static str,
    },
}

/// Inspected contextual variables actualized at the moment of info build.
#[derive(Default)]
pub struct InspectorActualVars(RwLock<HashMap<(PropertyId, usize), AnyVar>>);
impl InspectorActualVars {
    /// Get the actualized property var, if at the moment of info build it was contextual (and existed).
    pub fn get(&self, property: PropertyId, member: usize) -> Option<AnyVar> {
        self.0.read().get(&(property, member)).cloned()
    }

    /// Get and downcast.
    pub fn downcast<T: VarValue>(&self, property: PropertyId, member: usize) -> Option<Var<T>> {
        self.get(property, member)?.downcast::<T>().ok()
    }

    /// Get and map debug.
    pub fn get_debug(&self, property: PropertyId, member: usize) -> Option<Var<Txt>> {
        let b = self.get(property, member)?;
        Some(b.map_debug(false))
    }

    #[cfg(feature = "inspector")]
    fn insert(&self, property: PropertyId, member: usize, var: AnyVar) {
        self.0.write().insert((property, member), var);
    }
}

/// Widget instance inspector info.
///
/// Can be accessed and queried using [`WidgetInfoInspectorExt`].
#[non_exhaustive]
pub struct InspectorInfo {
    /// Builder that was used to instantiate the widget.
    pub builder: WidgetBuilder,

    /// Final instance items.
    pub items: Box<[InstanceItem]>,

    /// Inspected contextual variables actualized at the moment of info build.
    pub actual_vars: InspectorActualVars,
}

impl std::fmt::Debug for InspectorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectorInfo")
            .field("builder", &self.builder)
            .field("items", &self.items)
            .field("actual_vars", &self.actual_vars.0.read().keys())
            .finish_non_exhaustive()
    }
}
impl InspectorInfo {
    /// Iterate over property items and if they are captured.
    pub fn properties(&self) -> impl Iterator<Item = (&dyn PropertyArgs, bool)> {
        self.items.iter().filter_map(|it| match it {
            InstanceItem::Property { args, captured } => Some((&**args, *captured)),
            InstanceItem::Intrinsic { .. } => None,
        })
    }
}

/// Extensions methods for [`WidgetInfo`].
pub trait WidgetInfoInspectorExt {
    /// Reference the builder that was used to generate the widget, the builder generated items and the widget info context.
    ///
    /// Returns `None` if not build with the `"inspector"` feature, or if the widget instance was not created using
    /// the standard builder.
    fn inspector_info(&self) -> Option<Arc<InspectorInfo>>;

    /// If a [`inspector_info`] is defined for the widget.
    ///
    /// [`inspector_info`]: Self::inspector_info
    fn can_inspect(&self) -> bool;

    /// Returns the first child that matches.
    fn inspect_child<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Returns the first descendant that matches.
    ///
    /// # Examples
    ///
    /// Example searches for a "button" descendant, using a string search that matches the end of the [`WidgetType::path`] and
    /// an exact widget mod that matches the [`WidgetType::type_id`].
    ///
    /// ```
    /// # use zng_app::widget::{inspector::*, info::*, builder::*};
    /// # fn main() { }
    /// mod widgets {
    ///     use zng_app::widget::*;
    ///     
    ///     #[widget($crate::widgets::Button)]
    ///     pub struct Button(base::WidgetBase);
    /// }
    ///
    /// # fn demo(info: WidgetInfo) {
    /// let fuzzy = info.inspect_descendant("button");
    /// let exact = info.inspect_descendant(std::any::TypeId::of::<crate::widgets::Button>());
    /// # }
    /// ```
    fn inspect_descendant<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Returns the first ancestor that matches.
    fn inspect_ancestor<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Search for a property set on the widget.
    ///
    /// # Examples
    ///
    /// Search for a property by name, and then downcast its value.
    ///
    /// ```
    /// # use zng_app::widget::{info::*, inspector::*};
    /// fn inspect_foo(info: WidgetInfo) -> Option<bool> {
    ///     info.inspect_property("foo")?.value(0).downcast_ref().copied()
    /// }
    /// ```
    fn inspect_property<P: InspectPropertyPattern>(&self, pattern: P) -> Option<&dyn PropertyArgs>;

    /// Gets the parent property that has this widget as an input.
    ///
    /// Returns `Some((PropertyId, member_index))`.
    fn parent_property(&self) -> Option<(PropertyId, usize)>;
}
impl WidgetInfoInspectorExt for WidgetInfo {
    fn inspector_info(&self) -> Option<Arc<InspectorInfo>> {
        self.meta().get_clone(*INSPECTOR_INFO_ID)
    }

    fn can_inspect(&self) -> bool {
        self.meta().contains(*INSPECTOR_INFO_ID)
    }

    fn inspect_child<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.children().find(|c| match c.meta().get(*INSPECTOR_INFO_ID) {
            Some(wgt) => pattern.matches(wgt),
            None => false,
        })
    }

    fn inspect_descendant<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.descendants().find(|c| match c.meta().get(*INSPECTOR_INFO_ID) {
            Some(info) => pattern.matches(info),
            None => false,
        })
    }

    fn inspect_ancestor<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.ancestors().find(|c| match c.meta().get(*INSPECTOR_INFO_ID) {
            Some(info) => pattern.matches(info),
            None => false,
        })
    }

    fn inspect_property<P: InspectPropertyPattern>(&self, pattern: P) -> Option<&dyn PropertyArgs> {
        self.meta()
            .get(*INSPECTOR_INFO_ID)?
            .properties()
            .find_map(|(args, cap)| if pattern.matches(args, cap) { Some(args) } else { None })
    }

    fn parent_property(&self) -> Option<(PropertyId, usize)> {
        self.parent()?.meta().get(*INSPECTOR_INFO_ID)?.properties().find_map(|(args, _)| {
            let id = self.id();
            let info = args.property();
            for (i, input) in info.inputs.iter().enumerate() {
                match input.kind {
                    InputKind::UiNode => {
                        let node = args.ui_node(i);
                        let mut found = false;
                        node.try_node(|n| {
                            if n.is_list() {
                                // parent's property input is a list, are we on that list?
                                n.for_each_child(|_, n| {
                                    if !found && let Some(mut wgt) = n.as_widget() {
                                        found = wgt.id() == id;
                                    }
                                });
                            } else if let Some(mut wgt) = n.as_widget() {
                                // parent's property input is an widget, is that us?
                                found = wgt.id() == id;
                            }
                        });
                        if found {
                            return Some((args.id(), i));
                        }
                    }
                    _ => continue,
                }
            }
            None
        })
    }
}

/// Query pattern for the [`WidgetInfoInspectorExt`] inspect methods.
pub trait InspectWidgetPattern {
    /// Returns `true` if the pattern includes the widget.
    fn matches(&self, info: &InspectorInfo) -> bool;
}
/// Matches if the [`WidgetType::path`] ends with the string.
impl InspectWidgetPattern for &str {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().path.ends_with(self)
    }
}
impl InspectWidgetPattern for TypeId {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().type_id == *self
    }
}
impl InspectWidgetPattern for WidgetType {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().type_id == self.type_id
    }
}

/// Query pattern for the [`WidgetInfoInspectorExt`] inspect methods.
pub trait InspectPropertyPattern {
    /// Returns `true` if the pattern includes the property.
    fn matches(&self, args: &dyn PropertyArgs, captured: bool) -> bool;
}
/// Matches if the [`PropertyInfo::name`] exactly.
///
/// [`PropertyInfo::name`]: crate::widget::builder::PropertyInfo::name
impl InspectPropertyPattern for &str {
    fn matches(&self, args: &dyn PropertyArgs, _: bool) -> bool {
        args.property().name == *self
    }
}
impl InspectPropertyPattern for PropertyId {
    fn matches(&self, args: &dyn PropertyArgs, _: bool) -> bool {
        args.id() == *self
    }
}
