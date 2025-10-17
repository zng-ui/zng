use core::fmt;
use std::sync::Arc;

use zng_app::APP;
use zng_app::event::{AnyEventArgs, event, event_args};
use zng_app::update::EventUpdate;
use zng_app::view_process::raw_events::{RAW_MONITORS_CHANGED_EVENT, RAW_SCALE_FACTOR_CHANGED_EVENT, RawMonitorsChangedArgs};
use zng_app::window::{MonitorId, WINDOW, WindowId};
use zng_app_context::app_local;
use zng_layout::unit::{Dip, DipRect, DipSize, DipToPx, Factor, FactorUnits, Ppi, Px, PxPoint, PxRect, PxSize, PxToDip};
use zng_txt::{ToTxt, Txt};
use zng_unique_id::IdMap;
use zng_var::{Var, VarValue, impl_from_and_into_var, var};
use zng_view_api::window::VideoMode;

use crate::{WINDOWS, WindowManager};

app_local! {
    pub(super) static MONITORS_SV: MonitorsService = {
        APP.extensions().require::<WindowManager>();
        MonitorsService {
            monitors: var(IdMap::new()),
        }
    };
}

/// Monitors service.
///
/// List monitor screens and configure the PPI of a given monitor.
///
/// # Uses
///
/// Uses of this service:
///
/// #### Start Position
///
/// Windows are positioned on a virtual screen that overlaps all monitors, but all position configuration is done relative to
/// an specific parent monitor, it is important to track the parent monitor as it defines properties that affect the layout of the window.
/// This service is used to provide information to implement this feature.
///
/// #### Fullscreen
///
/// To set a window to fullscreen a monitor must be selected, by default it can be the one the window is at but
/// the users may want to select a monitor. To enter fullscreen exclusive the video mode must also be decided, all video
/// modes supported by the monitor are available in the [`MonitorInfo`] value.
///
/// #### Real-Size Preview
///
/// Some apps, like image editors, may implement a feature where the user can preview the *real* dimensions of
/// the content they are editing, to accurately implement this you must known the real dimensions of the monitor screen,
/// unfortunately this information is not provided by display drivers. You can ask the user to measure their screen and
/// set the **pixel-per-inch** ratio for the screen using the [`ppi`] variable, this value is then available in the [`LayoutMetrics`]
/// for the next layout. If not set, the default is `96.0ppi`.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`] extension, it will panic if used in an app not extended.
///
/// [`ppi`]: MonitorInfo::ppi
/// [`scale_factor`]: MonitorInfo::scale_factor
/// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
/// [`WindowManager`]: crate::WindowManager
pub struct MONITORS;
impl MONITORS {
    /// Get monitor info.
    ///
    /// Returns `None` if the monitor was not found or the app is running in headless mode without renderer.
    pub fn monitor(&self, monitor_id: MonitorId) -> Option<MonitorInfo> {
        MONITORS_SV.read().monitors.with(|m| m.get(&monitor_id).cloned())
    }

    /// List all available monitors.
    ///
    /// Is empty if no monitor was found or the app is running in headless mode without renderer.
    pub fn available_monitors(&self) -> Var<Vec<MonitorInfo>> {
        MONITORS_SV.read().monitors.map(|w| {
            let mut list: Vec<_> = w.values().cloned().collect();
            list.sort_by(|a, b| a.name.with(|a| b.name.with(|b| a.cmp(b))));
            list
        })
    }

    /// Gets the monitor info marked as primary.
    pub fn primary_monitor(&self) -> Var<Option<MonitorInfo>> {
        MONITORS_SV
            .read()
            .monitors
            .map(|w| w.values().find(|m| m.is_primary().get()).cloned())
    }
}

pub(super) struct MonitorsService {
    monitors: Var<IdMap<MonitorId, MonitorInfo>>,
}
impl MonitorsService {
    fn on_monitors_changed(&mut self, args: &RawMonitorsChangedArgs) {
        let mut available_monitors: IdMap<_, _> = args.available_monitors.iter().cloned().collect();
        let event_ts = args.timestamp;
        let event_propagation = args.propagation().clone();

        self.monitors.modify(move |m| {
            let mut removed = vec![];
            let mut changed = vec![];

            m.retain(|key, value| {
                if let Some(new) = available_monitors.remove(key) {
                    if value.update(new) {
                        changed.push(*key);
                    }
                    true
                } else {
                    removed.push(*key);
                    false
                }
            });

            let mut added = Vec::with_capacity(available_monitors.len());

            for (id, info) in available_monitors {
                added.push(id);

                m.insert(id, MonitorInfo::from_gen(id, info));
            }

            if !removed.is_empty() || !added.is_empty() || !changed.is_empty() {
                let args = MonitorsChangedArgs::new(event_ts, event_propagation, removed, added, changed);
                MONITORS_CHANGED_EVENT.notify(args);
            }
        });
    }

    pub(super) fn on_pre_event(update: &EventUpdate) {
        if let Some(args) = RAW_SCALE_FACTOR_CHANGED_EVENT.on(update) {
            MONITORS_SV.read().monitors.with(|m| {
                if let Some(m) = m.get(&args.monitor_id) {
                    m.scale_factor.set(args.scale_factor);
                }
            });
        } else if let Some(args) = RAW_MONITORS_CHANGED_EVENT.on(update) {
            MONITORS_SV.write().on_monitors_changed(args);
        }
    }
}

/// "Monitor" configuration used by windows in [headless mode].
///
/// [headless mode]: zng_app::window::WindowMode::is_headless
#[derive(Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct HeadlessMonitor {
    /// The scale factor used for the headless layout and rendering.
    ///
    /// If set to `None`, falls back to the [`parent`] scale-factor, or `1.0` if the headless window has not parent.
    ///
    /// `None` by default.
    ///
    /// [`parent`]: crate::WindowVars::parent
    pub scale_factor: Option<Factor>,

    /// Size of the imaginary monitor screen that contains the headless window.
    ///
    /// This is used to calculate relative lengths in the window size definition and is defined in
    /// layout pixels instead of device like in a real monitor info.
    ///
    /// `(11608, 8708)` by default.
    pub size: DipSize,

    /// Pixel-per-inches used for the headless layout and rendering.
    pub ppi: Ppi,
}
impl fmt::Debug for HeadlessMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || self.ppi != Ppi::default() {
            f.debug_struct("HeadlessMonitor")
                .field("scale_factor", &self.scale_factor)
                .field("screen_size", &self.size)
                .field("ppi", &self.ppi)
                .finish()
        } else {
            write!(f, "({:?}, ({}, {}))", self.scale_factor, self.size.width, self.size.height)
        }
    }
}
impl HeadlessMonitor {
    /// New with custom size at `None` scale.
    pub fn new(size: DipSize) -> Self {
        HeadlessMonitor {
            scale_factor: None,
            size,
            ppi: Ppi::default(),
        }
    }

    /// New with custom size and scale.
    pub fn new_scaled(size: DipSize, scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor: Some(scale_factor),
            size,
            ppi: Ppi::default(),
        }
    }

    /// New with default size `(11608, 8708)` and custom scale.
    pub fn new_scale(scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor: Some(scale_factor),
            ..Self::default()
        }
    }
}
impl Default for HeadlessMonitor {
    /// New `(11608, 8708)` at `None` scale.
    fn default() -> Self {
        (11608, 8708).into()
    }
}
impl_from_and_into_var! {
    fn from<W: Into<Dip>, H: Into<Dip>>((width, height): (W, H)) -> HeadlessMonitor {
        HeadlessMonitor::new(DipSize::new(width.into(), height.into()))
    }
    fn from<W: Into<Dip>, H: Into<Dip>, F: Into<Factor>>((width, height, scale): (W, H, F)) -> HeadlessMonitor {
        HeadlessMonitor::new_scaled(DipSize::new(width.into(), height.into()), scale.into())
    }
}

/// All information about a monitor that [`MONITORS`] can provide.
#[derive(Clone)]
pub struct MonitorInfo {
    id: MonitorId,
    is_primary: Var<bool>,
    name: Var<Txt>,
    position: Var<PxPoint>,
    size: Var<PxSize>,
    video_modes: Var<Vec<VideoMode>>,
    scale_factor: Var<Factor>,
    ppi: Var<Ppi>,
}
impl fmt::Debug for MonitorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitorInfo")
            .field("id", &self.id)
            .field("name", &self.name.get())
            .field("position", &self.position.get())
            .field("size", &self.size.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for MonitorInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name.var_eq(&other.name)
    }
}
impl MonitorInfo {
    /// New from a [`zng_view_api::MonitorInfo`].
    fn from_gen(id: MonitorId, info: zng_view_api::window::MonitorInfo) -> Self {
        MonitorInfo {
            id,
            is_primary: var(info.is_primary),
            name: var(info.name.to_txt()),
            position: var(info.position),
            size: var(info.size),
            scale_factor: var(info.scale_factor),
            video_modes: var(info.video_modes),
            ppi: var(Ppi::default()),
        }
    }

    /// Update variables from fresh [`zng_view_api::MonitorInfo`],
    /// returns if any value changed.
    fn update(&self, info: zng_view_api::window::MonitorInfo) -> bool {
        fn check_set<T: VarValue + PartialEq>(var: &Var<T>, value: T) -> bool {
            let ne = var.with(|v| v != &value);
            var.set(value);
            ne
        }

        check_set(&self.is_primary, info.is_primary)
            | check_set(&self.name, info.name.to_txt())
            | check_set(&self.position, info.position)
            | check_set(&self.size, info.size)
            | check_set(&self.scale_factor, info.scale_factor)
            | check_set(&self.video_modes, info.video_modes)
    }

    /// Unique ID in this process instance.
    pub fn id(&self) -> MonitorId {
        self.id
    }

    /// If this monitor is the primary screen.
    pub fn is_primary(&self) -> Var<bool> {
        self.is_primary.read_only()
    }

    /// Name of the monitor.
    pub fn name(&self) -> Var<Txt> {
        self.name.read_only()
    }
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub fn position(&self) -> Var<PxPoint> {
        self.position.read_only()
    }
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub fn size(&self) -> Var<PxSize> {
        self.size.read_only()
    }

    /// Exclusive fullscreen video modes.
    pub fn video_modes(&self) -> Var<Vec<VideoMode>> {
        self.video_modes.read_only()
    }

    /// The monitor scale factor.
    ///
    /// Can update if the user changes system settings.
    pub fn scale_factor(&self) -> Var<Factor> {
        self.scale_factor.read_only()
    }
    /// Pixel-per-inch config var.
    pub fn ppi(&self) -> Var<Ppi> {
        self.ppi.clone()
    }

    /// Gets the monitor area in pixels.
    pub fn px_rect(&self) -> PxRect {
        let pos = self.position.get();
        let size = self.size.get();

        PxRect::new(pos, size)
    }

    /// Gets the monitor area in device independent pixels.
    pub fn dip_rect(&self) -> DipRect {
        let pos = self.position.get();
        let size = self.size.get();
        let factor = self.scale_factor.get();

        PxRect::new(pos, size).to_dip(factor)
    }

    /// Bogus metadata for the [`MonitorId::fallback`].
    ///
    /// [`MonitorId::fallback`]: crate::monitor::MonitorId::fallback
    pub fn fallback() -> Self {
        let defaults = HeadlessMonitor::default();
        let fct = 1.fct();

        Self {
            id: MonitorId::fallback(),
            is_primary: var(false),
            name: var("<fallback>".into()),
            position: var(PxPoint::zero()),
            size: var(defaults.size.to_px(fct)),
            video_modes: var(vec![]),
            scale_factor: var(fct),
            ppi: var(Ppi::default()),
        }
    }
}

/// A selector that returns a [`MonitorInfo`].
#[derive(Clone, Default)]
pub enum MonitorQuery {
    /// The parent window monitor, or `Primary` if the window has no parent.
    ///
    /// Note that the window is not moved automatically if the parent window is moved to another monitor, only
    /// after the query variable updates.
    ///
    /// This is the default value.
    #[default]
    ParentOrPrimary,

    /// The primary monitor, if there is any monitor.
    Primary,
    /// Custom query closure.
    ///
    /// If the closure returns `None` the `ParentOrPrimary` query is used, if there is any.
    ///
    /// You can use the [`MONITORS`] service in the query closure to select a monitor.
    Query(Arc<dyn Fn() -> Option<MonitorInfo> + Send + Sync>),
}
impl std::fmt::Debug for MonitorQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "MonitorQuery::")?;
        }
        match self {
            Self::ParentOrPrimary => write!(f, "ParentOrPrimary"),
            Self::Primary => write!(f, "Primary"),
            Self::Query(_) => write!(f, "Query(_)"),
        }
    }
}
impl MonitorQuery {
    /// New query.
    pub fn new(query: impl Fn() -> Option<MonitorInfo> + Send + Sync + 'static) -> Self {
        Self::Query(Arc::new(query))
    }

    /// Runs the query.
    pub fn select(&self) -> Option<MonitorInfo> {
        self.select_for(WINDOW.id())
    }
    fn select_for(&self, win_id: WindowId) -> Option<MonitorInfo> {
        match self {
            MonitorQuery::ParentOrPrimary => Self::parent_or_primary_query(win_id),
            MonitorQuery::Primary => Self::primary_query(),
            MonitorQuery::Query(q) => q(),
        }
    }

    /// Runs the query. Falls back to `Primary`, or the largest or [`MonitorInfo::fallback`].
    pub fn select_fallback(&self) -> MonitorInfo {
        match self {
            MonitorQuery::ParentOrPrimary => Self::parent_or_primary_query(WINDOW.id()),
            MonitorQuery::Primary => Self::primary_query(),
            MonitorQuery::Query(q) => q().or_else(Self::primary_query),
        }
        .unwrap_or_else(Self::fallback)
    }

    fn fallback() -> MonitorInfo {
        MONITORS_SV.read().monitors.with(|m| {
            let mut best = None;
            let mut best_area = Px(0);
            for m in m.values() {
                let m_area = m.px_rect().area();
                if m_area > best_area {
                    best = Some(m);
                    best_area = m_area;
                }
            }
            best.cloned().unwrap_or_else(MonitorInfo::fallback)
        })
    }

    fn parent_or_primary_query(win_id: WindowId) -> Option<MonitorInfo> {
        if let Some(parent) = WINDOWS.vars(win_id).unwrap().parent().get()
            && let Ok(w) = WINDOWS.vars(parent)
        {
            return if let Some(monitor) = w.actual_monitor().get() {
                MONITORS.monitor(monitor)
            } else {
                w.monitor().get().select_for(parent)
            };
        }
        MONITORS.primary_monitor().get()
    }

    fn primary_query() -> Option<MonitorInfo> {
        MONITORS.primary_monitor().get()
    }
}
impl PartialEq for MonitorQuery {
    /// Returns `true` only if both are [`MonitorQuery::Primary`].
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Primary, Self::Primary))
    }
}

event_args! {
    /// [`MONITORS_CHANGED_EVENT`] args.
    pub struct MonitorsChangedArgs {
        /// Removed monitors.
        pub removed: Vec<MonitorId>,

        /// Added monitors.
        ///
        /// Use the [`MONITORS`] service to get metadata about the added monitors.
        pub added: Vec<MonitorId>,

        /// Modified monitors.
        ///
        /// The monitor metadata is tracked using variables that are now flagged new.
        pub modified: Vec<MonitorId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// Monitors added, removed or modified event.
    pub static MONITORS_CHANGED_EVENT: MonitorsChangedArgs;
}
