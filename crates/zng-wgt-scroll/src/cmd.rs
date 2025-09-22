//! Commands that control the scoped scroll widget.
//!
//! The scroll widget implements all of this commands scoped to its widget ID.

use super::*;
use zng_app::event::{CommandArgs, CommandParam};
use zng_ext_window::WINDOWS;
use zng_wgt::ICONS;

command! {
    /// Represents the **scroll up** by one [`v_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`v_line_unit`]: fn@crate::v_line_unit
    pub static SCROLL_UP_CMD = {
        l10n!: true,
        name: "Scroll Up",
        info: "Scroll Up by one scroll unit",
        shortcut: shortcut!(ArrowUp),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **scroll down** by one [`v_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`v_line_unit`]: fn@crate::v_line_unit
    pub static SCROLL_DOWN_CMD = {
        l10n!: true,
        name: "Scroll Down",
        info: "Scroll Down by one scroll unit",
        shortcut: shortcut!(ArrowDown),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **scroll left** by one [`h_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`h_line_unit`]: fn@crate::h_line_unit
    pub static SCROLL_LEFT_CMD = {
        l10n!: true,
        name: "Scroll Left",
        info: "Scroll Left by one scroll unit",
        shortcut: shortcut!(ArrowLeft),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **scroll right** by one [`h_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`h_line_unit`]: fn@crate::h_line_unit
    pub static SCROLL_RIGHT_CMD = {
        l10n!: true,
        name: "Scroll Right",
        info: "Scroll Right by one scroll unit",
        shortcut: shortcut!(ArrowRight),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **page up** by one [`v_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`v_page_unit`]: fn@crate::v_page_unit
    pub static PAGE_UP_CMD = {
        l10n!: true,
        name: "Page Up",
        info: "Scroll Up by one page unit",
        shortcut: shortcut!(PageUp),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **page down** by one [`v_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`v_page_unit`]: fn@crate::v_page_unit
    pub static PAGE_DOWN_CMD = {
        l10n!: true,
        name: "Page Down",
        info: "Scroll down by one page unit",
        shortcut: shortcut!(PageDown),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **page left** by one [`h_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`h_page_unit`]: fn@crate::h_page_unit
    pub static PAGE_LEFT_CMD = {
        l10n!: true,
        name: "Page Left",
        info: "Scroll Left by one page unit",
        shortcut: shortcut!(SHIFT + PageUp),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **page right** by one [`h_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that contains more configurations.
    ///
    /// [`h_page_unit`]: fn@crate::h_page_unit
    pub static PAGE_RIGHT_CMD = {
        l10n!: true,
        name: "Page Right",
        info: "Scroll Right by one page unit",
        shortcut: shortcut!(SHIFT + PageDown),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **scroll to top** action.
    pub static SCROLL_TO_TOP_CMD = {
        l10n!: true,
        name: "Scroll to Top",
        info: "Scroll up to the content top",
        shortcut: [shortcut!(Home), shortcut!(CTRL + Home)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get(["scroll-top", "vertical-align-top"])),
    };

    /// Represents the **scroll to bottom** action.
    pub static SCROLL_TO_BOTTOM_CMD = {
        l10n!: true,
        name: "Scroll to Bottom",
        info: "Scroll down to the content bottom.",
        shortcut: [shortcut!(End), shortcut!(CTRL + End)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get(["scroll-bottom", "vertical-align-bottom"])),
    };

    /// Represents the **scroll to leftmost** action.
    pub static SCROLL_TO_LEFTMOST_CMD = {
        l10n!: true,
        name: "Scroll to Leftmost",
        info: "Scroll left to the content left edge",
        shortcut: [shortcut!(SHIFT + Home), shortcut!(CTRL | SHIFT + Home)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **scroll to rightmost** action.
    pub static SCROLL_TO_RIGHTMOST_CMD = {
        l10n!: true,
        name: "Scroll to Rightmost",
        info: "Scroll right to the content right edge",
        shortcut: [shortcut!(SHIFT + End), shortcut!(CTRL | SHIFT + End)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the action of scrolling until a child widget is fully visible, the command can
    /// also adjust the zoom scale.
    ///
    /// # Metadata
    ///
    /// This command initializes with no extra metadata.
    ///
    /// # Parameter
    ///
    /// This command requires a parameter to work, it can be a [`ScrollToRequest`] instance, or a
    /// [`ScrollToTarget`], or the [`WidgetId`] of a descendant of the scroll, or a [`Rect`] resolved in the scrollable space.
    ///
    /// You can use the [`scroll_to`] function to invoke this command in all parent scrolls automatically.
    ///
    /// [`WidgetId`]: zng_wgt::prelude::WidgetId
    /// [`Rect`]: zng_wgt::prelude::Rect
    pub static SCROLL_TO_CMD;

    /// Represents the **zoom in** action.
    ///
    /// # Parameter
    ///
    /// This commands accepts an optional [`Point`] parameter that defines the origin of the
    /// scale transform, relative values are resolved in the viewport space. The default value
    /// is *top-start*.
    ///
    /// [`Point`]: zng_wgt::prelude::Point
    pub static ZOOM_IN_CMD = {
        l10n!: true,
        name: "Zoom In",
        shortcut: shortcut!(CTRL + '+'),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("zoom-in")),
    };

    /// Represents the **zoom out** action.
    ///
    /// # Parameter
    ///
    /// This commands accepts an optional [`Point`] parameter that defines the origin of the
    /// scale transform, relative values are resolved in the viewport space. The default value
    /// is *top-start*.
    ///
    /// [`Point`]: zng_wgt::prelude::Point
    pub static ZOOM_OUT_CMD = {
        l10n!: true,
        name: "Zoom Out",
        shortcut: shortcut!(CTRL + '-'),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("zoom-out")),
    };

    /// Represents the **zoom to fit** action.
    ///
    /// The content is scaled to fit the viewport, the equivalent to `ImageFit::Contain`.
    ///
    /// # Parameter
    ///
    /// This command accepts an optional [`ZoomToFitRequest`] parameter with configuration.
    pub static ZOOM_TO_FIT_CMD = {
        l10n!: true,
        name: "Zoom to Fit",
        shortcut: shortcut!(CTRL + '0'),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get(["zoom-to-fit", "fit-screen"])),
    };

    /// Represents the **reset zoom** action.
    ///
    /// The content is scaled back to 100%, without adjusting the scroll.
    pub static ZOOM_RESET_CMD = {
        l10n!: true,
        name: "Reset Zoom",
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the **auto scroll** toggle.
    ///
    /// # Parameter
    ///
    /// The parameter can be a [`DipVector`] that starts auto scrolling at the direction and velocity (dip/s). If
    /// no parameter is provided the default speed is zero, which stops auto scrolling.
    ///
    /// [`DipVector`]: zng_wgt::prelude::DipVector
    pub static AUTO_SCROLL_CMD;
}

/// Parameters for the [`ZOOM_TO_FIT_CMD`].
///
/// Also see the property [`zoom_to_fit_mode`].
#[derive(Default, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ZoomToFitRequest {
    /// Apply the change immediately, no easing/smooth animation.
    pub skip_animation: bool,
}
impl ZoomToFitRequest {
    /// Pack the request into a command parameter.
    pub fn to_param(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_param(p: &CommandParam) -> Option<Self> {
        p.downcast_ref::<Self>().cloned()
    }

    /// Extract a clone of the request from [`CommandArgs::param`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event.
    ///
    /// [`CommandArgs::param`]: zng_app::event::CommandArgs
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.param {
            if args.propagation().is_stopped() {
                None
            } else {
                Self::from_param(p)
            }
        } else {
            None
        }
    }
}

/// Parameters for the scroll and page commands.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ScrollRequest {
    /// If the [alt factor] should be applied to the base scroll unit when scrolling.
    ///
    /// [alt factor]: super::ALT_FACTOR_VAR
    pub alternate: bool,
    /// Only scroll within this inclusive range. The range is normalized `0.0..=1.0`, the default is `(f32::MIN, f32::MAX)`.
    ///
    /// Note that the commands are enabled and disabled for the full range, this parameter controls
    /// the range for the request only.
    pub clamp: (f32, f32),

    /// Apply the change immediately, no easing/smooth animation.
    pub skip_animation: bool,
}
impl Default for ScrollRequest {
    fn default() -> Self {
        Self {
            alternate: Default::default(),
            clamp: (f32::MIN, f32::MAX),
            skip_animation: false,
        }
    }
}
impl ScrollRequest {
    /// Pack the request into a command parameter.
    pub fn to_param(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_param(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            p.downcast_ref::<bool>().map(|&alt| ScrollRequest {
                alternate: alt,
                ..Default::default()
            })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::param`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event.
    ///
    /// [`CommandArgs::param`]: zng_app::event::CommandArgs
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.param {
            if args.propagation().is_stopped() {
                None
            } else {
                Self::from_param(p)
            }
        } else {
            None
        }
    }
}
impl_from_and_into_var! {
    fn from(alternate: bool) -> ScrollRequest {
        ScrollRequest {
            alternate,
            ..Default::default()
        }
    }
}

/// Target for the [`SCROLL_TO_CMD`].
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollToTarget {
    /// Widget (inner bounds) that will be scrolled into view.
    Descendant(WidgetId),
    /// Rectangle in the content space that will be scrolled into view.
    Rect(Rect),
}
impl_from_and_into_var! {
    fn from(widget_id: WidgetId) -> ScrollToTarget {
        ScrollToTarget::Descendant(widget_id)
    }
    fn from(widget_id: &'static str) -> ScrollToTarget {
        ScrollToTarget::Descendant(widget_id.into())
    }
    fn from(rect: Rect) -> ScrollToTarget {
        ScrollToTarget::Rect(rect)
    }
}

/// Parameters for the [`SCROLL_TO_CMD`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ScrollToRequest {
    /// Area that will be scrolled into view.
    pub target: ScrollToTarget,

    /// How much the scroll position will change to showcase the target widget.
    pub mode: ScrollToMode,

    /// Optional zoom scale target.
    ///
    /// If set the offsets and scale will animate so that the `mode`
    /// is fulfilled when this zoom factor is reached. If not set the scroll will happen in
    /// the current zoom scale.
    ///
    /// Note that the viewport size can change due to a scrollbar visibility changing, this size
    /// change is not accounted for when calculating minimal.
    pub zoom: Option<Factor>,

    /// If should scroll immediately to the target, no smooth animation.
    pub skip_animation: bool,
}
impl ScrollToRequest {
    /// New with target and mode.
    pub fn new(target: impl Into<ScrollToTarget>, mode: impl Into<ScrollToMode>) -> Self {
        Self {
            target: target.into(),
            mode: mode.into(),
            zoom: None,
            skip_animation: false,
        }
    }

    /// Pack the request into a command parameter.
    pub fn to_param(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_param(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            Some(ScrollToRequest {
                target: if let Some(target) = p.downcast_ref::<ScrollToTarget>() {
                    target.clone()
                } else if let Some(target) = p.downcast_ref::<WidgetId>() {
                    ScrollToTarget::Descendant(*target)
                } else if let Some(target) = p.downcast_ref::<Rect>() {
                    ScrollToTarget::Rect(target.clone())
                } else {
                    return None;
                },
                mode: ScrollToMode::default(),
                zoom: None,
                skip_animation: false,
            })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::param`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event and the command was enabled when it was send.
    ///
    /// [`CommandArgs::param`]: zng_app::event::CommandArgs
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.param {
            if !args.enabled || args.propagation().is_stopped() {
                None
            } else {
                Self::from_param(p)
            }
        } else {
            None
        }
    }
}

/// Defines how much the [`SCROLL_TO_CMD`] will scroll to showcase the target widget.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScrollToMode {
    /// Scroll will change only just enough so that the widget inner rect is fully visible with the optional
    /// extra margin offsets.
    Minimal {
        /// Extra margin added so that the widget is touching the scroll edge.
        margin: SideOffsets,
    },
    /// Scroll so that the point relative to the widget inner rectangle is at the same screen point on
    /// the scroll viewport.
    Center {
        /// A point relative to the target widget inner size.
        widget_point: Point,
        /// A point relative to the scroll viewport.
        scroll_point: Point,
    },
}
impl ScrollToMode {
    /// New [`Minimal`] mode.
    ///
    /// [`Minimal`]: Self::Minimal
    pub fn minimal(margin: impl Into<SideOffsets>) -> Self {
        ScrollToMode::Minimal { margin: margin.into() }
    }

    /// New [`Minimal`] mode.
    ///
    /// The minimal scroll needed so that `rect` in the content widget is fully visible.
    ///
    /// [`Minimal`]: Self::Minimal
    pub fn minimal_rect(rect: impl Into<Rect>) -> Self {
        let rect = rect.into();
        ScrollToMode::Minimal {
            margin: SideOffsets::new(
                -rect.origin.y.clone(),
                rect.origin.x.clone() + rect.size.width - 100.pct(),
                rect.origin.y + rect.size.height - 100.pct(),
                -rect.origin.x,
            ),
        }
    }

    /// New [`Center`] mode using the center points of widget and scroll.
    ///
    /// [`Center`]: Self::Center
    pub fn center() -> Self {
        Self::center_points(Point::center(), Point::center())
    }

    /// New [`Center`] mode.
    ///
    /// [`Center`]: Self::Center
    pub fn center_points(widget_point: impl Into<Point>, scroll_point: impl Into<Point>) -> Self {
        ScrollToMode::Center {
            widget_point: widget_point.into(),
            scroll_point: scroll_point.into(),
        }
    }
}
impl Default for ScrollToMode {
    /// Minimal with margin 10.
    fn default() -> Self {
        Self::minimal(10)
    }
}
impl_from_and_into_var! {
    fn from(some: ScrollToMode) -> Option<ScrollToMode>;
}

/// Scroll all parent [`is_scroll`] widgets of `target` so that it becomes visible.
///
/// This function is a helper for searching for the `target` in all windows and sending [`SCROLL_TO_CMD`] for all required scroll widgets.
/// Does nothing if the `target` is not found.
///
/// [`is_scroll`]: WidgetInfoExt::is_scroll
pub fn scroll_to(target: impl ScrollToTargetProvider, mode: impl Into<ScrollToMode>) {
    scroll_to_impl(target.find_target(), mode.into(), None)
}

/// Like [`scroll_to`], but also adjusts the zoom scale.
pub fn scroll_to_zoom(target: impl ScrollToTargetProvider, mode: impl Into<ScrollToMode>, zoom: impl Into<Factor>) {
    scroll_to_impl(target.find_target(), mode.into(), Some(zoom.into()))
}

fn scroll_to_impl(target: Option<WidgetInfo>, mode: ScrollToMode, zoom: Option<Factor>) {
    if let Some(target) = target {
        let mut t = target.id();
        for a in target.ancestors() {
            if a.is_scroll() {
                SCROLL_TO_CMD.scoped(a.id()).notify_param(ScrollToRequest {
                    target: ScrollToTarget::Descendant(t),
                    mode: mode.clone(),
                    zoom,
                    skip_animation: false,
                });
                t = a.id();
            }
        }
    }
}

/// Scroll at the direction and velocity (dip/sec) until the end or another auto scroll request.
///
/// Zero stops auto scrolling.
pub fn auto_scroll(scroll_id: impl Into<WidgetId>, velocity: DipVector) {
    auto_scroll_impl(scroll_id.into(), velocity)
}
fn auto_scroll_impl(scroll_id: WidgetId, vel: DipVector) {
    AUTO_SCROLL_CMD.scoped(scroll_id).notify_param(vel);
}

/// Provides a target for scroll-to command methods.
///
/// Implemented for `"widget-id"`, `WidgetId` and `WidgetInfo`.
pub trait ScrollToTargetProvider {
    /// Find the target info.
    fn find_target(self) -> Option<WidgetInfo>;
}
impl ScrollToTargetProvider for &'static str {
    fn find_target(self) -> Option<WidgetInfo> {
        WidgetId::named(self).find_target()
    }
}
impl ScrollToTargetProvider for WidgetId {
    fn find_target(self) -> Option<WidgetInfo> {
        WINDOWS.widget_info(self)
    }
}
impl ScrollToTargetProvider for WidgetInfo {
    fn find_target(self) -> Option<WidgetInfo> {
        Some(self)
    }
}
