#![cfg(any(doc, inspector))]
#![cfg_attr(doc_nightly, doc(cfg(any(debug_assertions, inspector))))]

//! Helper types for inspecting an UI tree.
//!
//! When compiled with the `"inspector"` feature all widget instances are instrumented with inspection nodes
//! that generate a [`WidgetInstanceInfo`] in the [`WidgetInfo`], it contains metadata about the widget properties, constructors
//! and `when` conditions, each property's argument value is also available for inspection as a variable of [`PropertyValue`], the
//! variable updates with the original variable if the property was set to a variable.
//!
//! The primary use of this module is as a data source for UI inspectors, but it can also be used for rudimentary *reflection*, note
//! that there is a runtime performance impact, compiling with `"inspector"` is the equivalent of using `"dyn_node"`, all static arguments
//! are cloned, a duplicate of each kept in the heap, widget init and info is slowed by all the metadata collection.

pub mod prompt;

use std::any::type_name;
use std::fmt;
use std::{any::Any, rc::Rc};

use crate::context::{InfoContext, LayoutContext, MeasureContext, RenderContext, StaticStateId, UpdatesTrace, WidgetUpdates};
use crate::event::EventUpdate;
use crate::render::FrameUpdate;
use crate::text::Text;
use crate::widget_info::{WidgetInfo, WidgetInfoBuilder, WidgetLayout};
use crate::{units::*, var::*, *};

/// Represents an inspected property var value.
#[derive(Clone)]
pub struct PropertyValue {
    value: Rc<dyn Any>,
    type_name: &'static str,
    fmt: Option<fn(&dyn Any, f: &mut fmt::Formatter) -> fmt::Result>,
}
fn fmt_property_value<T: fmt::Debug + 'static>(value: &dyn Any, f: &mut fmt::Formatter) -> fmt::Result {
    let value = value.downcast_ref::<T>().unwrap();
    fmt::Debug::fmt(value, f)
}
impl PropertyValue {
    /// New property var mapped from the `property_var`.
    pub fn new_var<T: VarValue>(property_var: &impl Var<T>) -> BoxedVar<PropertyValue> {
        property_var
            .map(|v| PropertyValue {
                value: Rc::new(v.clone()),
                type_name: type_name::<T>(),
                fmt: Some(fmt_property_value::<T>),
            })
            .boxed()
    }

    /// New property var from `value`.
    pub fn new_value<T: fmt::Debug + Any>(value: T) -> BoxedVar<PropertyValue> {
        PropertyValue {
            value: Rc::new(value),
            type_name: type_name::<T>(),
            fmt: Some(fmt_property_value::<T>),
        }
        .into_var()
        .boxed()
    }

    /// New property from `value` that is not debug.
    pub fn new_any<T: Clone + Any>(value: T) -> BoxedVar<PropertyValue> {
        PropertyValue {
            value: Rc::new(value),
            type_name: type_name::<T>(),
            fmt: None,
        }
        .into_var()
        .boxed()
    }

    /// New property with no value, an anonymous nil value is used, but with the type name from `T`,
    /// this is the ultimate fallback for abnormal properties.
    pub fn new_type_name_only<T>() -> BoxedVar<PropertyValue> {
        struct TypeNameOnly;
        PropertyValue {
            value: Rc::new(TypeNameOnly),
            type_name: type_name::<T>(),
            fmt: None,
        }
        .into_var()
        .boxed()
    }

    /// Value type name.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Gets the [`type_name`] cleaned or replaced for display, common core generic types
    /// are replaced with custom names and other types are formatted to `<{type_name}>`.
    pub fn type_name_display(&self) -> Text {
        if self.type_name.contains("::WidgetNode") {
            Text::from_static("<widget!>")
        } else if self.type_name.contains("::WidgetVec") || self.type_name.contains("::WidgetList") {
            Text::from_static("<[ui_list!]>")
        } else if self.type_name.contains("::UiNodeVec") || self.type_name.contains("::UiNodeList") {
            Text::from_static("<[ui_list!]>")
        } else if self.type_name.ends_with("{{closure}}") {
            Text::from_static("<{{closure}}>")
        } else if self.type_name.contains("::FnMutWidgetHandler<") {
            Text::from_static("hn!({{closure}})")
        } else if self.type_name.contains("::FnOnceWidgetHandler<") {
            Text::from_static("hn_once!({{closure}})")
        } else if self.type_name.contains("::AsyncFnMutWidgetHandler<") {
            Text::from_static("async_hn!({{closure}})")
        } else if self.type_name.contains("::AsyncFnOnceWidgetHandler<") {
            Text::from_static("async_hn_once!({{closure}})")
        } else if self.type_name.contains("::FnMutAppHandler<") {
            Text::from_static("app_hn!({{closure}})")
        } else if self.type_name.contains("::FnOnceAppHandler<") {
            Text::from_static("app_hn_once!({{closure}})")
        } else if self.type_name.contains("::AsyncFnMutAppHandler<") {
            Text::from_static("async_app_hn!({{closure}})")
        } else if self.type_name.contains("::AsyncFnOnceAppHandler<") {
            Text::from_static("async_app_hn_once!({{closure}})")
        } else if self.type_name.contains("::Box<dyn zero_ui_core::ui_node::WidgetBoxed>") {
            Text::from_static("BoxedWidget")
        } else {
            formatx!("<{}>", pretty_type_name::pretty_type_name_str(self.type_name))
        }
    }

    /// Value reference.
    ///
    /// Note that you can debug print the `PropertyValue` and to print the value without knowing the type.
    pub fn value(&self) -> &dyn Any {
        &*self.value
    }

    /// If debug printing the `PropertyValue` prints the value, if `true` the call is redirected to the value [`fmt::Debug`],
    /// if `false` the `"<{type_name}>"` is print.
    pub fn value_is_debug(&self) -> bool {
        self.fmt.is_some()
    }
}
impl fmt::Debug for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fmt {
            Some(fmt) => fmt(&*self.value, f),
            None => write!(f, "{}", self.type_name_display()),
        }
    }
}

/// Represents one argument of a property.
#[derive(Clone)]
pub struct PropertyArg {
    /// Argument name.
    pub name: &'static str,
    /// Argument value, the variable updates when the original value updates.
    pub value: BoxedVar<PropertyValue>,
}
impl fmt::Debug for PropertyArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyArg").field("name", &self.name).finish_non_exhaustive()
    }
}




/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPriority {
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
    /// [Capture-only](crate::property#capture_only) property.
    CaptureOnly,
}
impl PropertyPriority {
    /// Items from `Context` to `ChildLayout`.
    pub fn context_to_child_layout() -> &'static [PropertyPriority] {
        &[
            PropertyPriority::Context,
            PropertyPriority::Event,
            PropertyPriority::Layout,
            PropertyPriority::Size,
            PropertyPriority::Border,
            PropertyPriority::Fill,
            PropertyPriority::ChildContext,
            PropertyPriority::ChildLayout,
        ]
    }

    /// Name of the priority in a property attribute declaration.
    pub fn name(self) -> &'static str {
        match self {
            PropertyPriority::Context => "context",
            PropertyPriority::Event => "event",
            PropertyPriority::Layout => "layout",
            PropertyPriority::Size => "size",
            PropertyPriority::Border => "border",
            PropertyPriority::Fill => "fill",
            PropertyPriority::ChildContext => "child_context",
            PropertyPriority::ChildLayout => "child_layout",
            PropertyPriority::CaptureOnly => "capture_only",
        }
    }

    /// Name of the widget static constructor function for the priority.
    ///
    /// Returns empty string for `CaptureOnly`.
    pub fn name_constructor(self) -> &'static str {
        match self {
            PropertyPriority::Context => "new_context",
            PropertyPriority::Event => "new_event",
            PropertyPriority::Layout => "new_layout",
            PropertyPriority::Size => "new_size",
            PropertyPriority::Border => "new_border",
            PropertyPriority::Fill => "new_fill",
            PropertyPriority::ChildContext => "new_child_context",
            PropertyPriority::ChildLayout => "new_child_layout",
            PropertyPriority::CaptureOnly => "",
        }
    }

    /// Name of the widget dynamic constructor function for the priority.
    ///
    /// Returns empty string for `CaptureOnly`.
    pub fn name_constructor_dyn(self) -> &'static str {
        match self {
            PropertyPriority::Context => "new_context_dyn",
            PropertyPriority::Event => "new_event_dyn",
            PropertyPriority::Layout => "new_layout_dyn",
            PropertyPriority::Size => "new_size_dyn",
            PropertyPriority::Border => "new_border_dyn",
            PropertyPriority::Fill => "new_fill_dyn",
            PropertyPriority::ChildContext => "new_child_context_dyn",
            PropertyPriority::ChildLayout => "new_child_layout_dyn",
            PropertyPriority::CaptureOnly => "",
        }
    }
}

/// Debug information about a property of a widget instance.
#[derive(Debug, Clone)]
pub struct PropertyInstanceInfo {
    /// About the property.
    pub meta: PropertyInstanceMeta,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArg]>,
}

/// Identifies the property or constructor of the parent widget that introduces the widget inspected.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WidgetInstanceParent {
    /// The widget is the root widget.
    Root,
    /// The widget is introduced by the property name in the parent widget.
    Property(&'static str),
    /// The widget is introduced by the constructor name in the parent widget.
    Constructor(&'static str),
}
impl WidgetInstanceParent {
    /// Returns the property name, constructor name or an empty string.
    pub fn as_str(self) -> &'static str {
        match self {
            WidgetInstanceParent::Root => "",
            WidgetInstanceParent::Property(s) => s,
            WidgetInstanceParent::Constructor(s) => s,
        }
    }
}

/// Debug information about a widget instance.
///
/// Use  the [`WidgetInfoInspectorExt::instance`] to get the inspection info for a [`WidgetInfo`].
pub struct WidgetInstanceInfo {
    /// About the widget.
    pub meta: WidgetInstanceMeta,

    /// Widget properties, from *outermost* to *innermost*.
    pub properties: Vec<PropertyInstanceInfo>,

    /// When blocks setup by this widget instance.
    pub whens: Box<[WhenInfo]>,

    /// Widget constructors and captured properties.
    pub constructors: Vec<WidgetConstructorInfo>,

    /// Name of the parent widget property or new function that introduces this widget.
    ///
    /// Empty string (`""`) when the widget has no parent with debug enabled.
    pub parent_name: WidgetInstanceParent,
}
impl WidgetInstanceInfo {
    /// Find the static or dynamic constructor function for the given static name.
    ///
    /// If `static_ctor` is `"new_context"` searches for it and `"new_context_dyn"`.
    pub fn constructor(&self, static_ctor: &str) -> Option<&WidgetConstructorInfo> {
        for ctor in &self.constructors {
            if let Some(maybe_dyn) = ctor.fn_name.strip_prefix(static_ctor) {
                if maybe_dyn.is_empty() || maybe_dyn == "_dyn" {
                    return Some(ctor);
                }
            }
        }
        None
    }

    /// Fin a property or property capture with the `property_name`.
    pub fn property(&self, property_name: &str) -> Option<PropertyOrCapture> {
        for p in self.properties.iter() {
            if p.meta.property_name == property_name {
                return Some(PropertyOrCapture::Property(p));
            }
        }
        for ctor in &self.constructors {
            for cap in ctor.captures.iter() {
                if cap.property_name == property_name {
                    return Some(PropertyOrCapture::Capture(cap));
                }
            }
        }
        None
    }

    /// Iterate over all captured and full properties of the `priority`.
    pub fn properties(&self, priority: PropertyPriority) -> impl Iterator<Item = PropertyOrCapture> {
        let caps = self
            .constructor(priority.name_constructor())
            .into_iter()
            .flat_map(|ctor| ctor.captures.iter().map(PropertyOrCapture::Capture));

        let fulls = self
            .properties
            .iter()
            .filter(move |p| p.meta.priority == priority)
            .map(PropertyOrCapture::Property);

        caps.chain(fulls)
    }
}

/// Full property or capture.
#[derive(Debug)]
pub enum PropertyOrCapture<'a> {
    /// Represents a full property, implemented by a property function.
    Property(&'a PropertyInstanceInfo),
    /// Represents a captured property, implemented by one of the widget's constructors.
    Capture(&'a CapturedPropertyInfo),
}
impl<'a> PropertyOrCapture<'a> {
    /// Get the property name.
    pub fn property_name(&self) -> &'static str {
        match self {
            PropertyOrCapture::Property(p) => p.meta.property_name,
            PropertyOrCapture::Capture(p) => p.property_name,
        }
    }

    /// Source-code location of the widget instantiation or property assign.
    pub fn instance_location(&self) -> SourceLocation {
        match self {
            PropertyOrCapture::Property(p) => p.meta.instance_location,
            PropertyOrCapture::Capture(p) => p.instance_location,
        }
    }

    /// Get the property arguments.
    pub fn args(&self) -> &[PropertyArg] {
        match self {
            PropertyOrCapture::Property(p) => &p.args,
            PropertyOrCapture::Capture(p) => &p.args,
        }
    }

    /// Get the property argument by index.
    pub fn arg(&self, index: usize) -> &PropertyArg {
        &self.args()[index]
    }

    /// If the user assigned this property.
    pub fn user_assigned(&self) -> bool {
        match self {
            PropertyOrCapture::Property(p) => p.meta.user_assigned,
            PropertyOrCapture::Capture(p) => p.user_assigned,
        }
    }
}

/// Adds the the inspector methods to [`WidgetInfo`].
pub trait WidgetInfoInspectorExt<'a> {
    /// If the widget contains inspected info.
    #[allow(clippy::wrong_self_convention)] // WidgetInfo is a reference.
    fn is_inspected(self) -> bool;

    /// Gets the inspector info about the widget, if it is inspected.
    fn instance(self) -> Option<&'a WidgetInstanceInfo>;

    /// Find for an inspected child widget with the `widget_name`.
    fn child_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>>;

    /// Find for an inspected descendant widget with the `widget_name`.
    fn descendant_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>>;

    /// Find for an inspected parent widget with the `widget_name`.
    fn ancestor_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>>;
}
impl<'a> WidgetInfoInspectorExt<'a> for WidgetInfo<'a> {
    fn is_inspected(self) -> bool {
        self.meta().contains(&WIDGET_INSTANCE_INFO_ID)
    }

    fn instance(self) -> Option<&'a WidgetInstanceInfo> {
        self.meta().get(&WIDGET_INSTANCE_INFO_ID)
    }

    fn child_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>> {
        for c in self.children() {
            if let Some(inst) = c.instance() {
                if inst.meta.widget_name == widget_name {
                    return Some(c);
                }
            }
        }
        None
    }

    fn descendant_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>> {
        for c in self.descendants() {
            if let Some(inst) = c.instance() {
                if inst.meta.widget_name == widget_name {
                    return Some(c);
                }
            }
        }
        None
    }

    fn ancestor_instance(self, widget_name: &str) -> Option<WidgetInfo<'a>> {
        for c in self.ancestors() {
            if let Some(inst) = c.instance() {
                if inst.meta.widget_name == widget_name {
                    return Some(c);
                }
            }
        }
        None
    }
}

static WIDGET_INSTANCE_INFO_ID: StaticStateId<WidgetInstanceInfo> = StaticStateId::new_unique();

/// Metadata about an inspected property.
///
/// See [`PropertyInstanceInfo`].
#[derive(Clone, Debug)]
pub struct PropertyInstanceMeta {
    /// Property priority in a widget.
    ///
    /// See [the property doc](crate::property#priority) for more details.
    pub priority: PropertyPriority,

    /// Original name of the property.
    pub original_name: &'static str,
    /// Source-code location of the property declaration.
    pub decl_location: SourceLocation,

    /// Name of the property in the widget.
    pub property_name: &'static str,
    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// If the user assigned this property.
    pub user_assigned: bool,
}

/// Metadata about an inspected widget.
///
/// See [`WidgetInstanceInfo`].
#[derive(Clone, Debug)]
pub struct WidgetInstanceMeta {
    /// Unique ID of the widget instantiation.
    pub instance_id: WidgetInstanceId,

    /// Widget type name.
    pub widget_name: &'static str,

    /// Source-code location of the widget declaration.
    pub decl_location: SourceLocation,

    /// Source-code location of the widget instantiation.
    pub instance_location: SourceLocation,
}
unique_id_64! {
    /// Unique ID of a widget instance.
    ///
    /// This is different from the `WidgetId` in that it cannot be manipulated by the user
    /// and identifies the widget *instantiation* event during debug mode.
    #[derive(Debug)]
    pub struct WidgetInstanceId;
}

/// Inspector info about a property captured by a widget constructor function.
#[derive(Debug, Clone)]
pub struct CapturedPropertyInfo {
    /// Name of the property in the widget.
    pub property_name: &'static str,

    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// If the user assigned this property.
    pub user_assigned: bool,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArg]>,
}

/// Inspector info about one of the constructor functions of a widget.
#[derive(Debug, Clone)]
pub struct WidgetConstructorInfo {
    /// Constructor function name.
    pub fn_name: &'static str,

    /// Properties captured by the constructor.
    pub captures: Box<[CapturedPropertyInfo]>,
}

/// When block setup by a widget instance.
#[derive(Clone)]
pub struct WhenInfo {
    /// When condition expression.
    pub condition_expr: &'static str,
    /// Current when condition result.
    pub condition: BoxedVar<bool>,

    /// Properties affected by this when block.
    pub properties: Box<[&'static str]>,

    /// Source-code location of the when block declaration.
    pub decl_location: SourceLocation,

    /// If the user declared the when block in the widget instance.
    pub user_declared: bool,
}
impl fmt::Debug for WhenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhenInfo")
            .field("condition_expr", &self.condition_expr)
            .field("properties", &self.properties)
            .field("decl_location", &self.decl_location)
            .field("user_declared", &self.user_declared)
            .finish_non_exhaustive()
    }
}

/// Wraps the `property_node` with the property inspector metadata.
pub fn inspect_property(property_node: BoxedUiNode, meta: PropertyInstanceMeta, args: Box<[PropertyArg]>) -> BoxedUiNode {
    InspectPropertyNode {
        child: property_node,
        meta,
        args,
    }
    .boxed()
}

#[ui_node(struct InspectPropertyNode {
    child: BoxedUiNode,
    meta: PropertyInstanceMeta,
    args: Box<[PropertyArg]>,
})]
impl UiNode for InspectPropertyNode {
    fn init(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "init");
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "deinit");
        self.child.deinit(ctx);
    }

    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "info");
        if let Some(wgt) = info.meta().get_mut(&WIDGET_INSTANCE_INFO_ID) {
            wgt.properties.push(PropertyInstanceInfo {
                meta: self.meta.clone(),
                args: self.args.clone(),
            });
        }

        let parent = WidgetInstanceParent::Property(self.meta.property_name);
        WIDGET_PARENT.with_context(&mut Some(parent), || {
            self.child.info(ctx, info);
        });
    }

    fn event(&mut self, ctx: &mut context::WidgetContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "event");
        self.child.event(ctx, update);
    }

    fn update(&mut self, ctx: &mut context::WidgetContext, updates: &mut WidgetUpdates) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "update");
        self.child.update(ctx, updates);
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "measure");
        self.child.measure(ctx)
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> units::PxSize {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "layout");
        self.child.layout(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "render");
        self.child.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let _span = UpdatesTrace::property_span(self.meta.property_name, "render_update");
        self.child.render_update(ctx, update);
    }
}

/// Wraps the `widget_outermost_node` with the widget inspector metadata.
pub fn inspect_widget(widget_outermost_node: BoxedUiNode, meta: WidgetInstanceMeta, whens: Box<[WhenInfo]>) -> BoxedUiNode {
    InspectWidgetNode {
        child: widget_outermost_node,
        meta,
        whens,
    }
    .boxed()
}

#[ui_node(struct InspectWidgetNode {
    child: BoxedUiNode,
    meta: WidgetInstanceMeta,
    whens: Box<[WhenInfo]>,
})]
impl UiNode for InspectWidgetNode {
    fn init(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "init");
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "deinit");
        self.child.deinit(ctx);
    }

    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "info");
        info.meta().set(
            &WIDGET_INSTANCE_INFO_ID,
            WidgetInstanceInfo {
                meta: self.meta.clone(),
                properties: vec![],
                whens: self.whens.clone(),
                constructors: vec![],
                parent_name: WIDGET_PARENT.get(),
            },
        );
        self.child.info(ctx, info);
    }

    fn event(&mut self, ctx: &mut context::WidgetContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "event");
        self.child.event(ctx, update);
    }

    fn update(&mut self, ctx: &mut context::WidgetContext, updates: &mut WidgetUpdates) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "update");
        self.child.update(ctx, updates);
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "measure");
        self.child.measure(ctx)
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "layout");
        self.child.layout(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "render");
        self.child.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let _span = UpdatesTrace::widget_span(ctx.path.widget_id(), self.meta.widget_name, "render_update");
        self.child.render_update(ctx, update);
    }
}

/// Wraps the `constructor_node` with the widget constructor inspector metadata.
pub fn inspect_constructor(constructor_node: BoxedUiNode, fn_name: &'static str, captures: Box<[CapturedPropertyInfo]>) -> BoxedUiNode {
    InspectConstructorNode {
        child: constructor_node,
        fn_name,
        captures,
    }
    .boxed()
}

#[ui_node(struct InspectConstructorNode {
    child: BoxedUiNode,
    fn_name: &'static str,
    captures: Box<[CapturedPropertyInfo]>,
})]
impl UiNode for InspectConstructorNode {
    fn init(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "init");
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut context::WidgetContext) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "deinit");
        self.child.deinit(ctx);
    }

    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "info");

        if let Some(wgt) = info.meta().get_mut(&WIDGET_INSTANCE_INFO_ID) {
            wgt.constructors.push(WidgetConstructorInfo {
                fn_name: self.fn_name,
                captures: self.captures.clone(),
            });
        }

        let parent = WidgetInstanceParent::Constructor(self.fn_name);
        WIDGET_PARENT.with_context(&mut Some(parent), || {
            self.child.info(ctx, info);
        });
    }

    fn event(&mut self, ctx: &mut context::WidgetContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "event");
        self.child.event(ctx, update);
    }

    fn update(&mut self, ctx: &mut context::WidgetContext, updates: &mut WidgetUpdates) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "update");
        self.child.update(ctx, updates);
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "measure");
        self.child.measure(ctx)
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "layout");
        self.child.layout(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "render");
        self.child.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let _span = UpdatesTrace::constructor_span(self.fn_name, "render_update");
        self.child.render_update(ctx, update);
    }
}

context_value! {
    static WIDGET_PARENT: WidgetInstanceParent = WidgetInstanceParent::Root;
}

/// Remove the constructor function info node from the `child`.
///
/// Widgets constructors may try to cast the child parameter to the type returned by the
/// inner constructor, but if the widget is instantiating with inspector that node was wrapped with the
/// an inspector node, this function removes this inspector node.
///
/// # Examples
///
/// The example demonstrates a custom widget that passes a custom node that is modified in each constructor function,
/// the [`downcast_unbox`] would fail without the unwrap, because the `FooNode` is auto wrapped in an info node.
///
/// [`downcast_unbox`]: UiNode::downcast_unbox
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::*;
/// #
/// # #[derive(Default)]
/// # struct FooNode {
/// #     bar: bool,
/// # }
/// # #[ui_node(none)]
/// # impl UiNode for FooNode { }
/// #
/// #[widget($crate::foo)]
/// pub mod foo {
///     use super::*;
///
///     fn new_child() -> impl UiNode {
///         FooNode::default()
///     }
///     
///     fn new_child_layout(child: impl UiNode) -> impl UiNode {
///         let child = child.boxed();
///         #[cfg(feature = "inspector")]
///         let child = zero_ui_core::inspector::unwrap_constructor(child);
///
///         match child.downcast_unbox::<FooNode>() {
///             Ok(mut foo) => {
///                 foo.bar = true;
///                 foo.boxed()
///             },
///             Err(n) => n
///         }
///     }
///
///     // .. other constructors
/// }
/// ```
pub fn unwrap_constructor(child: BoxedUiNode) -> BoxedUiNode {
    let mut child = child;

    loop {
        match child.downcast_unbox::<InspectConstructorNode>() {
            Ok(n) => child = n.child,
            Err(w) => match w.downcast_unbox::<InspectWidgetNode>() {
                Ok(n) => {
                    child = match n.child.downcast_unbox::<InspectConstructorNode>() {
                        Ok(n) => n.child,
                        Err(child) => child,
                    }
                }
                Err(child) => return child,
            },
        }
    }
}

#[doc(hidden)]
pub mod debug_var_util {
    use std::{any::Any, fmt::Debug};

    use crate::var::{BoxedVar, IntoValue, IntoVar, Var, VarValue};

    use super::PropertyValue;

    pub struct Wrap<T>(pub T);

    //
    // `Wrap` - type_name only
    //
    pub trait FromTypeNameOnly {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T> FromTypeNameOnly for Wrap<&T> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_type_name_only::<T>()
        }
    }

    //
    // `&Wrap` - Clone + Any
    //
    pub trait FromCloneOnly {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T: Clone + Any> FromTypeNameOnly for &Wrap<&T> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_any(self.0.clone())
        }
    }

    //
    // `&&Wrap` - Into<Debug + Any>
    //
    pub trait FromIntoValueDebug<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T: Debug + Any, V: IntoValue<T>> FromIntoValueDebug<T> for &&Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_value(self.0.clone().into())
        }
    }

    //
    // `&&&Wrap` - IntoVar<T>
    //
    pub trait FromIntoVar<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T: VarValue, V: IntoVar<T>> FromIntoVar<T> for &&&Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_var(&self.0.clone().into_var())
        }
    }

    //
    // `&&&&Wrap` - Debug + Clone + Any
    //
    pub trait FromDebug {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T: Debug + Clone + Any> FromDebug for &&&&Wrap<&T> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_value(self.0.clone())
        }
    }

    //
    // `&&&&&Wrap` - Var<T>
    //
    pub trait FromVarDebugOnly<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<PropertyValue>;
    }
    impl<T: VarValue, V: Var<T>> FromVarDebugOnly<T> for &&&&&Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<PropertyValue> {
            PropertyValue::new_var(self.0)
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::var::{IntoValue, Var};

        macro_rules! debug_var_util_trick {
            ($value:expr) => {{
                use $crate::inspector::debug_var_util::*;
                (&&&&&Wrap($value)).debug_var()
            }};
        }

        use crate::context::TestWidgetContext;

        #[test]
        fn from_into_var() {
            use crate::var::IntoVar;
            fn value() -> impl IntoVar<&'static str> {
                #[derive(Clone, Copy)]
                struct Test;
                impl IntoVar<&'static str> for Test {
                    type Var = crate::var::LocalVar<&'static str>;

                    fn into_var(self) -> Self::Var {
                        crate::var::LocalVar("called into_var")
                    }
                }
                Test
            }
            let value = value();

            let r = debug_var_util_trick!(&value);

            assert_eq!("\"called into_var\"", format!("{:?}", r.get()))
        }

        #[test]
        pub fn from_into_value() {
            fn value() -> impl IntoValue<bool> {
                true
            }
            let value = value();

            let r = debug_var_util_trick!(&value);

            assert_eq!("true", format!("{:?}", r.get()))
        }

        #[test]
        fn from_var() {
            use crate::var::var;

            let value = var(true);

            let r = debug_var_util_trick!(&value);

            let mut ctx = TestWidgetContext::new();

            assert_eq!("true", format!("{:?}", r.get()));

            value.set(&ctx.vars, false);

            ctx.apply_updates();

            assert_eq!("false", format!("{:?}", r.get()));
        }

        #[test]
        fn from_debug() {
            let value = true;

            let r = debug_var_util_trick!(&value);

            assert_eq!("true", format!("{:?}", r.get()))
        }

        #[test]
        fn from_any() {
            struct Foo;
            let value = Foo;

            let r = debug_var_util_trick!(&value);

            assert!(format!("{:?}", r.get()).contains("Foo"));
        }
    }
}

#[doc(hidden)]
pub mod v1 {
    // types used by the proc-macro instrumentation.

    pub use super::{
        debug_var_util, inspect_constructor, inspect_property, inspect_widget, source_location, CapturedPropertyInfo, PropertyArg,
        PropertyInstanceMeta, PropertyPriority, SourceLocation, WhenInfo, WidgetInstanceId, WidgetInstanceMeta,
    };
}
