use core::fmt;
use std::sync::Arc;

use zero_ui_app::event::{event, event_args};
use zero_ui_app::update::EventUpdate;
use zero_ui_app::view_process::VIEW_PROCESS_INITED_EVENT;
use zero_ui_app::view_process::raw_events::{RawMonitorsChangedArgs, RAW_SCALE_FACTOR_CHANGED_EVENT, RAW_MONITORS_CHANGED_EVENT};
use zero_ui_app::window::{MonitorId, WINDOW};
use zero_ui_app_context::app_local;
use zero_ui_layout::units::{DipSize, Factor, Ppi, PxPoint, PxSize, Dip, PxRect, DipRect};
use zero_ui_txt::Txt;
use zero_ui_unique_id::IdMap;
use zero_ui_var::{impl_from_and_into_var, ArcVar, ReadOnlyArcVar, VarValue, var, Var};
use zero_ui_view_api::window::VideoMode;

use crate::WINDOWS;

app_local! {
    pub(super) static MONITORS_SV: MonitorsService = const {
        MonitorsService {
            monitors: IdMap::new(),
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
/// ## Start Position
///
/// Windows are positioned on a *virtual screen* that overlaps all monitors, but all position configuration is done relative to
/// an specific *parent* monitor, it is important to track the parent monitor as it defines properties that affect the layout of the window,
/// this service is used to provide information to implement this feature.
///
/// See [The Virtual Screen] for more information about this in the Windows OS.
///
/// ## Full-Screen
///
/// To set a window to full-screen a monitor must be selected, by default it can be the one the window is at but
/// the users may want to select a monitor. To enter full-screen exclusive the video mode must be decided also, all video
/// modes supported by the monitor are available in the [`MonitorInfo`] value.
///
/// ## Real-Size Preview
///
/// Some apps, like image editors, may implement a feature where the user can preview the *real* dimensions of
/// the content they are editing, to accurately implement this you must known the real dimensions of the monitor screen,
/// unfortunately this information is not provided by display drivers. You can ask the user to measure their screen and
/// set the **pixel-per-inch** ratio for the screen using the [`ppi`] variable, this value is then available in the [`LayoutMetrics`]
/// for the next layout. If not set, the default is `96.0ppi`.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`], the service instance is in [`MONITORS`].
///
/// [`ppi`]: MonitorInfo::ppi
/// [`scale_factor`]: MonitorInfo::scale_factor
/// [`LayoutMetrics`]: crate::context::LayoutMetrics
/// [The Virtual Screen]: https://docs.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen
/// [`WindowManager`]: crate::window::WindowManager
pub struct MONITORS;
impl MONITORS {
    /// Get monitor info.
    ///
    /// Returns `None` if the monitor was not found or the app is running in headless mode without renderer.
    pub fn monitor(&self, monitor_id: MonitorId) -> Option<MonitorInfo> {
        MONITORS_SV.read().monitors.get(&monitor_id).cloned()
    }

    /// Iterate over all available monitors.
    ///
    /// Is empty if no monitor was found or the app is running in headless mode without renderer.
    pub fn available_monitors(&self) -> Vec<MonitorInfo> {
        MONITORS_SV.read().monitors.values().cloned().collect()
    }

    /// Gets the monitor info marked as primary.
    pub fn primary_monitor(&self) -> Option<MonitorInfo> {
        MONITORS_SV.read().monitors.values().find(|m| m.is_primary().get()).cloned()
    }
}

pub(super) struct MonitorsService {
    monitors: IdMap<MonitorId, MonitorInfo>,
}
impl MonitorsService {
    fn on_monitors_changed(&mut self, args: &RawMonitorsChangedArgs) {
        let mut available_monitors: IdMap<_, _> = args.available_monitors.iter().cloned().collect();

        let mut removed = vec![];
        let mut changed = vec![];

        self.monitors.retain(|key, value| {
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

            self.monitors.insert(id, MonitorInfo::from_gen(id, info));
        }

        if !removed.is_empty() || !added.is_empty() || !changed.is_empty() {
            let args = MonitorsChangedArgs::new(args.timestamp, args.propagation().clone(), removed, added, changed);
            MONITORS_CHANGED_EVENT.notify(args);
        }
    }

    pub(super) fn on_pre_event(update: &EventUpdate) {
        if let Some(args) = RAW_SCALE_FACTOR_CHANGED_EVENT.on(update) {
            if let Some(m) = MONITORS_SV.read().monitors.get(&args.monitor_id) {
                m.scale_factor.set(args.scale_factor);
            }
        } else if let Some(args) = RAW_MONITORS_CHANGED_EVENT.on(update) {
            MONITORS_SV.write().on_monitors_changed(args);
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let args = RawMonitorsChangedArgs::new(args.timestamp, args.propagation().clone(), args.available_monitors.clone());
            MONITORS_SV.write().on_monitors_changed(&args);
        }
    }
}

/// "Monitor" configuration used by windows in [headless mode].
///
/// [headless mode]: crate::window::WindowMode::is_headless
#[derive(Clone, Copy, PartialEq)]
pub struct HeadlessMonitor {
    /// The scale factor used for the headless layout and rendering.
    ///
    /// If set to `None`, falls-back to the [`parent`] scale-factor, or `1.0` if the headless window has not parent.
    ///
    /// `None` by default.
    ///
    /// [`parent`]: crate::window::WindowVars::parent
    pub scale_factor: Option<Factor>,

    /// Size of the imaginary monitor screen that contains the headless window.
    ///
    /// This is used to calculate relative lengths in the window size definition and is defined in
    /// layout pixels instead of device like in a real monitor info.
    ///
    /// `(1920, 1080)` by default.
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

    /// New with default size `1920x1080` and custom scale.
    pub fn new_scale(scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor: Some(scale_factor),
            ..Self::default()
        }
    }
}
impl Default for HeadlessMonitor {
    /// New `1920x1080` at `None` scale.
    fn default() -> Self {
        (1920, 1080).into()
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
    is_primary: ArcVar<bool>,
    name: ArcVar<Txt>,
    position: ArcVar<PxPoint>,
    size: ArcVar<PxSize>,
    video_modes: ArcVar<Vec<VideoMode>>,
    scale_factor: ArcVar<Factor>,
    ppi: ArcVar<Ppi>,
}
impl fmt::Debug for MonitorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitorFullInfo").field("id", &self.id).finish_non_exhaustive()
    }
}
impl MonitorInfo {
    /// New from a [`zero_ui_view_api::MonitorInfo`].
    fn from_gen(id: MonitorId, info: zero_ui_view_api::window::MonitorInfo) -> Self {
        MonitorInfo {
            id,
            is_primary: var(info.is_primary),
            name: var(info.name.to_text()),
            position: var(info.position),
            size: var(info.size),
            scale_factor: var(info.scale_factor),
            video_modes: var(info.video_modes),
            ppi: var(Ppi::default()),
        }
    }

    /// Update variables from fresh [`zero_ui_view_api::MonitorInfo`],
    /// returns if any value changed.
    fn update(&self, info: zero_ui_view_api::window::MonitorInfo) -> bool {
        fn check_set<T: VarValue + PartialEq>(var: &impl Var<T>, value: T) -> bool {
            let ne = var.with(|v| v != &value);
            var.set(value).unwrap();
            ne
        }

        check_set(&self.is_primary, info.is_primary)
            | check_set(&self.name, info.name.to_text())
            | check_set(&self.position, info.position)
            | check_set(&self.size, info.size)
            | check_set(&self.scale_factor, info.scale_factor)
            | check_set(&self.video_modes, info.video_modes)
    }

    /// Unique ID.
    pub fn id(&self) -> MonitorId {
        self.id
    }

    /// If could determine this monitor is the primary.
    pub fn is_primary(&self) -> ReadOnlyArcVar<bool> {
        self.is_primary.read_only()
    }

    /// Name of the monitor.
    pub fn name(&self) -> ReadOnlyArcVar<Txt> {
        self.name.read_only()
    }
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub fn position(&self) -> ReadOnlyArcVar<PxPoint> {
        self.position.read_only()
    }
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub fn size(&self) -> ReadOnlyArcVar<PxSize> {
        self.size.read_only()
    }

    /// Exclusive fullscreen video modes.
    pub fn video_modes(&self) -> ReadOnlyArcVar<Vec<VideoMode>> {
        self.video_modes.read_only()
    }

    /// The monitor scale factor.
    ///
    /// Can update if the user changes system settings.
    pub fn scale_factor(&self) -> ReadOnlyArcVar<Factor> {
        self.scale_factor.read_only()
    }
    /// PPI config var.
    pub fn ppi(&self) -> ArcVar<Ppi> {
        self.ppi.clone()
    }

    /// Gets the monitor area in device pixels.
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

/// A *selector* that returns a [`MonitorInfo`].
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
    #[allow(clippy::type_complexity)]
    Query(Arc<dyn Fn() -> Option<MonitorInfo> + Send + Sync>),
}
impl MonitorQuery {
    /// New query.
    pub fn new(query: impl Fn() -> Option<MonitorInfo> + Send + Sync + 'static) -> Self {
        Self::Query(Arc::new(query))
    }

    /// Runs the query.
    pub fn select(&self) -> Option<MonitorInfo> {
        match self {
            MonitorQuery::ParentOrPrimary => Self::parent_or_primary_query(),
            MonitorQuery::Primary => Self::primary_query(),
            MonitorQuery::Query(q) => q(),
        }
    }

    /// Runs the query, fallback to `Primary` and [`MonitorInfo::fallback`]
    pub fn select_fallback(&self) -> MonitorInfo {
        match self {
            MonitorQuery::ParentOrPrimary => Self::parent_or_primary_query(),
            MonitorQuery::Primary => Self::primary_query(),
            MonitorQuery::Query(q) => q().or_else(Self::primary_query),
        }
        .unwrap_or_else(MonitorInfo::fallback)
    }

    fn parent_or_primary_query() -> Option<MonitorInfo> {
        if let Some(parent) = WINDOW.vars().parent().get() {
            if let Ok(w) = WINDOWS.vars(parent) {
                return if let Some(monitor) = w.actual_monitor().get() {
                    MONITORS.monitor(monitor)
                } else {
                    w.monitor().get().select()
                };
            }
        }
        MONITORS.primary_monitor()
    }

    fn primary_query() -> Option<MonitorInfo> {
        MONITORS.primary_monitor()
    }
}
impl PartialEq for MonitorQuery {
    /// Returns `true` only if both are [`MonitorQuery::Primary`].
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Primary, Self::Primary))
    }
}
impl fmt::Debug for MonitorQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorQuery(Arc<..>)")
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
        pub changed: Vec<MonitorId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// Monitors added or removed event.
    pub static MONITORS_CHANGED_EVENT: MonitorsChangedArgs;
}
