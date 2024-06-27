use std::time::Duration;

use zng_ext_config::{AnyConfig as _, ConfigKey, ConfigStatus, CONFIG};
use zng_ext_window::{
    AutoSize, FrameCaptureMode, MonitorQuery, WINDOW_Ext as _, WindowButton, WindowIcon, WindowLoadingHandle, WindowState, WindowVars,
    MONITORS, WINDOW_LOAD_EVENT,
};
use zng_wgt::prelude::*;

use serde::{Deserialize, Serialize};

use super::Window;

fn bind_window_var<T, V>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> V + Send + 'static) -> impl UiNode
where
    T: VarValue + PartialEq,
    V: Var<T>,
{
    #[cfg(feature = "dyn_closure")]
    let select: Box<dyn Fn(&WindowVars) -> V + Send> = Box::new(select);
    bind_window_var_impl(child.cfg_boxed(), user_var.into_var(), select).cfg_boxed()
}
fn bind_window_var_impl<T, V>(
    child: impl UiNode,
    user_var: impl IntoVar<T>,
    select: impl Fn(&WindowVars) -> V + Send + 'static,
) -> impl UiNode
where
    T: VarValue + PartialEq,
    V: Var<T>,
{
    let user_var = user_var.into_var();

    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let window_var = select(&WINDOW.vars());
            if !user_var.capabilities().is_always_static() {
                let binding = user_var.bind_bidi(&window_var);
                WIDGET.push_var_handles(binding);
            }
            window_var.set_from(&user_var).unwrap();
        }
    })
}

// Properties that set the full value.
macro_rules! set_properties {
    ($(
        $ident:ident: $Type:ty,
    )+) => {
        $(paste::paste! {
            #[doc = "Binds the [`"$ident "`](fn@WindowVars::"$ident ") window var with the property value."]
            ///
            /// The binding is bidirectional and the window variable is assigned on init.
            #[property(CONTEXT, widget_impl(Window))]
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

    font_size: Length,

    chrome: bool,
    icon: WindowIcon,
    title: Txt,

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

    enabled_buttons: WindowButton,
}

macro_rules! map_properties {
    ($(
        $ident:ident . $member:ident = $name:ident : $Type:ty,
    )+) => {$(paste::paste! {
        #[doc = "Binds the `"$member "` of the [`"$ident "`](fn@WindowVars::"$ident ") window var with the property value."]
        ///
        /// The binding is bidirectional and the window variable is assigned on init.
        #[property(CONTEXT, widget_impl(Window))]
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

/// Window clear color.
///
/// Color used to clear the previous frame pixels before rendering a new frame.
/// It is visible if window content does not completely fill the content area, this
/// can happen if you do not set a background or the background is semi-transparent, also
/// can happen during very fast resizes.
#[property(CONTEXT, default(colors::WHITE), widget_impl(Window))]
pub fn clear_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    let clear_color = color.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&clear_color);
        }
        UiNodeOp::Render { frame } => {
            frame.set_clear_color(clear_color.get());
        }
        UiNodeOp::RenderUpdate { update } => {
            update.set_clear_color(clear_color.get());
        }
        _ => {}
    })
}

/// Window or widget persistence config.
///
/// See the [`save_state`] property for more details.
///
/// [`save_state`]: fn@save_state
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SaveState {
    /// Save and restore state.
    Enabled {
        /// Config key that identifies the window or widget.
        ///
        /// If `None` a key is generated from the widget and window IDs.
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
    /// Don't save nor restore state.
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

    /// Gets the config key used for the widget identified by `id`.
    pub fn widget_key(&self, id: WidgetId) -> Option<ConfigKey> {
        match self {
            SaveState::Enabled { key, .. } => Some(key.clone().unwrap_or_else(|| {
                let name = id.name();
                if name.is_empty() {
                    formatx!("widget.sequential({}).state", id.sequential())
                } else {
                    formatx!("widget.{name}.state")
                }
            })),
            SaveState::Disabled => None,
        }
    }

    /// Get the loading timeout if it is enabled and the duration is greater than zero.
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

    /// Returns `true` if it is enabled.
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
/// If enabled a config entry is created for the window state in [`CONFIG`], and if a config backend is set
/// the window state is persisted on change and restored when the app reopens.
///
/// It is highly recommended to open the window with a named ID,
/// otherwise the state will be associated with the sequential ID of the window.
///
/// This property is enabled by default in the `Window!` widget.
///
/// [`CONFIG`]: zng_ext_config::CONFIG
#[property(CONTEXT, default(SaveState::Disabled), widget_impl(Window))]
pub fn save_state(child: impl UiNode, enabled: impl IntoValue<SaveState>) -> impl UiNode {
    let enabled = enabled.into();

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct WindowStateCfg {
        state: WindowState,
        restore_rect: euclid::Rect<f32, Dip>,
    }
    let mut cfg = None;

    struct Loading {
        cfg_status: BoxedVar<ConfigStatus>,
        _cfg_status_sub: VarHandle,
        _win_load_sub: EventHandle,
        _win_block: Option<WindowLoadingHandle>,
    }
    let mut loading = None;

    match_node(child, move |child, op| {
        let mut apply_to_window = false;
        match op {
            UiNodeOp::Init => {
                if let Some(key) = enabled.window_key(WINDOW.id()) {
                    let vars = WINDOW.vars();
                    let state = vars.state();
                    let restore_rect = vars.restore_rect();
                    WIDGET.sub_var(&vars.state()).sub_var(&vars.restore_rect());

                    let cfg_status = CONFIG.status();
                    if !cfg_status.get().is_idle() {
                        // if status updates before the WINDOW_LOAD_EVENT we will still apply
                        loading = Some(Box::new(Loading {
                            _cfg_status_sub: cfg_status.subscribe(UpdateOp::Update, WIDGET.id()),
                            _win_block: enabled.loading_timeout().and_then(|t| WINDOW.loading_handle(t)),
                            _win_load_sub: WINDOW_LOAD_EVENT.subscribe(WIDGET.id()),
                            cfg_status,
                        }))
                    } else {
                        apply_to_window = CONFIG.contains_key(key.clone()).get();
                    }

                    cfg = Some(CONFIG.get(key, || WindowStateCfg {
                        state: state.get(),
                        restore_rect: restore_rect.get().cast(),
                    }));
                }
            }
            UiNodeOp::Deinit => {
                loading = None;
                cfg = None;
            }
            UiNodeOp::Event { update } => {
                child.event(update);
                if WINDOW_LOAD_EVENT.has(update) {
                    loading = None;
                }
            }
            UiNodeOp::Update { .. } => {
                if let Some(l) = &loading {
                    if l.cfg_status.get().is_idle() {
                        if let Some(key) = enabled.window_key(WINDOW.id()) {
                            apply_to_window = CONFIG.contains_key(key).get();
                        }
                        loading = None
                    }
                }
                if enabled.is_enabled() {
                    let vars = WINDOW.vars();
                    if vars.state().is_new() || vars.restore_rect().is_new() {
                        let _ = cfg.as_ref().unwrap().set(WindowStateCfg {
                            state: vars.state().get(),
                            restore_rect: vars.restore_rect().get().cast(),
                        });
                    }
                }
            }
            _ => {}
        }
        if apply_to_window {
            let vars = WINDOW.vars();
            let cfg = cfg.as_ref().unwrap().get();

            vars.state().set(cfg.state);

            let restore_rect: DipRect = cfg.restore_rect.cast();
            let visible = MONITORS.available_monitors().iter().any(|m| m.dip_rect().intersects(&restore_rect));
            if visible {
                vars.position().set(restore_rect.origin);
            }
            vars.size().set(restore_rect.size);
        }
    })
}

/// Defines if a widget load affects the parent window load.
///
/// Widgets that support this behavior have a `block_window_load` property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockWindowLoad {
    /// Widget requests a [`WindowLoadingHandle`] and retains it until the widget is loaded.
    ///
    /// [`WindowLoadingHandle`]: zng_ext_window::WindowLoadingHandle
    Enabled {
        /// Handle expiration deadline, if the widget takes longer than this deadline the window loads anyway.
        deadline: Deadline,
    },
    /// Widget does not hold back window load.
    Disabled,
}
impl BlockWindowLoad {
    /// Enabled value.
    pub fn enabled(deadline: impl Into<Deadline>) -> BlockWindowLoad {
        BlockWindowLoad::Enabled { deadline: deadline.into() }
    }

    /// Returns `true` if it is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// Returns `true` if it is disabled.
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Returns the block deadline if it is enabled and the deadline has not expired.
    pub fn deadline(self) -> Option<Deadline> {
        match self {
            BlockWindowLoad::Enabled { deadline } => {
                if deadline.has_elapsed() {
                    None
                } else {
                    Some(deadline)
                }
            }
            BlockWindowLoad::Disabled => None,
        }
    }
}
impl_from_and_into_var! {
    /// Converts `true` to `BlockWindowLoad::enabled(1.secs())` and `false` to `BlockWindowLoad::Disabled`.
    fn from(enabled: bool) -> BlockWindowLoad {
        if enabled {
            BlockWindowLoad::enabled(1.secs())
        } else {
            BlockWindowLoad::Disabled
        }
    }

    /// Converts to enabled with the duration timeout.
    fn from(enabled_timeout: Duration) -> BlockWindowLoad {
        BlockWindowLoad::enabled(enabled_timeout)
    }
}
