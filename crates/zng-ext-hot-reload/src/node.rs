use std::{any::Any, sync::Arc};

use zng_app::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
    widget::{
        info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
        node::{BoxedUiNode, NilUiNode, UiNode},
    },
};
use zng_app_context::LocalContext;
use zng_unit::PxSize;

use crate::HOT_LIB;

/// Arguments for hot node.
#[doc(hidden)]
#[derive(Clone)]
pub struct HotNodeArgs {
    args: Arc<Vec<Box<dyn Any + Send + Sync>>>,
}
impl HotNodeArgs {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            args: Arc::new(Vec::with_capacity(capacity)),
        }
    }
}

/// Hot node host, dynamically re-inits the widget when the library rebuilds.
///
/// Captures and propagates the `LocalContext` because `static` variables are not the same
/// in the dynamically loaded library.
#[doc(hidden)]
pub struct HotNodeHost {
    name: &'static str,
    args: HotNodeArgs,
    instance: HotNode,
}
impl HotNodeHost {
    pub fn new(name: &'static str, args: HotNodeArgs) -> Self {
        Self {
            name,
            args,
            instance: HotNode::nil(),
        }
    }
}
impl UiNode for HotNodeHost {
    fn init(&mut self) {
        self.instance = HOT_LIB.instantiate(self.name, self.args.clone());
        let mut ctx = LocalContext::capture();
        self.instance.init(&mut ctx);
    }

    fn deinit(&mut self) {
        let mut ctx = LocalContext::capture();
        self.instance.deinit(&mut ctx);
        self.instance.child = NilUiNode.boxed();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        let mut ctx = LocalContext::capture();
        self.instance.info(&mut ctx, info);
    }

    fn event(&mut self, update: &EventUpdate) {
        let mut ctx = LocalContext::capture();
        self.instance.event(&mut ctx, update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        // !!: TODO, on library reload WIDGET.reinit();

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

    fn is_widget(&self) -> bool {
        let mut ctx = LocalContext::capture();
        self.instance.is_widget(&mut ctx)
    }

    fn is_nil(&self) -> bool {
        let mut ctx = LocalContext::capture();
        self.instance.is_nil(&mut ctx)
    }

    fn with_context<R, F>(&mut self, update_mode: zng_app::widget::WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        let mut ctx = LocalContext::capture();
        let mut r = None;
        let mut f = Some(f);
        self.instance.with_context(&mut ctx, update_mode, &mut || {
            r = Some(f.take().unwrap()());
        });
        r
    }
}

/// Hot loaded node.
#[doc(hidden)]
pub struct HotNode {
    child: BoxedUiNode,
    // keep alive because `child` is code from it.
    pub(crate) _lib: Option<Arc<libloading::Library>>,
}
impl HotNode {
    pub(crate) fn nil() -> Self {
        Self {
            child: Box::new(NilUiNode),
            _lib: None,
        }
    }

    fn init(&mut self, ctx: &mut LocalContext) {
        ctx.with_context(|| self.child.init())
    }

    fn deinit(&mut self, ctx: &mut LocalContext) {
        ctx.with_context(|| self.child.deinit())
    }

    fn info(&mut self, ctx: &mut LocalContext, info: &mut WidgetInfoBuilder) {
        ctx.with_context(|| self.child.info(info))
    }

    fn event(&mut self, ctx: &mut LocalContext, update: &EventUpdate) {
        ctx.with_context(|| self.child.event(update))
    }

    fn update(&mut self, ctx: &mut LocalContext, updates: &WidgetUpdates) {
        ctx.with_context(|| self.child.update(updates))
    }

    fn measure(&mut self, ctx: &mut LocalContext, wm: &mut WidgetMeasure) -> PxSize {
        ctx.with_context(|| self.child.measure(wm))
    }

    fn layout(&mut self, ctx: &mut LocalContext, wl: &mut WidgetLayout) -> PxSize {
        ctx.with_context(|| self.child.layout(wl))
    }

    fn render(&mut self, ctx: &mut LocalContext, frame: &mut FrameBuilder) {
        ctx.with_context(|| self.child.render(frame))
    }

    fn render_update(&mut self, ctx: &mut LocalContext, update: &mut FrameUpdate) {
        ctx.with_context(|| self.child.render_update(update))
    }

    fn is_widget(&self, ctx: &mut LocalContext) -> bool {
        ctx.with_context(|| self.child.is_widget())
    }

    fn is_nil(&self, ctx: &mut LocalContext) -> bool {
        ctx.with_context(|| self.child.is_nil())
    }

    fn with_context(&mut self, ctx: &mut LocalContext, update_mode: zng_app::widget::WidgetUpdateMode, f: &mut dyn FnMut()) {
        ctx.with_context(|| {
            self.child.with_context(update_mode, f);
        })
    }
}
