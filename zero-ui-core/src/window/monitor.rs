use std::{fmt, rc::Rc};

use linear_map::LinearMap;

use super::VideoMode;
use crate::{
    app::{
        raw_events::{RawMonitorsChangedArgs, RAW_MONITORS_CHANGED_EVENT, RAW_SCALE_FACTOR_CHANGED_EVENT},
        view_process::VIEW_PROCESS_INITED_EVENT,
    },
    context::AppContext,
    event::{event, AnyEventArgs, EventUpdate, Events},
    event_args,
    service::Service,
    text::*,
    units::*,
    var::*,
};

unique_id_32! {
    /// Unique identifier of a monitor screen.
    pub struct MonitorId;
}
impl fmt::Debug for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("MonitorId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .finish()
        } else {
            write!(f, "MonitorId({})", self.sequential())
        }
    }
}
impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorId({})", self.sequential())
    }
}
impl MonitorId {
    /// ID of a fake monitor for cases where no monitor is available.
    pub fn fallback() -> MonitorId {
        static FALLBACK: once_cell::sync::Lazy<MonitorId> = once_cell::sync::Lazy::new(MonitorId::new_unique);
        *FALLBACK
    }
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
/// This service is provided by the [`WindowManager`].
///
/// [`ppi`]: MonitorInfo::ppi
/// [`scale_factor`]: MonitorInfo::scale_factor
/// [`LayoutMetrics`]: crate::context::LayoutMetrics
/// [The Virtual Screen]: https://docs.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen
/// [`WindowManager`]: crate::window::WindowManager
#[derive(Service)]
pub struct Monitors {
    monitors: LinearMap<MonitorId, MonitorInfo>,
}
impl Monitors {
    /// Initial PPI of monitors, `96.0`.
    pub const DEFAULT_PPI: f32 = 96.0;

    pub(super) fn new() -> Self {
        Monitors {
            monitors: LinearMap::new(),
        }
    }

    /// Reference the monitor info.
    ///
    /// Returns `None` if the monitor was not found or the app is running in headless mode without renderer.
    pub fn monitor(&self, monitor_id: MonitorId) -> Option<&MonitorInfo> {
        self.monitors.get(&monitor_id)
    }

    /// Iterate over all available monitors.
    ///
    /// Is empty if no monitor was found or the app is running in headless mode without renderer.
    pub fn available_monitors(&self) -> impl Iterator<Item = &MonitorInfo> {
        self.monitors.values()
    }

    /// Gets the monitor info marked as primary.
    pub fn primary_monitor(&self) -> Option<&MonitorInfo> {
        self.available_monitors().find(|m| m.is_primary().get())
    }

    fn on_monitors_changed(&mut self, events: &mut Events, vars: &Vars, args: &RawMonitorsChangedArgs) {
        let mut available_monitors: LinearMap<_, _> = args.available_monitors.iter().cloned().collect();

        let mut removed = vec![];
        let mut changed = vec![];

        self.monitors.retain(|key, value| {
            if let Some(new) = available_monitors.remove(key) {
                if value.update(vars, new) {
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

            self.monitors.insert(id, MonitorInfo::from_view(id, info));
        }

        if !removed.is_empty() || !added.is_empty() || !changed.is_empty() {
            let args = MonitorsChangedArgs::new(args.timestamp, args.propagation().clone(), removed, added, changed);
            MONITORS_CHANGED_EVENT.notify(events, args);
        }
    }

    pub(super) fn on_pre_event(ctx: &mut AppContext, update: &mut EventUpdate) {
        if let Some(args) = RAW_SCALE_FACTOR_CHANGED_EVENT.on(update) {
            if let Some(m) = Monitors::req(ctx.services).monitor(args.monitor_id) {
                m.scale_factor.set_ne(ctx.vars, args.scale_factor).unwrap();
            }
        } else if let Some(args) = RAW_MONITORS_CHANGED_EVENT.on(update) {
            Monitors::req(ctx.services).on_monitors_changed(ctx.events, ctx.vars, args);
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let args = RawMonitorsChangedArgs::new(args.timestamp, args.propagation().clone(), args.available_monitors.clone());
            Monitors::req(ctx.services).on_monitors_changed(ctx.events, ctx.vars, &args);
        }
    }
}

/// "Monitor" configuration used by windows in [headless mode].
///
/// [headless mode]: crate::window::WindowMode::is_headless
#[derive(Clone, Copy)]
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
    ///
    /// [`Monitors::DEFAULT_PPI`] by default.
    pub ppi: f32,
}
impl fmt::Debug for HeadlessMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || about_eq(self.ppi, Monitors::DEFAULT_PPI, 0.001) {
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
            ppi: Monitors::DEFAULT_PPI,
        }
    }

    /// New with custom size and scale.
    pub fn new_scaled(size: DipSize, scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor: Some(scale_factor),
            size,
            ppi: Monitors::DEFAULT_PPI,
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
    fn from<W: Into<Dip> + Clone, H: Into<Dip> + Clone>((width, height): (W, H)) -> HeadlessMonitor {
        HeadlessMonitor::new(DipSize::new(width.into(), height.into()))
    }
    fn from<W: Into<Dip> + Clone, H: Into<Dip> + Clone, F: Into<Factor> + Clone>((width, height, scale): (W, H, F)) -> HeadlessMonitor {
        HeadlessMonitor::new_scaled(DipSize::new(width.into(), height.into()), scale.into())
    }
}

/// All information about a monitor that [`Monitors`] can provide.
#[derive(Clone)]
pub struct MonitorInfo {
    id: MonitorId,
    is_primary: RcVar<bool>,
    name: RcVar<Text>,
    position: RcVar<PxPoint>,
    size: RcVar<PxSize>,
    video_modes: RcVar<Vec<VideoMode>>,
    scale_factor: RcVar<Factor>,
    ppi: RcVar<f32>,
}
impl fmt::Debug for MonitorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitorFullInfo").field("id", &self.id).finish_non_exhaustive()
    }
}
impl MonitorInfo {
    /// New from a [`zero_ui_view_api::MonitorInfo`].
    fn from_view(id: MonitorId, info: zero_ui_view_api::MonitorInfo) -> Self {
        MonitorInfo {
            id,
            is_primary: var(info.is_primary),
            name: var(info.name.to_text()),
            position: var(info.position),
            size: var(info.size),
            scale_factor: var(info.scale_factor.fct()),
            video_modes: var(info.video_modes),
            ppi: var(Monitors::DEFAULT_PPI),
        }
    }

    /// Update variables from fresh [`zero_ui_view_api::MonitorInfo`],
    /// returns if any value changed.
    fn update(&self, vars: &Vars, info: zero_ui_view_api::MonitorInfo) -> bool {
        fn check_set<T: VarValue + PartialEq>(vars: &Vars, var: &impl Var<T>, value: T) -> bool {
            let ne = var.with(|v| v != &value);
            var.set_ne(vars, value).unwrap();
            ne
        }

        check_set(vars, &self.is_primary, info.is_primary)
            | check_set(vars, &self.name, info.name.to_text())
            | check_set(vars, &self.position, info.position)
            | check_set(vars, &self.size, info.size)
            | check_set(vars, &self.scale_factor, info.scale_factor.fct())
            | check_set(vars, &self.video_modes, info.video_modes)
    }

    /// Unique ID.
    pub fn id(&self) -> MonitorId {
        self.id
    }

    /// If could determine this monitor is the primary.
    pub fn is_primary(&self) -> ReadOnlyRcVar<bool> {
        self.is_primary.read_only()
    }

    /// Name of the monitor.
    pub fn name(&self) -> ReadOnlyRcVar<Text> {
        self.name.read_only()
    }
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub fn position(&self) -> ReadOnlyRcVar<PxPoint> {
        self.position.read_only()
    }
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub fn size(&self) -> ReadOnlyRcVar<PxSize> {
        self.size.read_only()
    }

    /// Exclusive fullscreen video modes.
    pub fn video_modes(&self) -> ReadOnlyRcVar<Vec<VideoMode>> {
        self.video_modes.read_only()
    }

    /// The monitor scale factor.
    ///
    /// Can update if the user changes system settings.
    pub fn scale_factor(&self) -> ReadOnlyRcVar<Factor> {
        self.scale_factor.read_only()
    }
    /// PPI config var.
    pub fn ppi(&self) -> RcVar<f32> {
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

        PxRect::new(pos, size).to_dip(factor.0)
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
            size: var(defaults.size.to_px(fct.0)),
            video_modes: var(vec![]),
            scale_factor: var(fct),
            ppi: var(Monitors::DEFAULT_PPI),
        }
    }
}

/// A *selector* that returns a [`MonitorInfo`] from [`Monitors`].
#[derive(Clone)]
pub enum MonitorQuery {
    /// The primary monitor, if there is any monitor.
    Primary,
    /// Custom query closure.
    ///
    /// If the closure returns `None` the primary monitor is used, if there is any.
    #[allow(clippy::type_complexity)]
    Query(Rc<dyn Fn(&mut Monitors) -> Option<&MonitorInfo>>),
}
impl MonitorQuery {
    /// New query.
    pub fn new(query: impl Fn(&mut Monitors) -> Option<&MonitorInfo> + 'static) -> Self {
        Self::Query(Rc::new(query))
    }

    /// Runs the query.
    pub fn select<'a, 'm>(&'a self, monitors: &'m mut Monitors) -> Option<&'m MonitorInfo> {
        match self {
            MonitorQuery::Primary => Self::primary_query(monitors),
            MonitorQuery::Query(q) => q(monitors),
        }
    }

    /// Runs the query, fallback to `Primary` and [`MonitorInfo::fallback`]
    pub fn select_fallback(&self, monitors: &mut Monitors) -> MonitorInfo {
        match self {
            MonitorQuery::Primary => Self::primary_query(monitors).cloned(),
            MonitorQuery::Query(q) => q(monitors).cloned().or_else(|| Self::primary_query(monitors).cloned()),
        }
        .unwrap_or_else(MonitorInfo::fallback)
    }

    fn primary_query(m: &mut Monitors) -> Option<&MonitorInfo> {
        m.available_monitors().find(|m| m.is_primary.get())
    }
}
impl PartialEq for MonitorQuery {
    /// Returns `true` only if both are [`MonitorQuery::Primary`].
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Primary, Self::Primary))
    }
}
impl Default for MonitorQuery {
    /// Returns [`MonitorQuery::Primary`].
    fn default() -> Self {
        Self::Primary
    }
}
impl fmt::Debug for MonitorQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorQuery(Rc<..>)")
    }
}

event_args! {
    /// [`MONITORS_CHANGED_EVENT`] args.
    pub struct MonitorsChangedArgs {
        /// Removed monitors.
        pub removed: Vec<MonitorId>,

        /// Added monitors.
        ///
        /// Use the [`Monitors`] service to get metadata about the added monitors.
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
