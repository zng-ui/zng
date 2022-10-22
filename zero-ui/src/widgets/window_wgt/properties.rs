//! Window stand-alone properties.
//!
//! These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
//! but you can also use then stand-alone. They configure the window from any widget inside the window.

use std::marker::PhantomData;
use std::time::Duration;

use crate::core::color::ColorScheme;
use crate::core::config::{Config, ConfigKey};
use crate::core::text::formatx;
use crate::core::window::{
    AutoSize, FrameCaptureMode, MonitorQuery, Monitors, WindowChrome, WindowIcon, WindowId, WindowLoadingHandle, WindowState, WindowVars,
    Windows, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT,
};
use crate::prelude::new_property::*;
use serde::{Deserialize, Serialize};
use zero_ui_core::window::{HeadlessMonitor, StartPosition};

fn bind_window_var<T, V>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> V + 'static) -> impl UiNode
where
    T: VarValue + PartialEq,
    V: Var<T>,
{
    #[ui_node(struct BindWindowVarNode<T: VarValue + PartialEq, SV: Var<T>> {
        _t: PhantomData<T>,
        child: impl UiNode,
        user_var: impl Var<T>,
        select: impl Fn(&WindowVars) -> SV + 'static,
    })]
    impl UiNode for BindWindowVarNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let window_var = (self.select)(WindowVars::req(ctx));
            if !self.user_var.capabilities().is_always_static() {
                let binding = self.user_var.bind_bidi(&window_var);
                ctx.handles.push_vars(binding);
            }
            window_var.set_ne(ctx.vars, self.user_var.get()).unwrap();
            self.child.init(ctx);
        }
    }
    BindWindowVarNode {
        _t: PhantomData,
        child,
        user_var: user_var.into_var(),
        select,
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

    color_scheme: Option<ColorScheme>,

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
    #[ui_node(struct ClearColorNode {
        child: impl UiNode,
        #[var] clear_color: impl Var<Rgba>,
    })]
    impl UiNode for ClearColorNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.clear_color.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx, updates);
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.set_clear_color(self.clear_color.get().into());
            self.child.render(ctx, frame);
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.set_clear_color(self.clear_color.get().into());
            self.child.render_update(ctx, update);
        }
    }
    ClearColorNode {
        child,
        clear_color: color.into_var(),
    }
}

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
        /// If `None` a key is generated for the window, using the [`window_key`] method.
        ///
        /// [`window_key`]: Self::window_key
        key: Option<ConfigKey>,
        /// Maximum time to keep the window in the loading state awaiting for the config to load.
        ///
        /// If the config fails to load in this time frame the window is opened in it's default state.
        ///
        /// This is one second by default.
        loading_timeout: Duration,
    },
    /// Don't save & restore state.
    Disabled,
}
impl Default for SaveState {
    /// Enabled, no key, delay 1s.
    fn default() -> Self {
        SaveState::Enabled {
            key: None,
            loading_timeout: 1.secs(),
        }
    }
}
impl SaveState {
    /// Default, enabled, no key, delay 1s.
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Gets the config key used for the window identified by `id`.
    pub fn window_key(&self, id: WindowId) -> Option<ConfigKey> {
        match self {
            SaveState::Enabled { key, .. } => Some(key.clone().unwrap_or_else(|| {
                let name = id.name();
                if name.is_empty() {
                    formatx!("window.sequential({}).state", id.sequential())
                } else {
                    formatx!("window.{name}.state")
                }
            })),
            SaveState::Disabled => None,
        }
    }

    /// Get the `loading_timeout` if is enabled and the duration is greater than zero.
    pub fn loading_timeout(&self) -> Option<Duration> {
        match self {
            SaveState::Enabled { loading_timeout, .. } => {
                if *loading_timeout == Duration::ZERO {
                    None
                } else {
                    Some(*loading_timeout)
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
#[property(context, default(SaveState::Disabled))]
pub fn save_state(child: impl UiNode, enabled: impl IntoValue<SaveState>) -> impl UiNode {
    enum Task {
        None,
        Read {
            rsp: ResponseVar<Option<WindowStateCfg>>,
            #[allow(dead_code)] // hold handle alive
            loading: Option<WindowLoadingHandle>,
        },
    }

    #[ui_node(struct SaveStateNode {
        child: impl UiNode,
        enabled: SaveState,
        handles: Option<[EventHandle; 2]>,

        task: Task,
    })]
    impl UiNode for SaveStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            if let Some(key) = self.enabled.window_key(ctx.path.window_id()) {
                let rsp = Config::req(ctx.services).read(key);
                let loading = self
                    .enabled
                    .loading_timeout()
                    .and_then(|t| Windows::req(ctx.services).loading_handle(ctx.path.window_id(), t));
                rsp.subscribe(ctx.path.widget_id()).perm();
                self.task = Task::Read { rsp, loading };
            }

            if self.enabled.is_enabled() {
                self.handles = Some([
                    WINDOW_CLOSE_REQUESTED_EVENT.subscribe(ctx.path.widget_id()),
                    WINDOW_LOAD_EVENT.subscribe(ctx.path.widget_id()),
                ]);
            }

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.handles = None;
            self.child.deinit(ctx);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);
            if WINDOW_LOAD_EVENT.has(update) {
                self.task = Task::None;
            } else if let Some(args) = WINDOW_CLOSE_REQUESTED_EVENT.on(update) {
                if !args.propagation().is_stopped() {
                    if let Some(key) = self.enabled.window_key(ctx.path.window_id()) {
                        match &self.task {
                            Task::None => {
                                // request write.
                                let window_vars = WindowVars::req(&ctx.window_state);
                                let cfg = WindowStateCfg {
                                    state: window_vars.state().get(),
                                    restore_rect: window_vars.restore_rect().get().cast(),
                                };

                                Config::req(ctx.services).write(key, cfg);
                            }
                            Task::Read { .. } => {
                                // closing quick, ignore
                            }
                        }
                    }
                }
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Task::Read { rsp, .. } = &mut self.task {
                if let Some(rsp) = rsp.rsp() {
                    if let Some(s) = rsp {
                        let window_vars = WindowVars::req(&ctx.window_state);
                        window_vars.state().set_ne(ctx.vars, s.state);
                        let restore_rect: DipRect = s.restore_rect.cast();

                        let visible = Monitors::req(ctx.services)
                            .available_monitors()
                            .any(|m| m.dip_rect().intersects(&restore_rect));
                        if visible {
                            window_vars.position().set_ne(ctx.vars, restore_rect.origin);
                        }

                        window_vars.size().set_ne(ctx.vars, restore_rect.size);
                    }
                    self.task = Task::None;
                }
            }
            self.child.update(ctx, updates);
        }
    }
    SaveStateNode {
        child,
        enabled,
        handles: None,
        task: Task::None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WindowStateCfg {
    state: WindowState,
    restore_rect: euclid::Rect<f32, Dip>,
}

/// Window position when it opens.
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(StartPosition::Default))]
pub fn start_position(child: impl UiNode, position: impl IntoValue<StartPosition>) -> impl UiNode {
    let _ = position;
    tracing::error!("property `start_position` must be captured");
    child
}

/// If the Inspector can be opened for this window.
///
/// The default value is `true`, but only applies if built with the `inspector` feature.
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[cfg(inspector)]
#[property(context, default(true))]
pub fn can_inspect(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `can_inspect` must be captured");
    child
}

/// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
///
/// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
/// is taken from the monitor. In headless mode these values can be configured manually.
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(HeadlessMonitor::default()))]
pub fn headless_monitor(child: impl UiNode, enabled: impl IntoValue<HeadlessMonitor>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `headless_monitor` must be captured");
    child
}

/// If the window is forced to be the foreground keyboard focus after opening.
///
/// By default the windows manager decides if the window will receive focus after opening, usually it is focused
/// only if the process that started the window already has focus. Setting the property to `true` ensures that focus
/// is moved to the new window, potentially stealing the focus from other apps and disrupting the user.
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(false))]
pub fn start_focused(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `start_focused` must be captured");
    child
}

/// Lock-in kiosk mode.
///
/// In kiosk mode the only window states allowed are full-screen or full-screen exclusive, and
/// all subsequent windows opened are child of the kiosk window.
///
/// Note that this does not configure the windows manager,
/// you still need to setup a kiosk environment, it does not block `ALT+TAB`. This just stops the
/// app itself from accidentally exiting kiosk mode.
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(false))]
pub fn kiosk(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `kiosk` must be captured");
    child
}

/// If semi-transparent content is "see-through", mixin with the OS pixels "behind" the window.
///
/// This is `true` by default, as it avoids the screen flashing black for windows opening in maximized or fullscreen modes
/// in the Microsoft Windows OS.
///
/// Note that to make use of this feature you must unset the [`clear_color`] and [`background_color`] or set then to
/// a semi-transparent color. The composition is a simple alpha blend, effects like blur do not apply to
/// the pixels "behind" the window.
///
/// [`clear_color`]: #wp-clear_color
/// [`background_color`]: #wp-background_color
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(true))]
pub fn allow_transparency(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `allow_transparency` must be captured");
    child
}

/// Render performance mode overwrite for this window, if set to `None` the [`Windows::default_render_mode`] is used.
///
/// # Examples
///
/// Prefer `Dedicated` renderer backend for just this window:
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// fn example(ctx: &mut WindowContext) -> Window {
///     let selected_mode = WindowVars::req(ctx).render_mode();
///     window! {
///         title = "Render Mode";
///         render_mode = RenderMode::Dedicated;
///         content = text(selected_mode.map(|m| formatx!("Preference: Dedicated\nActual: {m:?}")));
///     }
/// }
/// ```
///
/// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
/// see [`RenderMode`] for more details about each mode and fallbacks.
///
/// [`Windows::default_render_mode`]: crate::core::window::Windows::default_render_mode
///
/// # Capture Only
///
/// This property is not standalone, the value is captured on build.
#[property(context, default(None))]
pub fn render_mode(child: impl UiNode, enabled: impl IntoValue<Option<RenderMode>>) -> impl UiNode {
    let _ = enabled;
    tracing::error!("property `render_mode` must be captured");
    child
}
