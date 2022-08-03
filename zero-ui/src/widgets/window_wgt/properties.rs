//! Window stand-alone properties.
//!
//! These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
//! but you can also use then stand-alone. They configure the window from any widget inside the window.

use std::marker::PhantomData;

use crate::core::window::{AutoSize, FrameCaptureMode, MonitorQuery, WindowChrome, WindowIcon, WindowId, WindowState, WindowVars};
use crate::prelude::new_property::*;

fn bind_window_var<T, V>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> V + 'static) -> impl UiNode
where
    T: VarValue + PartialEq,
    V: Var<T>,
{
    struct BindWindowVarNode<C, V, S, T> {
        _p: PhantomData<T>,
        child: C,
        user_var: V,
        select: S,
        binding: Option<VarBindingHandle>,
    }

    #[impl_ui_node(child)]
    impl<T, C, V, SV, S> UiNode for BindWindowVarNode<C, V, S, T>
    where
        T: VarValue + PartialEq,
        C: UiNode,
        V: Var<T>,
        SV: Var<T>,
        S: Fn(&WindowVars) -> SV + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let window_var = (self.select)(WindowVars::req(ctx));
            if self.user_var.can_update() {
                self.binding = Some(self.user_var.bind_bidi(ctx.vars, &window_var));
            }
            window_var.set_ne(ctx.vars, self.user_var.get_clone(ctx.vars)).unwrap();
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.binding = None;
            self.child.deinit(ctx);
        }
    }
    BindWindowVarNode {
        _p: PhantomData,
        child,
        user_var: user_var.into_var(),
        select,
        binding: None,
    }
}

// Properties that set the full value.
macro_rules! set_properties {
    ($(
        $ident:ident: $Type:ty,
    )+) => {
        $(paste::paste! {
            #[doc = "Binds the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
            ///
            /// The binding is bidirectional and the window variable is assigned on init.
            #[property(context)]
            pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                bind_window_var(child, $ident, |w|w.$ident().clone())
            }
        })+
    }
}
set_properties! {
    position: Point,
    monitor: MonitorQuery,

    state: WindowState,

    size: Size,
    min_size: Size,
    max_size: Size,

    chrome: WindowChrome,
    icon: WindowIcon,
    title: Text,

    auto_size: AutoSize,
    auto_size_origin: Point,

    resizable: bool,
    movable: bool,

    always_on_top: bool,

    visible: bool,
    taskbar_visible: bool,

    parent: Option<WindowId>,
    modal: bool,

    frame_capture_mode: FrameCaptureMode,
}

macro_rules! map_properties {
            ($(
                $ident:ident . $member:ident = $name:ident : $Type:ty,
            )+) => {$(paste::paste! {
                #[doc = "Binds the `"$member "` of the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
                ///
                /// The binding is bidirectional and the window variable is assigned on init.
                #[property(context)]
                pub fn $name(child: impl UiNode, $name: impl IntoVar<$Type>) -> impl UiNode {
                    bind_window_var(child, $name, |w|w.$ident().map_ref_bidi(|v| &v.$member, |v|&mut v.$member))
                }
            })+}
        }
map_properties! {
    position.x = x: Length,
    position.y = y: Length,
    size.width = width: Length,
    size.height = height: Length,
    min_size.width = min_width: Length,
    min_size.height = min_height: Length,
    max_size.width = max_width: Length,
    max_size.height = max_height: Length,
}

/// Sets the frame clear color.
#[property(context, default(colors::WHITE))]
pub fn clear_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    struct ClearColorNode<U, C> {
        child: U,
        clear_color: C,
    }
    #[impl_ui_node(child)]
    impl<U: UiNode, C: Var<Rgba>> UiNode for ClearColorNode<U, C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.clear_color);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.clear_color.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx);
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.set_clear_color(self.clear_color.copy(ctx).into());
            self.child.render(ctx, frame);
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.set_clear_color(self.clear_color.copy(ctx).into());
            self.child.render_update(ctx, update);
        }
    }
    ClearColorNode {
        child,
        clear_color: color.into_var(),
    }
}

// TODO read-only properties.
