//! Window stand-alone properties.
//!
//! These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
//! but you can also use then stand-alone. They configure the window from any widget inside the window.

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use crate::core::config::{Config, ConfigKey};
use crate::core::text::formatx;
use crate::core::window::{
    AutoSize, FrameCaptureMode, MonitorQuery, WindowChrome, WindowCloseRequestedEvent, WindowIcon, WindowId, WindowState, WindowVars,
};
use crate::prelude::new_property::*;
use serde::{Deserialize, Serialize};

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

/// Window persistence config.
///
/// See the [`save_state`] property for more details.
///
/// [`save_state`]: fn@save_state
#[derive(Clone, Debug)]
pub enum SaveState {
    /// Save & restore state.
    Enabled {
        /// Config key that identifies the window.
        ///
        /// If `None` a key is generated for the window, using the [`state_key`] method.
        ///
        /// [`window_key`]: Self::window_key
        key: Option<ConfigKey>,
        /// Maximum duration to delay the window view open awaiting the config to load.
        ///
        /// The config starts reading in parallel on init, the window opens after the first layout, if
        /// the config has not loaded on the first layout the window will await at maximum this duration from
        /// the moment the config started reading.
        ///
        /// If the config does not load in time it is ignored and the default position and state is used.
        ///
        /// This is one second by default.
        delay_open: Duration,
    },
    /// Don't save & restore state.
    Disabled,
}
impl Default for SaveState {
    /// Enabled, no key, delay 1s.
    fn default() -> Self {
        SaveState::Enabled {
            key: None,
            delay_open: 1.secs(),
        }
    }
}
impl SaveState {
    /// Default, enabled, no key, delay 1s.
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Gets the state key used for the window identified by `id`.
    pub fn state_key(&self, id: WindowId) -> Option<ConfigKey> {
        match self {
            SaveState::Enabled { key, .. } => Some(key.clone().unwrap_or_else(|| {
                let name = id.name();
                if name.is_empty() {
                    formatx!("window.id#{}.state", id.sequential())
                } else {
                    formatx!("window.{name}.state")
                }
            })),
            SaveState::Disabled => None,
        }
    }

    /// Get the `delay_open` if is enabled and the duration is greater than zero.
    pub fn delay_open(&self) -> Option<Duration> {
        match self {
            SaveState::Enabled { delay_open, .. } => {
                if *delay_open == Duration::ZERO {
                    None
                } else {
                    Some(*delay_open)
                }
            }
            SaveState::Disabled => None,
        }
    }

    /// Returns `true` if is enabled.
    pub fn is_enabled(&self) -> bool {
        match self {
            SaveState::Enabled { .. } => true,
            SaveState::Disabled => false,
        }
    }
}
impl_from_and_into_var! {
    /// Convert `true` to default config and `false` to `None`.
    fn from(persist: bool) -> SaveState {
        if persist {
            SaveState::default()
        } else {
            SaveState::Disabled
        }
    }
}

/// Save and restore the window state.
///
/// If enabled a config entry is created for the window state in [`Config`], and if a config backend is set
/// the window state is persisted and restored when the app reopens.
///
/// This property is enabled by default in the `window!` widget, it is recommended to open the window with a name if
/// the app can open more than one window.
#[property(context, allowed_in_when = false, default(SaveState::Disabled))]
pub fn save_state(child: impl UiNode, enabled: SaveState) -> impl UiNode {
    struct SaveStateNode<C> {
        child: C,
        enabled: SaveState,

        task: SaveTask,
    }
    enum SaveTask {
        None,
        Read(ResponseVar<Option<WindowStateCfg>>, Instant),
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for SaveStateNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            if let Some(key) = self.enabled.state_key(ctx.path.window_id()) {
                self.task = SaveTask::Read(Config::req(ctx.services).read(key), Instant::now());
            }
            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            match &self.task {
                SaveTask::Read(var, _) => {
                    subs.var(ctx.vars, var);
                }
                SaveTask::None => {}
            }
            if self.enabled.is_enabled() {
                subs.event(WindowCloseRequestedEvent);
            }
            self.child.subscriptions(ctx, subs);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = WindowCloseRequestedEvent.update(args) {
                self.child.event(ctx, args);

                if !args.propagation().is_stopped() {
                    if let Some(key) = self.enabled.state_key(ctx.path.window_id()) {
                        match &self.task {
                            SaveTask::None => {
                                // request write.
                                let window_vars = WindowVars::req(&ctx.window_state);
                                let cfg = WindowStateCfg {
                                    state: window_vars.state().copy(ctx.vars),
                                    rect: window_vars.restore_rect().copy(ctx.vars).cast(),
                                };

                                Config::req(ctx.services).write(key, cfg);
                            }
                            SaveTask::Read(_, _) => {
                                // closing quick, ignore
                            }
                        }
                    }
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let SaveTask::Read(var, _) = &self.task {
                if let Some(rsp) = var.rsp(ctx.vars) {
                    if let Some(s) = rsp {
                        let window_vars = WindowVars::req(&ctx.window_state);
                        window_vars.state().set_ne(ctx.vars, s.state);
                        window_vars.position().set_ne(ctx.vars, s.rect.origin.cast());
                        window_vars.size().set_ne(ctx.vars, s.rect.size.cast());
                    }
                    self.task = SaveTask::None;
                    ctx.updates.subscriptions();
                }
            }
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if let SaveTask::Read(_, start) = &self.task {
                if let Some(delay) = self.enabled.delay_open() {
                    let elapsed = start.elapsed();
                    if elapsed < delay {
                        let dur = delay - elapsed;
                        tracing::error!("TODO wait config for {dur:?}");
                    }
                }
            }

            self.child.layout(ctx, wl)
        }
    }
    SaveStateNode {
        child,
        enabled,
        task: SaveTask::None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WindowStateCfg {
    state: WindowState,
    rect: euclid::Rect<f32, Dip>,
}
