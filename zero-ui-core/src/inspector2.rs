use std::any::type_name;
use std::fmt;
use std::{any::Any, rc::Rc};

use linear_map::set::LinearSet;

use crate::context::{InfoContext, LayoutContext, MeasureContext, RenderContext, StaticStateId};
use crate::widget_info::{WidgetInfo, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions};
use crate::{units::*, var::*, *};

/// Represents an inspected property var value.
#[derive(Clone)]
pub struct PropertyValue {
    value: Rc<dyn Any>,
    type_name: &'static str,
    fmt: Option<fn(&dyn Any, f: &mut fmt::Formatter) -> fmt::Result>,
}
fn fmt_property_value<T: VarValue>(value: &dyn Any, f: &mut fmt::Formatter) -> fmt::Result {
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
    pub fn new_value<T: VarValue>(value: &T) -> BoxedVar<PropertyValue> {
        PropertyValue {
            value: Rc::new(value.clone()),
            type_name: type_name::<T>(),
            fmt: Some(fmt_property_value::<T>),
        }
        .into_var()
        .boxed()
    }

    /// New property from `value` that is not debug.
    pub fn new_any<T: Clone + Any>(value: &T) -> BoxedVar<PropertyValue> {
        PropertyValue {
            value: Rc::new(value.clone()),
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
            None => write!(f, "<{}>", self.type_name),
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

///<span data-del-macro-root></span> New [`SourceLocation`] that represents the location you call this macro.
#[macro_export]
macro_rules! source_location {
    () => {
        $crate::inspector::SourceLocation {
            file: std::file!(),
            line: std::line!(),
            column: std::column!(),
        }
    };
}
#[doc(inline)]
pub use crate::source_location;

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
    fn context_to_child_layout() -> &'static [PropertyPriority] {
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

    fn token_str(self) -> &'static str {
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

/// Debug information about a widget instance.
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
    /// Get the widget inspect info, if the widget is inspected.
    pub fn get(info: WidgetInfo) -> Option<&WidgetInstanceInfo> {
        info.meta().get(&WIDGET_INSTANCE_INFO_ID)
    }

    /// Require the widget inspect info, panics if not present.
    pub fn req(info: WidgetInfo) -> &WidgetInstanceInfo {
        info.meta().req(&WIDGET_INSTANCE_INFO_ID)
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
pub struct PropertyCaptureInfo {
    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArg]>,
}

/// Inspector info about one of the constructor functions of a widget.
#[derive(Debug, Clone)]
pub struct WidgetConstructorInfo {
    /// Constructor function name.
    pub fn_name: &'static str,

    /// Properties captured by the constructor.
    pub captures: Box<[PropertyCaptureInfo]>,
}

/// When block setup by a widget instance.
#[derive(Clone)]
pub struct WhenInfo {
    /// When condition expression.
    pub condition_expr: &'static str,
    /// Current when condition result.
    pub condition: BoxedVar<bool>,

    /// Properties affected by this when block.
    pub properties: LinearSet<&'static str>,

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
    struct InspectPropertyNode {
        child: BoxedUiNode,
        meta: PropertyInstanceMeta,
        args: Box<[PropertyArg]>,
    }
    impl UiNode for InspectPropertyNode {
        fn init(&mut self, ctx: &mut context::WidgetContext) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "init").entered();
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut context::WidgetContext) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "deinit").entered();
            self.child.init(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "info").entered();
            if let Some(wgt) = info.meta().get_mut(&WIDGET_INSTANCE_INFO_ID) {
                wgt.properties.push(PropertyInstanceInfo {
                    meta: self.meta.clone(),
                    args: self.args.clone(),
                });
            }
            ctx.vars.with_context_var(
                WIDGET_PARENT_VAR,
                ContextVarData::fixed(&WidgetInstanceParent::Property(self.meta.property_name)),
                || {
                    self.child.info(ctx, info);
                },
            );
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "subscriptions").entered();
            self.child.subscriptions(ctx, subs);
        }

        fn event<A: event::EventUpdateArgs>(&mut self, ctx: &mut context::WidgetContext, args: &A) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "event").entered();
            self.child.event(ctx, args);
        }

        fn update(&mut self, ctx: &mut context::WidgetContext) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "update").entered();
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "measure").entered();
            self.child.measure(ctx)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> units::PxSize {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "layout").entered();
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "render").entered();
            self.child.render(ctx, frame);
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut render::FrameUpdate) {
            let _span = tracing::trace_span!("property", name = self.meta.property_name, node_mtd = "render_update").entered();
            self.child.render_update(ctx, update);
        }
    }
    InspectPropertyNode {
        child: property_node,
        meta,
        args,
    }
    .boxed()
}

/// Wraps the `widget_outermost_node` with the widget inspector metadata.
pub fn inspect_widget(widget_outermost_node: BoxedUiNode, meta: WidgetInstanceMeta, whens: Box<[WhenInfo]>) -> BoxedUiNode {
    struct InspectWidgetNode {
        child: BoxedUiNode,
        meta: WidgetInstanceMeta,
        whens: Box<[WhenInfo]>,
    }
    impl UiNode for InspectWidgetNode {
        fn init(&mut self, ctx: &mut context::WidgetContext) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "init").entered();
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut context::WidgetContext) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "deinit").entered();
            self.child.init(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "info").entered();
            info.meta().set(
                &WIDGET_INSTANCE_INFO_ID,
                WidgetInstanceInfo {
                    meta: self.meta.clone(),
                    properties: vec![],
                    whens: self.whens.clone(),
                    constructors: vec![],
                    parent_name: WIDGET_PARENT_VAR.copy(ctx),
                },
            );
            self.child.info(ctx, info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "subscriptions")
                    .entered();
            self.child.subscriptions(ctx, subs);
        }

        fn event<A: event::EventUpdateArgs>(&mut self, ctx: &mut context::WidgetContext, args: &A) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "event").entered();
            self.child.event(ctx, args);
        }

        fn update(&mut self, ctx: &mut context::WidgetContext) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "update").entered();
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "measure").entered();
            self.child.measure(ctx)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "layout").entered();
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "render").entered();
            self.child.render(ctx, frame);
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut render::FrameUpdate) {
            let _span =
                tracing::trace_span!("widget", name = self.meta.widget_name, id = ?ctx.path.widget_id(), node_mtd = "render_update")
                    .entered();
            self.child.render_update(ctx, update);
        }
    }
    InspectWidgetNode {
        child: widget_outermost_node,
        meta,
        whens,
    }
    .boxed()
}

/// Wraps the `constructor_node` with the widget constructor inspector metadata.
pub fn inspect_constructor(constructor_node: BoxedUiNode, fn_name: &'static str, captures: Box<[PropertyCaptureInfo]>) -> BoxedUiNode {
    struct InspectConstructorNode {
        child: BoxedUiNode,
        fn_name: &'static str,
        captures: Box<[PropertyCaptureInfo]>,
    }
    impl UiNode for InspectConstructorNode {
        fn init(&mut self, ctx: &mut context::WidgetContext) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "init").entered();
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut context::WidgetContext) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "deinit").entered();
            self.child.deinit(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "info").entered();

            if let Some(wgt) = info.meta().get_mut(&WIDGET_INSTANCE_INFO_ID) {
                wgt.constructors.push(WidgetConstructorInfo {
                    fn_name: self.fn_name,
                    captures: self.captures,
                });
            }
            ctx.vars.with_context_var(
                WIDGET_PARENT_VAR,
                ContextVarData::fixed(&WidgetInstanceParent::Constructor(self.fn_name)),
                || {
                    self.child.info(ctx, info);
                },
            );
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "subscriptions").entered();
            self.child.subscriptions(ctx, subs);
        }

        fn event<A: event::EventUpdateArgs>(&mut self, ctx: &mut context::WidgetContext, args: &A) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "event").entered();
            self.child.event(ctx, args);
        }

        fn update(&mut self, ctx: &mut context::WidgetContext) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "update").entered();
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "measure").entered();
            self.child.measure(ctx)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "layout").entered();
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut render::FrameBuilder) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "render").entered();
            self.child.render(ctx, frame);
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut render::FrameUpdate) {
            let _scope = tracing::trace_span!("constructor", name = %self.fn_name, node_mtd = "render_update").entered();
            self.child.render_update(ctx, update);
        }
    }
    InspectConstructorNode {
        child: constructor_node,
        fn_name,
        captures,
    }
    .boxed()
}

context_var! {
    static WIDGET_PARENT_VAR: WidgetInstanceParent = WidgetInstanceParent::Root;
}
