use std::{fmt, rc::Rc};

use linear_map::LinearMap;
pub use zero_ui_view_api::MonitorInfo;

use crate::{
    app::{
        raw_events::{RawMonitorsChangedArgs, RawMonitorsChangedEvent, RawScaleFactorChangedEvent},
        view_process::ViewProcess,
    },
    context::AppContext,
    event::{event, EventUpdateArgs, Events},
    event_args,
    service::Service,
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
/// Windows are positioned on a *virtual screen* that overlaps all monitors, but for the window start
/// position the user may want to select a specific monitor, this service is used to provide information
/// to implement this feature.
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
/// [`ppi`]: MonitorFullInfo::ppi
/// [`LayoutMetrics`]: crate::context::LayoutMetrics
/// [The Virtual Screen]: https://docs.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen
/// [`WindowManager`]: crate::window::WindowManager
#[derive(Service)]
pub struct Monitors {
    monitors: LinearMap<MonitorId, MonitorFullInfo>,
}
impl Monitors {
    /// Initial PPI of monitors, `96.0`.
    pub const DEFAULT_PPI: f32 = 96.0;

    pub(super) fn new(view: Option<&mut ViewProcess>) -> Self {
        Monitors {
            monitors: view
                .and_then(|v| v.available_monitors().ok())
                .map(|ms| {
                    ms.into_iter()
                        .map(|(id, info)| {
                            (
                                id,
                                MonitorFullInfo {
                                    id,
                                    info,
                                    ppi: var(Self::DEFAULT_PPI),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Reference the primary monitor.
    ///
    /// Returns `None` if no monitor was identified as the primary.
    pub fn primary_monitor(&mut self) -> Option<&MonitorFullInfo> {
        self.monitors.values().find(|m| m.info.is_primary)
    }

    /// Reference the monitor info.
    ///
    /// Returns `None` if the monitor was not found or the app is running in headless mode without renderer.
    pub fn monitor(&mut self, monitor_id: MonitorId) -> Option<&MonitorFullInfo> {
        self.monitors.get(&monitor_id)
    }

    pub(super) fn monitor_mut(&mut self, monitor_id: MonitorId) -> Option<&mut MonitorFullInfo> {
        self.monitors.get_mut(&monitor_id)
    }

    /// Iterate over all available monitors.
    ///
    /// The list entries change only when a [`MonitorsChangedEvent`] happens, the scale_factor
    /// of a entry can change TODO.
    ///
    /// Is empty if no monitor was found or the app is running in headless mode without renderer.
    pub fn available_monitors(&mut self) -> impl Iterator<Item = &MonitorFullInfo> {
        self.monitors.values()
    }

    fn on_monitors_changed(&mut self, events: &mut Events, args: &RawMonitorsChangedArgs) {
        let ms: LinearMap<_, _> = args.available_monitors.iter().cloned().collect();
        let removed: Vec<_> = self.monitors.keys().filter(|k| !ms.contains_key(k)).copied().collect();
        let added: Vec<_> = ms.keys().filter(|k| !self.monitors.contains_key(k)).copied().collect();

        for key in &removed {
            self.monitors.remove(key);
        }
        for key in &added {
            self.monitors.insert(
                *key,
                MonitorFullInfo {
                    id: *key,
                    info: ms.get(key).cloned().unwrap(),
                    ppi: var(Monitors::DEFAULT_PPI),
                },
            );
        }

        if !removed.is_empty() || !added.is_empty() {
            let args = MonitorsChangedArgs::new(args.timestamp, removed, added);
            MonitorsChangedEvent.notify(events, args);
        }
    }

    pub(super) fn on_pre_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawScaleFactorChangedEvent.update(args) {
            if let Some(m) = ctx.services.monitors().monitor_mut(args.monitor_id) {
                m.info.scale_factor = args.scale_factor.0;
            }
        } else if let Some(args) = RawMonitorsChangedEvent.update(args) {
            ctx.services.monitors().on_monitors_changed(ctx.events, args);
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
    /// `1.0` by default.
    pub scale_factor: Factor,

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
            write!(f, "({}, ({}, {}))", self.scale_factor, self.size.width, self.size.height)
        }
    }
}
impl HeadlessMonitor {
    /// New with custom size at `1.0` scale.
    #[inline]
    pub fn new(size: DipSize) -> Self {
        Self::new_scaled(size, 1.0.fct())
    }

    /// New with custom size and scale.
    #[inline]
    pub fn new_scaled(size: DipSize, scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor,
            size,
            ppi: Monitors::DEFAULT_PPI,
        }
    }

    /// New with default size `1920x1080` and custom scale.
    #[inline]
    pub fn new_scale(scale_factor: Factor) -> Self {
        HeadlessMonitor {
            scale_factor,
            ..Self::default()
        }
    }
}
impl Default for HeadlessMonitor {
    /// New `1920x1080` at `1.0` scale.
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
pub struct MonitorFullInfo {
    /// Unique ID.
    pub id: MonitorId,
    /// Metadata from the operating system.
    pub info: MonitorInfo,
    /// PPI config var.
    pub ppi: RcVar<f32>,
}
impl fmt::Debug for MonitorFullInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitorFullInfo")
            .field("id", &self.id)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

/// A *selector* that returns a [`MonitorFullInfo`] from [`Monitors`].
#[derive(Clone)]
pub enum MonitorQuery {
    /// The primary monitor, if there is any monitor.
    Primary,
    /// Custom query closure.
    ///
    /// If the closure returns `None` the primary monitor is used, if there is any.
    Query(Rc<dyn Fn(&mut Monitors) -> Option<&MonitorFullInfo>>),
}
impl MonitorQuery {
    /// New query.
    #[inline]
    pub fn new(query: impl Fn(&mut Monitors) -> Option<&MonitorFullInfo> + 'static) -> Self {
        Self::Query(Rc::new(query))
    }

    /// Runs the query.
    #[inline]
    pub fn select<'a, 'b>(&'a self, monitors: &'b mut Monitors) -> Option<&'b MonitorFullInfo> {
        match self {
            MonitorQuery::Primary => monitors.primary_monitor(),
            MonitorQuery::Query(q) => q(monitors),
        }
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
    /// [`MonitorsChangedEvent`] args.
    pub struct MonitorsChangedArgs {
        /// Removed monitors.
        pub removed: Vec<MonitorId>,

        /// Added monitors.
        ///
        /// Use the [`Monitors`] service to get metadata about the added monitors.
        pub added: Vec<MonitorId>,

        ..

        /// Concerns every widget.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

event! {
    /// Monitors added or removed event.
    pub MonitorsChangedEvent: MonitorsChangedArgs;
}
