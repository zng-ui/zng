//! Helper types for inspecting an UI tree.
//!
//! When compiled with the `"inspector"` feature all widget instances are instrumented with inspection node
//! that shares a clone of the [`WidgetBuilder`] in the [`WidgetInfo`].

pub mod prompt;

#[cfg(inspector)]
mod inspector_only {
    use std::rc::Rc;

    use crate::context::InfoContext;
    use crate::ui_node;
    use crate::widget_builder::WidgetBuilder;
    use crate::widget_info::WidgetInfoBuilder;
    use crate::widget_instance::{BoxedUiNode, UiNode};

    pub(crate) fn insert_widget_builder_info(child: BoxedUiNode, wgt: WidgetBuilder) -> impl UiNode {
        #[ui_node(struct InsertInfoNode {
            child: BoxedUiNode,
            builder: Rc<WidgetBuilder>,
        })]
        impl UiNode for InsertInfoNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                info.meta().set(&super::WIDGET_BUILDER_ID, self.builder.clone());
                self.child.info(ctx, info);
            }
        }
        InsertInfoNode {
            child,
            builder: Rc::new(wgt),
        }
    }
}
#[cfg(inspector)]
pub(crate) use inspector_only::*;

use std::rc::Rc;

use crate::{
    context::StaticStateId,
    widget_builder::{PropertyArgs, PropertyId, PropertyImplId, WidgetBuilder, WidgetImplId, WidgetMod},
    widget_info::WidgetInfo,
};

pub(super) static WIDGET_BUILDER_ID: StaticStateId<Rc<WidgetBuilder>> = StaticStateId::new_unique();

/// Extensions methods for [`WidgetInfo`].
pub trait WidgetInfoInspectorExt<'a> {
    /// Reference the builder that was used to instantiate the widget.
    ///
    /// Returns `None` if not build with the `"inspector"` feature, or if the widget instance was not created using
    /// the standard builder.
    fn builder(self) -> Option<Rc<WidgetBuilder>>;

    /// If a [`builder`] is defined for the widget.
    ///
    /// [`builder`]: Self::builder
    fn can_inspect(self) -> bool;

    /// Returns the first child that matches.
    fn inspect_child<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>>;

    /// Returns the first descendant that matches.
    ///
    /// # Examples
    ///
    /// Example searches for a "button" descendant, using a string search that matches the end of the [`WidgetMod::path`] and
    /// an exact widget mod that matches the [`WidgetMod::impl_id`].
    ///
    /// ```
    /// use zero_ui_core::inspector::*;
    /// use zero_ui_core::widget_info::*;
    /// mod mod widgets {
    ///     use zero_ui_core::*;
    ///     
    ///     #[widget($crate::widgets::button)]
    ///     pub mod button {
    ///         inherit!(zero_ui_core::widget_base::base);
    ///     }
    /// }
    /// fn demo(info: WidgetInfo) {
    /// let fuzzy = info.inspect_descendant("button");
    /// let exact = info.inspect_descendant(widget_mod!(crate::widgets::button));
    /// }
    /// ```
    fn inspect_descendant<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>>;

    /// Returns the first ancestor that matches.
    fn inspect_ancestor<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>>;

    /// Search for a property set on the widget.
    ///
    /// # Examples
    ///
    /// Search for a property by name, and then downcast its value.
    ///
    /// ```
    /// # use zero_ui_core::inspector::*;
    /// # use zero_ui_core::widget_info::*;
    /// fn inspect_foo(info: WidgetInfo) -> Option<bool> {
    ///     info.inspect_property("foo")?.value(0).as_any().downcast_ref().copied()
    /// }
    /// ```
    fn inspect_property<P: InspectPropertyPattern>(self, pattern: P) -> Option<&'a dyn PropertyArgs>;
}
impl<'a> WidgetInfoInspectorExt<'a> for WidgetInfo<'a> {
    fn builder(self) -> Option<Rc<WidgetBuilder>> {
        self.meta().get(&WIDGET_BUILDER_ID).cloned()
    }

    fn can_inspect(self) -> bool {
        self.meta().contains(&WIDGET_BUILDER_ID)
    }

    fn inspect_child<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>> {
        self.children().find(|c| match c.meta().get(&WIDGET_BUILDER_ID) {
            Some(wgt) => pattern.matches(wgt),
            None => false,
        })
    }

    fn inspect_descendant<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>> {
        self.descendants().find(|c| match c.meta().get(&WIDGET_BUILDER_ID) {
            Some(wgt) => pattern.matches(wgt),
            None => false,
        })
    }

    fn inspect_ancestor<P: InspectWidgetPattern>(self, pattern: P) -> Option<WidgetInfo<'a>> {
        self.ancestors().find(|c| match c.meta().get(&WIDGET_BUILDER_ID) {
            Some(wgt) => pattern.matches(wgt),
            None => false,
        })
    }

    fn inspect_property<P: InspectPropertyPattern>(self, pattern: P) -> Option<&'a dyn PropertyArgs> {
        self.meta()
            .get(&WIDGET_BUILDER_ID)?
            .properties()
            .find_map(|(_, _, args)| if pattern.matches(args) { Some(args) } else { None })
    }
}

/// Query pattern for the [`WidgetInspectorExt`] inspect methods.
pub trait InspectWidgetPattern {
    /// Returns `true` if the pattern includes the widget.
    fn matches(&self, wgt: &WidgetBuilder) -> bool;
}
/// Matches if the [`WidgetMod::path`] ends with the string.
impl<'s> InspectWidgetPattern for &'s str {
    fn matches(&self, wgt: &WidgetBuilder) -> bool {
        wgt.widget_mod().path.ends_with(self)
    }
}
impl InspectWidgetPattern for WidgetImplId {
    fn matches(&self, wgt: &WidgetBuilder) -> bool {
        wgt.widget_mod().impl_id == *self
    }
}
impl InspectWidgetPattern for WidgetMod {
    fn matches(&self, wgt: &WidgetBuilder) -> bool {
        wgt.widget_mod().impl_id == self.impl_id
    }
}

/// Query pattern for the [`WidgetInspectorExt`] inspect methods.
pub trait InspectPropertyPattern {
    /// Returns `true` if the pattern includes the property.
    fn matches(&self, args: &dyn PropertyArgs) -> bool;
}
/// Matches if the [`PropertyInstInfo::name`] exactly.
impl<'s> InspectPropertyPattern for &'s str {
    fn matches(&self, args: &dyn PropertyArgs) -> bool {
        args.instance().name == *self
    }
}
impl InspectPropertyPattern for PropertyId {
    fn matches(&self, args: &dyn PropertyArgs) -> bool {
        args.id() == *self
    }
}
impl InspectPropertyPattern for PropertyImplId {
    fn matches(&self, args: &dyn PropertyArgs) -> bool {
        args.property().impl_id == *self
    }
}
