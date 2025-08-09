use std::{any::Any, sync::Arc};

use zng_app::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
    widget::{
        WIDGET, WidgetUpdateMode,
        info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
        node::{ArcNode, IntoUiNode, UiNode, UiNodeImpl},
    },
};
use zng_app_context::LocalContext;
use zng_unit::PxSize;
use zng_var::{IntoValue, IntoVar, Var, VarValue};

use crate::{HOT_RELOAD, HOT_RELOAD_EVENT};

trait Arg: Any + Send {
    fn clone_boxed(&self) -> Box<dyn Arg>;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}
impl<T: VarValue> Arg for Var<T> {
    fn clone_boxed(&self) -> Box<dyn Arg> {
        Box::new(self.clone())
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
#[derive(Clone)]
struct ValueArg<T>(T);
impl<T: Clone + Send + Any> Arg for ValueArg<T> {
    fn clone_boxed(&self) -> Box<dyn Arg> {
        Box::new(self.clone())
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
impl Arg for ArcNode {
    fn clone_boxed(&self) -> Box<dyn Arg> {
        Box::new(self.clone())
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Arguments for hot node.
#[doc(hidden)]
pub struct HotNodeArgs {
    args: Vec<Box<dyn Arg>>,
}
impl HotNodeArgs {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            args: Vec::with_capacity(capacity),
        }
    }

    pub fn push_var<T: VarValue>(&mut self, arg: impl IntoVar<T>) {
        let arg = arg.into_var();
        self.args.push(Box::new(arg));
    }

    pub fn push_value<T: VarValue>(&mut self, arg: impl IntoValue<T>) {
        let arg = ValueArg(arg.into());
        self.args.push(Box::new(arg))
    }

    pub fn push_ui_node(&mut self, arg: impl IntoUiNode) {
        let arg = ArcNode::new(arg.into_node());
        self.args.push(Box::new(arg))
    }

    pub fn push_clone<T: Clone + Send + Any>(&mut self, arg: T) {
        let arg = ValueArg(arg);
        self.args.push(Box::new(arg));
    }

    fn pop_downcast<T: Any>(&mut self) -> T {
        *self.args.pop().unwrap().into_any().downcast().unwrap()
    }

    pub fn pop_var<T: VarValue>(&mut self) -> Var<T> {
        self.pop_downcast()
    }

    pub fn pop_value<T: VarValue>(&mut self) -> T {
        self.pop_downcast::<ValueArg<T>>().0
    }

    pub fn pop_ui_node(&mut self) -> UiNode {
        self.pop_downcast::<ArcNode>().take_on_init()
    }

    pub fn pop_clone<T: Clone + Send + Any>(&mut self) -> T {
        self.pop_downcast::<ValueArg<T>>().0
    }
}
impl Clone for HotNodeArgs {
    fn clone(&self) -> Self {
        let mut r = Self { args: vec![] };
        r.clone_from(self);
        r
    }

    fn clone_from(&mut self, source: &Self) {
        self.args.clear();
        self.args.reserve(source.args.len());
        for a in &source.args {
            self.args.push(a.clone_boxed());
        }
    }
}

/// Hot node host, dynamically re-inits the widget when the library rebuilds.
///
/// Captures and propagates the `LocalContext` because `static` variables are not the same
/// in the dynamically loaded library.
#[doc(hidden)]
pub struct HotNodeHost {
    manifest_dir: &'static str,
    name: &'static str,
    args: HotNodeArgs,
    fallback: fn(HotNodeArgs) -> HotNode,
    instance: HotNode,
}
impl HotNodeHost {
    pub fn new(manifest_dir: &'static str, name: &'static str, args: HotNodeArgs, fallback: fn(HotNodeArgs) -> HotNode) -> UiNode {
        UiNode::new(Self {
            manifest_dir,
            name,
            args,
            fallback,
            instance: HotNode::new(UiNode::nil()),
        })
    }
}
impl UiNodeImpl for HotNodeHost {
    fn children_len(&self) -> usize {
        self.instance.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        
    }

    fn init(&mut self) {
        WIDGET.sub_event(&HOT_RELOAD_EVENT);

        let mut ctx = LocalContext::capture();

        self.instance = match HOT_RELOAD.lib(self.manifest_dir) {
            Some(lib) => match lib.instantiate(self.name, &mut ctx, self.args.clone()) {
                Some(ok) => {
                    tracing::debug!("loaded hot `{}` in `{}`", self.name, WIDGET.trace_id());
                    ok
                }
                None => {
                    tracing::error!("hot node `{}` not found in `{}` library", self.name, self.manifest_dir);
                    (self.fallback)(self.args.clone())
                }
            },
            None => {
                tracing::debug!("hot lib `{}` not loaded yet", self.manifest_dir);
                (self.fallback)(self.args.clone())
            }
        };

        self.instance.init(&mut ctx);
    }

    fn deinit(&mut self) {
        let mut ctx = LocalContext::capture();
        self.instance.deinit(&mut ctx);
        self.instance = HotNode::new(UiNode::nil());
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        let mut ctx = LocalContext::capture();
        self.instance.info(&mut ctx, info);
    }

    fn event(&mut self, update: &EventUpdate) {
        let mut ctx = LocalContext::capture();
        self.instance.event(&mut ctx, update);

        if let Some(args) = HOT_RELOAD_EVENT.on(update) {
            if args.lib.manifest_dir() == self.manifest_dir {
                WIDGET.reinit();
                tracing::debug!("reinit `{}` to hot reload `{}`", WIDGET.trace_id(), self.name);
            }
        }
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        let mut ctx = LocalContext::capture();
        self.instance.update(&mut ctx, updates);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        let mut ctx = LocalContext::capture();
        self.instance.measure(&mut ctx, wm)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let mut ctx = LocalContext::capture();
        self.instance.layout(&mut ctx, wl)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        let mut ctx = LocalContext::capture();
        self.instance.render(&mut ctx, frame)
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        let mut ctx = LocalContext::capture();
        self.instance.render_update(&mut ctx, update)
    }
}

/// Hot loaded node.
#[doc(hidden)]
pub struct HotNode {
    child: UiNode,
    api: HotNodeApi,
    // keep alive because `child` is code from it.
    pub(crate) _lib: Option<Arc<libloading::Library>>,
}
impl HotNode {
    pub fn new(node: impl IntoUiNode) -> Self {
        Self {
            child: node.into_node(),
            api: HotNodeApi::capture(),
            _lib: None,
        }
    }

    fn children_len(&self) -> usize {
        (self.api.children_len)(&self.child)
    }

    fn init(&mut self, ctx: &mut LocalContext) {
        (self.api.init)(&mut self.child, ctx)
    }

    fn deinit(&mut self, ctx: &mut LocalContext) {
        (self.api.deinit)(&mut self.child, ctx)
    }

    fn info(&mut self, ctx: &mut LocalContext, info: &mut WidgetInfoBuilder) {
        (self.api.info)(&mut self.child, ctx, info)
    }

    fn event(&mut self, ctx: &mut LocalContext, update: &EventUpdate) {
        (self.api.event)(&mut self.child, ctx, update)
    }

    fn update(&mut self, ctx: &mut LocalContext, updates: &WidgetUpdates) {
        (self.api.update)(&mut self.child, ctx, updates)
    }

    fn measure(&mut self, ctx: &mut LocalContext, wm: &mut WidgetMeasure) -> PxSize {
        (self.api.measure)(&mut self.child, ctx, wm)
    }

    fn layout(&mut self, ctx: &mut LocalContext, wl: &mut WidgetLayout) -> PxSize {
        (self.api.layout)(&mut self.child, ctx, wl)
    }

    fn render(&mut self, ctx: &mut LocalContext, frame: &mut FrameBuilder) {
        (self.api.render)(&mut self.child, ctx, frame)
    }

    fn render_update(&mut self, ctx: &mut LocalContext, update: &mut FrameUpdate) {
        (self.api.render_update)(&mut self.child, ctx, update)
    }
}

// HotNode "methods" references from the dynamic loaded code to be called from the static code.
struct HotNodeApi {
    // !!: TODO update to new UiNode API
    children_len: fn(&UiNode) -> usize,
    init: fn(&mut UiNode, &mut LocalContext),
    deinit: fn(&mut UiNode, &mut LocalContext),
    info: fn(&mut UiNode, &mut LocalContext, &mut WidgetInfoBuilder),
    event: fn(&mut UiNode, &mut LocalContext, &EventUpdate),
    update: fn(&mut UiNode, &mut LocalContext, &WidgetUpdates),
    measure: fn(&mut UiNode, &mut LocalContext, &mut WidgetMeasure) -> PxSize,
    layout: fn(&mut UiNode, &mut LocalContext, &mut WidgetLayout) -> PxSize,
    render: fn(&mut UiNode, &mut LocalContext, &mut FrameBuilder),
    render_update: fn(&mut UiNode, &mut LocalContext, &mut FrameUpdate),
}
impl HotNodeApi {
    fn children_len(child: &UiNode) -> usize {
        child.children_len()
    }

    fn init(child: &mut UiNode, ctx: &mut LocalContext) {
        ctx.with_context(|| child.init())
    }

    fn deinit(child: &mut UiNode, ctx: &mut LocalContext) {
        ctx.with_context(|| child.deinit())
    }

    fn info(child: &mut UiNode, ctx: &mut LocalContext, info: &mut WidgetInfoBuilder) {
        ctx.with_context(|| child.info(info))
    }

    fn event(child: &mut UiNode, ctx: &mut LocalContext, update: &EventUpdate) {
        ctx.with_context(|| child.event(update))
    }

    fn update(child: &mut UiNode, ctx: &mut LocalContext, updates: &WidgetUpdates) {
        ctx.with_context(|| child.update(updates))
    }

    fn measure(child: &mut UiNode, ctx: &mut LocalContext, wm: &mut WidgetMeasure) -> PxSize {
        ctx.with_context(|| child.measure(wm))
    }

    fn layout(child: &mut UiNode, ctx: &mut LocalContext, wl: &mut WidgetLayout) -> PxSize {
        ctx.with_context(|| child.layout(wl))
    }

    fn render(child: &mut UiNode, ctx: &mut LocalContext, frame: &mut FrameBuilder) {
        ctx.with_context(|| child.render(frame))
    }

    fn render_update(child: &mut UiNode, ctx: &mut LocalContext, update: &mut FrameUpdate) {
        ctx.with_context(|| child.render_update(update))
    }

    fn capture() -> Self {
        Self {
            children_len: Self::children_len,
            init: Self::init,
            deinit: Self::deinit,
            info: Self::info,
            event: Self::event,
            update: Self::update,
            measure: Self::measure,
            layout: Self::layout,
            render: Self::render,
            render_update: Self::render_update,
            
        }
    }
}
