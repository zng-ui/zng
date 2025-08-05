use std::time::Duration;

use zng_ext_config::{AnyConfig as _, CONFIG, ConfigKey, ConfigStatus, ConfigValue};
use zng_ext_window::{
    AutoSize, FrameCaptureMode, MONITORS, MonitorQuery, WINDOW_Ext as _, WINDOW_LOAD_EVENT, WINDOWS, WindowButton, WindowIcon,
    WindowLoadingHandle, WindowState, WindowVars,
};
use zng_var::AnyVar;
use zng_wgt::prelude::*;

use serde::{Deserialize, Serialize};
use zng_wgt_layer::adorner_fn;

use super::Window;

fn bind_window_var<T>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> Var<T> + Send + 'static) -> impl UiNode
where
    T: VarValue + PartialEq,
{
    bind_window_var_impl(child.cfg_boxed(), user_var.into_var().into(), move |vars| select(vars).into()).cfg_boxed()
}
fn bind_window_var_impl(child: impl UiNode, user_var: AnyVar, select: impl Fn(&WindowVars) -> AnyVar + Send + 'static) -> impl UiNode {
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let window_var = select(&WINDOW.vars());
            if !user_var.capabilities().is_const() {
                let binding = user_var.bind_bidi(&window_var);
                WIDGET.push_var_handles(binding);
            }
            window_var.set_from(&user_var);
        }
    })
}

// Properties that set the full value.
macro_rules! set_properties {
    ($(
        $ident:ident: $Type:ty,
    )+) => {
        $(pastey::paste! {
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
    accent_color: Option<LightDark>,

    frame_capture_mode: FrameCaptureMode,

    enabled_buttons: WindowButton,
}

macro_rules! map_properties {
    ($(
        $ident:ident . $member:ident = $name:ident : $Type:ty,
    )+) => {$(pastey::paste! {
        #[doc = "Binds the `"$member "` of the [`"$ident "`](fn@WindowVars::"$ident ") window var with the property value."]
        ///
        /// The binding is bidirectional and the window variable is assigned on init.
        #[property(CONTEXT, widget_impl(Window))]
        pub fn $name(child: impl UiNode, $name: impl IntoVar<$Type>) -> impl UiNode {
            bind_window_var(child, $name, |w|w.$ident().map_bidi_modify(|v| v.$member.clone(), |v, m|m.$member = v.clone()))
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
/// See the [`save_state_node`] for more details.
///
/// [`save_state`]: fn@save_state
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SaveState {
    /// Save and restore state.
    Enabled {
        /// Config key that identifies the window or widget.
        ///
        /// If `None` a key is generated from the widget ID and window ID name, see [`enabled_key`] for
        /// details about how key generation.
        ///
        /// [`enabled_key`]: Self::enabled_key
        key: Option<ConfigKey>,
    },
    /// Don't save nor restore state.
    Disabled,
}
impl Default for SaveState {
    /// Enabled, no key, delay 1s.
    fn default() -> Self {
        Self::enabled()
    }
}
impl SaveState {
    /// Default, enabled, no key.
    pub const fn enabled() -> Self {
        Self::Enabled { key: None }
    }

    /// Gets the config key if is enabled and can enable on the context.
    ///
    /// If is enabled without a key, the key is generated from the widget or window name:
    ///
    /// * If the widget ID has a name the key is `"wgt-{name}-state"`.
    /// * If the context is the window root or just a window and the window ID has a name the key is `"win-{name}-state"`.
    pub fn enabled_key(&self) -> Option<ConfigKey> {
        match self {
            Self::Enabled { key } => {
                if key.is_some() {
                    return key.clone();
                }
                let mut try_win = true;
                if let Some(wgt) = WIDGET.try_id() {
                    let name = wgt.name();
                    if !name.is_empty() {
                        return Some(formatx!("wgt-{name}"));
                    }
                    try_win = WIDGET.parent_id().is_none();
                }
                if try_win {
                    if let Some(win) = WINDOW.try_id() {
                        let name = win.name();
                        if !name.is_empty() {
                            return Some(formatx!("win-{name}"));
                        }
                    }
                }
                None
            }
            Self::Disabled => None,
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

/// Helper node for implementing widgets save.
///
/// The `on_load_restore` closure is called on window load or on init if the window is already loaded. The argument
/// is the saved state from a previous instance.
///
/// The `on_update_save` closure is called every update after the window loads, if it returns a value the config is updated.
/// If the argument is `true` the closure must return a value, this value is used as the CONFIG fallback value that is required
/// by some config backends even when the config is already present.
pub fn save_state_node<S: ConfigValue>(
    child: impl UiNode,
    enabled: impl IntoValue<SaveState>,
    mut on_load_restore: impl FnMut(Option<S>) + Send + 'static,
    mut on_update_save: impl FnMut(bool) -> Option<S> + Send + 'static,
) -> impl UiNode {
    let enabled = enabled.into();
    enum State<S: ConfigValue> {
        Disabled,
        AwaitingLoad,
        Loaded,
        LoadedWithCfg(Var<S>),
    }
    let mut state = State::Disabled;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            if let Some(key) = enabled.enabled_key() {
                if WINDOW.is_loaded() {
                    if CONFIG.contains_key(key.clone()).get() {
                        let cfg = CONFIG.get(key, on_update_save(true).unwrap());
                        on_load_restore(Some(cfg.get()));
                        state = State::LoadedWithCfg(cfg);
                    } else {
                        on_load_restore(None);
                        state = State::Loaded;
                    }
                } else {
                    WIDGET.sub_event(&WINDOW_LOAD_EVENT);
                    state = State::AwaitingLoad;
                }
            } else {
                state = State::Disabled;
            }
        }
        UiNodeOp::Deinit => {
            state = State::Disabled;
        }
        UiNodeOp::Event { update } => {
            if matches!(&state, State::AwaitingLoad) && WINDOW_LOAD_EVENT.has(update) {
                if let Some(key) = enabled.enabled_key() {
                    if CONFIG.contains_key(key.clone()).get() {
                        let cfg = CONFIG.get(key, on_update_save(true).unwrap());
                        on_load_restore(Some(cfg.get()));
                        state = State::LoadedWithCfg(cfg);
                    } else {
                        on_load_restore(None);
                        state = State::Loaded;
                    }
                } else {
                    // this can happen if the parent widget node is not properly implemented (changed context)
                    state = State::Disabled;
                }
            }
        }
        UiNodeOp::Update { .. } => match &mut state {
            State::LoadedWithCfg(cfg) => {
                if let Some(new) = on_update_save(false) {
                    cfg.set(new);
                }
            }
            State::Loaded => {
                if let Some(new) = on_update_save(false) {
                    if let Some(key) = enabled.enabled_key() {
                        let cfg = CONFIG.insert(key, new.clone());
                        state = State::LoadedWithCfg(cfg);
                    } else {
                        state = State::Disabled;
                    }
                }
            }
            _ => {}
        },
        _ => {}
    })
}

/// Save and restore the window state.
///
/// If enabled a config entry is created for the window state in [`CONFIG`], and if a config backend is set
/// the window state is persisted on change and restored when the app reopens.
///
/// This property is enabled by default in the `Window!` widget, without a key. Note that without a config key
/// the state only actually enables if the window root widget ID or the window ID have a name.
///
/// [`CONFIG`]: zng_ext_config::CONFIG
#[property(CONTEXT, default(SaveState::Disabled), widget_impl(Window))]
pub fn save_state(child: impl UiNode, enabled: impl IntoValue<SaveState>) -> impl UiNode {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct WindowStateCfg {
        state: WindowState,
        restore_rect: euclid::Rect<f32, Dip>,
    }
    save_state_node::<WindowStateCfg>(
        child,
        enabled,
        |cfg| {
            let vars = WINDOW.vars();
            let state = vars.state();
            WIDGET.sub_var(&state).sub_var(&vars.restore_rect());

            if let Some(cfg) = cfg {
                // restore state
                state.set(cfg.state);

                // restore normal position if it is valid (visible in a monitor)
                let restore_rect: DipRect = cfg.restore_rect.cast();
                let visible = MONITORS.available_monitors().iter().any(|m| m.dip_rect().intersects(&restore_rect));
                if visible {
                    vars.position().set(restore_rect.origin);
                }
                vars.size().set(restore_rect.size);
            }
        },
        |required| {
            let vars = WINDOW.vars();
            let state = vars.state();
            let rect = vars.restore_rect();
            if required || state.is_new() || rect.is_new() {
                Some(WindowStateCfg {
                    state: state.get(),
                    restore_rect: rect.get().cast(),
                })
            } else {
                None
            }
        },
    )
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

/// Block window load until [`CONFIG.status`] is idle.
///
/// This property is enabled by default in the `Window!` widget.
///
/// [`CONFIG.status`]: CONFIG::status
#[property(CONTEXT, default(false), widget_impl(Window))]
pub fn config_block_window_load(child: impl UiNode, enabled: impl IntoValue<BlockWindowLoad>) -> impl UiNode {
    let enabled = enabled.into();

    enum State {
        Allow,
        Block {
            _handle: WindowLoadingHandle,
            cfg: Var<ConfigStatus>,
        },
    }
    let mut state = State::Allow;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            if let Some(delay) = enabled.deadline() {
                let cfg = CONFIG.status();
                if !cfg.get().is_idle() {
                    if let Some(_handle) = WINDOW.loading_handle(delay) {
                        WIDGET.sub_var(&cfg);
                        state = State::Block { _handle, cfg };
                    }
                }
            }
        }
        UiNodeOp::Deinit => {
            state = State::Allow;
        }
        UiNodeOp::Update { .. } => {
            if let State::Block { cfg, .. } = &state {
                if cfg.get().is_idle() {
                    state = State::Allow;
                }
            }
        }
        _ => {}
    })
}

/// Gets if is not headless, [`chrome`] is `true`, [`state`] is not fullscreen but [`WINDOWS.system_chrome`]
/// reports the system does not provide window decorations.
///
/// [`chrome`]: fn@chrome
/// [`state`]: fn@state
/// [`WINDOWS.system_chrome`]: WINDOWS::system_chrome
#[property(EVENT, default(var_state()), widget_impl(Window))]
pub fn needs_fallback_chrome(child: impl UiNode, needs: impl IntoVar<bool>) -> impl UiNode {
    zng_wgt::node::bind_state_init(
        child,
        || {
            if WINDOW.mode().is_headless() {
                const_var(false)
            } else {
                let vars = WINDOW.vars();
                expr_var! {
                    *#{vars.chrome()} && #{WINDOWS.system_chrome()}.needs_custom() && !#{vars.state()}.is_fullscreen()
                }
            }
        },
        needs,
    )
}

/// Gets if [`WINDOWS.system_chrome`] prefers custom chrome.
///
/// Note that you must set [`chrome`] to `false` when using this to provide a custom chrome.
///
/// [`chrome`]: fn@chrome
/// [`WINDOWS.system_chrome`]: WINDOWS::system_chrome
#[property(EVENT, default(var_state()), widget_impl(Window))]
pub fn prefer_custom_chrome(child: impl UiNode, prefer: impl IntoVar<bool>) -> impl UiNode {
    zng_wgt::node::bind_state(child, WINDOWS.system_chrome().map(|c| c.prefer_custom), prefer)
}

/// Adorner property specific for custom chrome overlays.
///
/// This property behaves exactly like [`adorner_fn`]. Using it instead of adorner frees the adorner property
/// for other usage in the window instance or in derived window types.
///
/// Note that you can also set the `custom_chrome_padding_fn` to ensure that the content is not hidden behind the adorner.
///
/// [`adorner_fn`]: fn@adorner_fn
#[property(FILL, default(WidgetFn::nil()), widget_impl(Window))]
pub fn custom_chrome_adorner_fn(child: impl UiNode, custom_chrome: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    adorner_fn(child, custom_chrome)
}

/// Extra padding for window content in windows that display a [`custom_chrome_adorner_fn`].
///
/// [`custom_chrome_adorner_fn`]: fn@custom_chrome_adorner_fn
#[property(CHILD_LAYOUT, default(0), widget_impl(Window))]
pub fn custom_chrome_padding_fn(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    zng_wgt_container::padding(child, padding)
}
