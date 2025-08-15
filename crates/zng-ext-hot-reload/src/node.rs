use std::{any::Any, sync::Arc};

use zng_app::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
    widget::{
        WIDGET,
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
    #[doc(hidden)]
    pub fn new_node(manifest_dir: &'static str, name: &'static str, args: HotNodeArgs, fallback: fn(HotNodeArgs) -> HotNode) -> UiNode {
        UiNode::new(Self {
            manifest_dir,
            name,
            args,
            fallback,
            instance: HotNode::new(UiNode::nil()),
        })
    }
}
// become the node, not a wrapper
impl UiNodeImpl for HotNodeHost {
    fn children_len(&self) -> usize {
        self.instance.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.instance.with_child(index, visitor)
    }

    fn is_list(&self) -> bool {
        self.instance.is_list()
    }

    fn as_widget(&mut self) -> Option<&mut dyn zng_app::widget::node::WidgetUiNodeImpl> {
        self.instance.node.as_dyn().as_widget()
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

        self.instance.init();
    }

    fn deinit(&mut self) {
        self.instance.deinit();
        self.instance.node = UiNode::nil();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.instance.info(info);
    }

    fn event(&mut self, update: &EventUpdate) {
        self.instance.event(update);

        if let Some(args) = HOT_RELOAD_EVENT.on(update)
            && args.lib.manifest_dir() == self.manifest_dir
        {
            WIDGET.reinit();
            tracing::debug!("reinit `{}` to hot reload `{}`", WIDGET.trace_id(), self.name);
        }
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.instance.update(updates);
    }
    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn zng_app::widget::node::UiNodeListObserver) {
        self.instance.update_list(updates, observer);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.instance.measure(wm)
    }
    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.instance.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.instance.layout(wl)
    }
    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.instance.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.instance.render(frame)
    }
    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.instance.render_list(frame, render);
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.instance.render_update(update)
    }
    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        self.instance.render_update_list(update, render_update);
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.instance.for_each_child(visitor);
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.instance.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: zng_var::BoxAnyVarValue,
        fold: &(dyn Fn(zng_var::BoxAnyVarValue, usize, &mut UiNode) -> zng_var::BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(zng_var::BoxAnyVarValue, zng_var::BoxAnyVarValue) -> zng_var::BoxAnyVarValue + Sync),
    ) -> zng_var::BoxAnyVarValue {
        self.instance.par_fold_reduce(identity, fold, reduce)
    }
}

/// Hot loaded node.
#[doc(hidden)]
pub struct HotNode {
    api: HotNodeApi,
    api_m: HotNodeApiManual,
    node: UiNode,
    // keep alive because `child` is code from it.
    pub(crate) _lib: Option<Arc<libloading::Library>>,
}
impl HotNode {
    pub fn new(node: impl IntoUiNode) -> Self {
        Self {
            api: HotNodeApi::capture(),
            api_m: HotNodeApiManual::capture(),
            node: node.into_node(),
            _lib: None,
        }
    }
}

struct HotNodeApiManual {
    // stuff that cannot be defined by define_api
    children_len: fn(&UiNode, &mut LocalContext) -> usize,
    is_list: fn(&UiNode, &mut LocalContext) -> bool,
}
impl HotNodeApiManual {
    fn children_len(child: &UiNode, ctx: &mut LocalContext) -> usize {
        ctx.with_context(|| child.children_len())
    }
    fn is_list(child: &UiNode, ctx: &mut LocalContext) -> bool {
        ctx.with_context(|| child.is_list())
    }

    fn capture() -> Self {
        Self {
            children_len: Self::children_len,
            is_list: Self::is_list,
        }
    }
}
impl HotNode {
    fn children_len(&self) -> usize {
        (self.api_m.children_len)(&self.node, &mut LocalContext::capture())
    }

    fn is_list(&self) -> bool {
        (self.api_m.is_list)(&self.node, &mut LocalContext::capture())
    }
}

macro_rules! define_api {
    ($(
        fn $ident:ident($($arg:ident : $ArgTy:ty),* $(,)?) $(-> $OutTy:ty)?;
    )+) => {
        struct HotNodeApi {
            $(
                $ident: fn(&mut UiNode, &mut LocalContext $(, $ArgTy)*) $(-> $OutTy)?,
            )+
        }
        impl HotNodeApi {
            $(
                fn $ident(child: &mut UiNode, ctx: &mut LocalContext $(, $arg: $ArgTy)*) $(-> $OutTy)? {
                    ctx.with_context(|| child.as_dyn().$ident($($arg),*))
                }
            )+

            fn capture() -> Self {
                Self {
                    $($ident: Self::$ident,)+
                }
            }
        }
        impl HotNode {
            $(
                fn $ident(&mut self $(, $arg: $ArgTy)*) $(-> $OutTy)? {
                    (self.api.$ident)(&mut self.node, &mut LocalContext::capture() $(, $arg)*)
                }
            )+
        }
    };
}
#[rustfmt::skip] // for cargo zng
define_api! {
    fn init();
    fn deinit();
    fn info(info: &mut WidgetInfoBuilder);
    fn event(update: &EventUpdate);
    fn update(updates: &WidgetUpdates);
    fn update_list(updates: &WidgetUpdates, observer: &mut dyn zng_app::widget::node::UiNodeListObserver);
    fn measure(wm: &mut WidgetMeasure) -> PxSize;
    fn measure_list(
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize;
    fn layout(wl: &mut WidgetLayout) -> PxSize;
    fn layout_list(
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize;
    fn render(frame: &mut FrameBuilder);
    fn render_list(frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync));
    fn render_update(update: &mut FrameUpdate);
    fn render_update_list(update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync));

    fn with_child(index: usize, visitor: &mut dyn FnMut(&mut UiNode));
    fn for_each_child(visitor: &mut dyn FnMut(usize, &mut UiNode));
    fn par_each_child(visitor: &(dyn Fn(usize, &mut UiNode) + Sync));
    fn par_fold_reduce(
        identity: zng_var::BoxAnyVarValue,
        fold: &(dyn Fn(zng_var::BoxAnyVarValue, usize, &mut UiNode) -> zng_var::BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(zng_var::BoxAnyVarValue, zng_var::BoxAnyVarValue) -> zng_var::BoxAnyVarValue + Sync),
    ) -> zng_var::BoxAnyVarValue;
}
