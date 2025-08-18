//! Window, surface and frame types.

use std::fmt;

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

use crate::{
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    display_list::{DisplayList, FrameValueUpdate},
    image::{ImageId, ImageLoadedData, ImageMaskMode},
};
use zng_unit::{Dip, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx as _, Factor, Px, PxPoint, PxSize, PxToDip, PxTransform, Rgba};

crate::declare_id! {
    /// Window ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is an unique id that survives View crashes.
    ///
    /// The App Process defines the ID.
    pub struct WindowId(_);

    /// Monitor screen ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct MonitorId(_);

    /// Identifies a frame request for collaborative resize in [`WindowChanged`].
    ///
    /// The View Process defines the ID.
    pub struct FrameWaitId(_);
}

/// Render backend preference.
///
/// This is mostly a trade-off between performance, power consumption and cold startup time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderMode {
    /// Prefer the best dedicated GPU, probably the best performance after initialization, but also the
    /// most power consumption.
    ///
    /// Falls back to `Integrated`, then `Software`.
    Dedicated,

    /// Prefer the integrated GPU (provided by the CPU), probably the best power consumption and good performance for most GUI applications,
    /// this is the default value.
    ///
    /// Falls back to `Dedicated`, then `Software`.
    Integrated,

    /// Use a software render fallback, this has the best compatibility and best initialization time. This is probably the
    /// best pick for one frame render tasks and small windows where the initialization time of a GPU context may not offset
    /// the render time gains.
    ///
    /// If the view-process implementation has no software, falls back to `Integrated`, then `Dedicated`.
    Software,
}
impl Default for RenderMode {
    /// [`RenderMode::Integrated`].
    fn default() -> Self {
        RenderMode::Integrated
    }
}
impl RenderMode {
    /// Returns fallbacks that view-process implementers will try if `self` is not available.
    pub fn fallbacks(self) -> [RenderMode; 2] {
        use RenderMode::*;
        match self {
            Dedicated => [Integrated, Software],
            Integrated => [Dedicated, Software],
            Software => [Integrated, Dedicated],
        }
    }

    /// Returns `self` plus [`fallbacks`].
    ///
    /// [`fallbacks`]: Self::fallbacks
    pub fn with_fallbacks(self) -> [RenderMode; 3] {
        let [f0, f1] = self.fallbacks();
        [self, f0, f1]
    }
}

#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(some: RenderMode) -> Option<RenderMode>;
}

/// Configuration of a new headless surface.
///
/// Headless surfaces are always [`capture_mode`] enabled.
///
/// [`capture_mode`]: WindowRequest::capture_mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeadlessRequest {
    /// ID that will identify the new headless surface.
    ///
    /// The surface is identified by a [`WindowId`] so that some API methods
    /// can apply to both windows or surfaces, no actual window is created.
    pub id: WindowId,

    /// Scale for the layout units in this config.
    pub scale_factor: Factor,

    /// Surface area (viewport size).
    pub size: DipSize,

    /// Render mode preference for this headless surface.
    pub render_mode: RenderMode,

    /// Initial payload for API extensions.
    ///
    /// The `zng-view` crate implements this by calling `WindowExtension::configure` and `RendererExtension::configure`
    /// with the payload.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
}
impl HeadlessRequest {
    /// New request.
    pub fn new(
        id: WindowId,
        scale_factor: Factor,
        size: DipSize,
        render_mode: RenderMode,
        extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
    ) -> Self {
        Self {
            id,
            scale_factor,
            size,
            render_mode,
            extensions,
        }
    }
}

/// Information about a monitor screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MonitorInfo {
    /// Readable name of the monitor.
    pub name: Txt,
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub position: PxPoint,
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub size: PxSize,
    /// The monitor scale factor.
    pub scale_factor: Factor,
    /// Exclusive fullscreen video modes.
    pub video_modes: Vec<VideoMode>,

    /// If could determine this monitor is the primary.
    pub is_primary: bool,
}
impl MonitorInfo {
    /// New info.
    pub fn new(name: Txt, position: PxPoint, size: PxSize, scale_factor: Factor, video_modes: Vec<VideoMode>, is_primary: bool) -> Self {
        Self {
            name,
            position,
            size,
            scale_factor,
            video_modes,
            is_primary,
        }
    }

    /// Returns the `size` descaled using the `scale_factor`.
    pub fn dip_size(&self) -> DipSize {
        self.size.to_dip(self.scale_factor)
    }
}

/// Exclusive video mode info.
///
/// You can get the options for a monitor using [`MonitorInfo::video_modes`].
///
/// Note that actual system video mode is selected by approximation,
/// closest `size`, then `bit_depth`, then `refresh_rate`.
///
/// [`MonitorInfo::video_modes`]: crate::window::MonitorInfo::video_modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: PxSize,
    /// The bit depth of this video mode.
    /// This is generally 24 bits or 32 bits on modern systems,
    /// depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode, in millihertz.
    pub refresh_rate: u32,
}
impl Default for VideoMode {
    fn default() -> Self {
        Self::MAX
    }
}
impl VideoMode {
    /// New video mode.
    pub fn new(size: PxSize, bit_depth: u16, refresh_rate: u32) -> Self {
        Self {
            size,
            bit_depth,
            refresh_rate,
        }
    }

    /// Default value, matches with the largest size, greatest bit-depth and refresh rate.
    pub const MAX: VideoMode = VideoMode {
        size: PxSize::new(Px::MAX, Px::MAX),
        bit_depth: u16::MAX,
        refresh_rate: u32::MAX,
    };
}
impl fmt::Display for VideoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::MAX {
            write!(f, "MAX")
        } else {
            write!(
                f,
                "{}x{}, {}, {}hz",
                self.size.width.0,
                self.size.height.0,
                self.bit_depth,
                (self.refresh_rate as f32 * 0.001).round()
            )
        }
    }
}

/// Information about a successfully opened window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WindowOpenData {
    /// Window complete state.
    pub state: WindowStateAll,

    /// Monitor that contains the window, if any.
    pub monitor: Option<MonitorId>,

    /// Final top-left offset of the window (excluding outer chrome).
    ///
    /// The values are the global position and the position in the monitor.
    pub position: (PxPoint, DipPoint),
    /// Final dimensions of the client area of the window (excluding outer chrome).
    pub size: DipSize,

    /// Final scale factor.
    pub scale_factor: Factor,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,

    /// Padding that must be applied to the window content so that it stays clear of screen obstructions
    /// such as a camera notch cutout.
    ///
    /// Note that the *unsafe* area must still be rendered as it may be partially visible, just don't place nay
    /// interactive or important content outside of this padding.
    pub safe_padding: DipSideOffsets,
}
impl WindowOpenData {
    /// New response.
    pub fn new(
        state: WindowStateAll,
        monitor: Option<MonitorId>,
        position: (PxPoint, DipPoint),
        size: DipSize,
        scale_factor: Factor,
        render_mode: RenderMode,
        safe_padding: DipSideOffsets,
    ) -> Self {
        Self {
            state,
            monitor,
            position,
            size,
            scale_factor,
            render_mode,
            safe_padding,
        }
    }
}

/// Information about a successfully opened headless surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeadlessOpenData {
    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,
}
impl HeadlessOpenData {
    /// New resonse.
    pub fn new(render_mode: RenderMode) -> Self {
        Self { render_mode }
    }
}

/// Represents a focus request indicator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum FocusIndicator {
    /// Activate critical focus request.
    Critical,
    /// Activate informational focus request.
    Info,
}

/// Frame image capture request.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FrameCapture {
    /// Don't capture the frame.
    #[default]
    None,
    /// Captures a full BGRA8 image.
    Full,
    /// Captures an A8 mask image.
    Mask(ImageMaskMode),
}

/// Data for rendering a new frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FrameRequest {
    /// ID of the new frame.
    pub id: FrameId,

    /// Frame clear color.
    pub clear_color: Rgba,

    /// Display list.
    pub display_list: DisplayList,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is sent with the image.
    ///
    /// [`Event::FrameImageReady`]: crate::Event::FrameImageReady
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameRequest {
    /// New request.
    pub fn new(id: FrameId, clear_color: Rgba, display_list: DisplayList, capture: FrameCapture, wait_id: Option<FrameWaitId>) -> Self {
        Self {
            id,
            clear_color,
            display_list,
            capture,
            wait_id,
        }
    }
}

/// Data for rendering a new frame that is derived from the current frame.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FrameUpdateRequest {
    /// ID of the new frame.
    pub id: FrameId,

    /// Bound transforms.
    pub transforms: Vec<FrameValueUpdate<PxTransform>>,
    /// Bound floats.
    pub floats: Vec<FrameValueUpdate<f32>>,
    /// Bound colors.
    pub colors: Vec<FrameValueUpdate<Rgba>>,

    /// New clear color.
    pub clear_color: Option<Rgba>,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is send with the image.
    ///
    /// [`Event::FrameImageReady`]: crate::Event::FrameImageReady
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,

    /// Update payload for API extensions.
    ///
    /// The `zng-view` crate implements this by calling `DisplayListExtension::update` with the payload.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
}
impl FrameUpdateRequest {
    /// New request.
    #[allow(clippy::too_many_arguments)] // already grouping stuff>
    pub fn new(
        id: FrameId,
        transforms: Vec<FrameValueUpdate<PxTransform>>,
        floats: Vec<FrameValueUpdate<f32>>,
        colors: Vec<FrameValueUpdate<Rgba>>,
        clear_color: Option<Rgba>,
        capture: FrameCapture,
        wait_id: Option<FrameWaitId>,
        extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
    ) -> Self {
        Self {
            id,
            transforms,
            floats,
            colors,
            extensions,
            clear_color,
            capture,
            wait_id,
        }
    }

    /// A request that does nothing, apart from re-rendering the frame.
    pub fn empty(id: FrameId) -> FrameUpdateRequest {
        FrameUpdateRequest {
            id,
            transforms: vec![],
            floats: vec![],
            colors: vec![],
            extensions: vec![],
            clear_color: None,
            capture: FrameCapture::None,
            wait_id: None,
        }
    }

    /// If some property updates are requested.
    pub fn has_bounds(&self) -> bool {
        !(self.transforms.is_empty() && self.floats.is_empty() && self.colors.is_empty())
    }

    /// If this request does not do anything, apart from notifying
    /// a new frame if send to the renderer.
    pub fn is_empty(&self) -> bool {
        !self.has_bounds() && self.extensions.is_empty() && self.clear_color.is_none() && self.capture != FrameCapture::None
    }
}
impl fmt::Debug for FrameUpdateRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameUpdateRequest")
            .field("id", &self.id)
            .field("transforms", &self.transforms)
            .field("floats", &self.floats)
            .field("colors", &self.colors)
            .field("clear_color", &self.clear_color)
            .field("capture", &self.capture)
            .finish()
    }
}

/// Configuration of a new window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WindowRequest {
    /// ID that will identify the new window.
    pub id: WindowId,
    /// Title text.
    pub title: Txt,

    /// Window state, position, size and restore rectangle.
    pub state: WindowStateAll,

    /// Lock-in kiosk mode.
    ///
    /// If `true` the app-process will only set fullscreen states, never hide or minimize the window, never
    /// make the window chrome visible and only request an opaque window. The view-process implementer is expected
    /// to also never exit the fullscreen state, even temporally.
    ///
    /// The app-process does not expect the view-process to configure the operating system to run in kiosk mode, but
    /// if possible to detect the view-process can assert that it is running in kiosk mode, logging an error if the assert fails.
    pub kiosk: bool,

    /// If the initial position should be provided the operating system,
    /// if this is not possible the `state.restore_rect.origin` is used.
    pub default_position: bool,

    /// Video mode used when the window is in exclusive state.
    pub video_mode: VideoMode,

    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// If the window is "top-most".
    pub always_on_top: bool,
    /// If the user can move the window.
    pub movable: bool,
    /// If the user can resize the window.
    pub resizable: bool,
    /// Window icon.
    pub icon: Option<ImageId>,
    /// Window cursor icon and visibility.
    pub cursor: Option<CursorIcon>,
    /// Window custom cursor with hotspot.
    pub cursor_image: Option<(ImageId, PxPoint)>,
    /// If the window is see-through in pixels that are not fully opaque.
    pub transparent: bool,

    /// If all or most frames will be *screen captured*.
    ///
    /// If `false` all resources for capturing frame images
    /// are discarded after each screenshot request.
    pub capture_mode: bool,

    /// Render mode preference for this window.
    pub render_mode: RenderMode,

    /// Focus request indicator on init.
    pub focus_indicator: Option<FocusIndicator>,

    /// Ensures the window is focused after open, if not set the initial focus is decided by
    /// the windows manager, usually focusing the new window only if the process that causes the window has focus.
    pub focus: bool,

    /// IME cursor area, if IME is enabled.
    pub ime_area: Option<DipRect>,

    /// Enabled window chrome buttons.
    pub enabled_buttons: WindowButton,

    /// System shutdown warning associated with the window.
    pub system_shutdown_warn: Txt,

    /// Initial payload for API extensions.
    ///
    /// The `zng-view` crate implements this by calling `WindowExtension::configure` and `RendererExtension::configure` with the payload.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
}
impl WindowRequest {
    /// New request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: WindowId,
        title: Txt,
        state: WindowStateAll,
        kiosk: bool,
        default_position: bool,
        video_mode: VideoMode,
        visible: bool,
        taskbar_visible: bool,
        always_on_top: bool,
        movable: bool,
        resizable: bool,
        icon: Option<ImageId>,
        cursor: Option<CursorIcon>,
        cursor_image: Option<(ImageId, PxPoint)>,
        transparent: bool,
        capture_mode: bool,
        render_mode: RenderMode,
        focus_indicator: Option<FocusIndicator>,
        focus: bool,
        ime_area: Option<DipRect>,
        enabled_buttons: WindowButton,
        system_shutdown_warn: Txt,
        extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
    ) -> Self {
        Self {
            id,
            title,
            state,
            kiosk,
            default_position,
            video_mode,
            visible,
            taskbar_visible,
            always_on_top,
            movable,
            resizable,
            icon,
            cursor,
            cursor_image,
            transparent,
            capture_mode,
            render_mode,
            focus_indicator,
            focus,
            extensions,
            ime_area,
            enabled_buttons,
            system_shutdown_warn,
        }
    }

    /// Corrects invalid values if [`kiosk`] is `true`.
    ///
    /// An error is logged for each invalid value.
    ///
    /// [`kiosk`]: Self::kiosk
    pub fn enforce_kiosk(&mut self) {
        if self.kiosk {
            if !self.state.state.is_fullscreen() {
                tracing::error!("window in `kiosk` mode did not request fullscreen");
                self.state.state = WindowState::Exclusive;
            }
            if self.state.chrome_visible {
                tracing::error!("window in `kiosk` mode request chrome");
                self.state.chrome_visible = false;
            }
            if !self.visible {
                tracing::error!("window in `kiosk` mode can only be visible");
                self.visible = true;
            }
        }
    }
}

/// Represents the properties of a window that affect its position, size and state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WindowStateAll {
    /// The window state.
    pub state: WindowState,

    /// Position across monitors.
    ///
    /// This is mostly used to find a monitor to resolve the `restore_rect` in.
    pub global_position: PxPoint,

    /// Position and size of the window in the `Normal` state.
    ///
    /// The position is relative to the monitor.
    pub restore_rect: DipRect,

    /// What state the window goes too when "restored".
    ///
    /// The *restore* state that the window must be set to be restored, if the [current state] is [`Maximized`], [`Fullscreen`] or [`Exclusive`]
    /// the restore state is [`Normal`], if the [current state] is [`Minimized`] the restore state is the previous state.
    ///
    /// When the restore state is [`Normal`] the [`restore_rect`] defines the window position and size.
    ///
    ///
    /// [current state]: Self::state
    /// [`Maximized`]: WindowState::Maximized
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    /// [`Normal`]: WindowState::Normal
    /// [`Minimized`]: WindowState::Minimized
    /// [`restore_rect`]: Self::restore_rect
    pub restore_state: WindowState,

    /// Minimal `Normal` size allowed.
    pub min_size: DipSize,
    /// Maximum `Normal` size allowed.
    pub max_size: DipSize,

    /// If the system provided outer-border and title-bar is visible.
    ///
    /// This is also called the "decoration" or "chrome" of the window. Note that the system may prefer
    pub chrome_visible: bool,
}
impl WindowStateAll {
    /// New state.
    pub fn new(
        state: WindowState,
        global_position: PxPoint,
        restore_rect: DipRect,
        restore_state: WindowState,
        min_size: DipSize,
        max_size: DipSize,
        chrome_visible: bool,
    ) -> Self {
        Self {
            state,
            global_position,
            restore_rect,
            restore_state,
            min_size,
            max_size,
            chrome_visible,
        }
    }

    /// Clamp the `restore_rect.size` to `min_size` and `max_size`.
    pub fn clamp_size(&mut self) {
        self.restore_rect.size = self.restore_rect.size.min(self.max_size).max(self.min_size)
    }

    /// Compute a value for [`restore_state`] given the previous [`state`] in `self` and the `new_state` and update the [`state`].
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_state(&mut self, new_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, self.state, new_state);
        self.state = new_state;
    }

    /// Compute a value for [`restore_state`] given the previous `prev_state` and the new [`state`] in `self`.
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_restore_state_from(&mut self, prev_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, prev_state, self.state);
    }

    fn compute_restore_state(restore_state: WindowState, prev_state: WindowState, new_state: WindowState) -> WindowState {
        if new_state == WindowState::Minimized {
            // restore to previous state from minimized.
            if prev_state != WindowState::Minimized {
                prev_state
            } else {
                WindowState::Normal
            }
        } else if new_state.is_fullscreen() && !prev_state.is_fullscreen() {
            // restore to maximized or normal from fullscreen.
            if prev_state == WindowState::Maximized {
                WindowState::Maximized
            } else {
                WindowState::Normal
            }
        } else if new_state == WindowState::Maximized {
            WindowState::Normal
        } else {
            // Fullscreen to/from Exclusive keeps the previous restore_state.
            restore_state
        }
    }
}

/// Named system dependent cursor icon.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor. Often rendered as arrow.
    #[default]
    Default,

    /// A context menu is available for the object under the cursor. Often
    /// rendered as an arrow with a small menu-like graphic next to it.
    ContextMenu,

    /// Help is available for the object under the cursor. Often rendered as a
    /// question mark or a balloon.
    Help,

    /// The cursor is a pointer that indicates a link. Often rendered as the
    /// backside of a hand with the index finger extended.
    Pointer,

    /// A progress indicator. The program is performing some processing, but is
    /// different from [`CursorIcon::Wait`] in that the user may still interact
    /// with the program.
    Progress,

    /// Indicates that the program is busy and the user should wait. Often
    /// rendered as a watch or hourglass.
    Wait,

    /// Indicates that a cell or set of cells may be selected. Often rendered as
    /// a thick plus-sign with a dot in the middle.
    Cell,

    /// A simple crosshair (e.g., short line segments resembling a "+" sign).
    /// Often used to indicate a two dimensional bitmap selection mode.
    Crosshair,

    /// Indicates text that may be selected. Often rendered as an I-beam.
    Text,

    /// Indicates vertical-text that may be selected. Often rendered as a
    /// horizontal I-beam.
    VerticalText,

    /// Indicates an alias of/shortcut to something is to be created. Often
    /// rendered as an arrow with a small curved arrow next to it.
    Alias,

    /// Indicates something is to be copied. Often rendered as an arrow with a
    /// small plus sign next to it.
    Copy,

    /// Indicates something is to be moved.
    Move,

    /// Indicates that the dragged item cannot be dropped at the current cursor
    /// location. Often rendered as a hand or pointer with a small circle with a
    /// line through it.
    NoDrop,

    /// Indicates that the requested action will not be carried out. Often
    /// rendered as a circle with a line through it.
    NotAllowed,

    /// Indicates that something can be grabbed (dragged to be moved). Often
    /// rendered as the backside of an open hand.
    Grab,

    /// Indicates that something is being grabbed (dragged to be moved). Often
    /// rendered as the backside of a hand with fingers closed mostly out of
    /// view.
    Grabbing,

    /// The east border to be moved.
    EResize,

    /// The north border to be moved.
    NResize,

    /// The north-east corner to be moved.
    NeResize,

    /// The north-west corner to be moved.
    NwResize,

    /// The south border to be moved.
    SResize,

    /// The south-east corner to be moved.
    SeResize,

    /// The south-west corner to be moved.
    SwResize,

    /// The west border to be moved.
    WResize,

    /// The east and west borders to be moved.
    EwResize,

    /// The south and north borders to be moved.
    NsResize,

    /// The north-east and south-west corners to be moved.
    NeswResize,

    /// The north-west and south-east corners to be moved.
    NwseResize,

    /// Indicates that the item/column can be resized horizontally. Often
    /// rendered as arrows pointing left and right with a vertical bar
    /// separating them.
    ColResize,

    /// Indicates that the item/row can be resized vertically. Often rendered as
    /// arrows pointing up and down with a horizontal bar separating them.
    RowResize,

    /// Indicates that the something can be scrolled in any direction. Often
    /// rendered as arrows pointing up, down, left, and right with a dot in the
    /// middle.
    AllScroll,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "+" in the center of the glass.
    ZoomIn,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "-" in the center of the glass.
    ZoomOut,
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(some: CursorIcon) -> Option<CursorIcon>;
}
impl CursorIcon {
    /// All cursor icons.
    pub const ALL: &'static [CursorIcon] = {
        use CursorIcon::*;
        &[
            Default,
            ContextMenu,
            Help,
            Pointer,
            Progress,
            Wait,
            Cell,
            Crosshair,
            Text,
            VerticalText,
            Alias,
            Copy,
            Move,
            NoDrop,
            NotAllowed,
            Grab,
            Grabbing,
            EResize,
            NResize,
            NeResize,
            NwResize,
            SResize,
            SeResize,
            SwResize,
            WResize,
            EwResize,
            NsResize,
            NeswResize,
            NwseResize,
            ColResize,
            RowResize,
            AllScroll,
            ZoomIn,
            ZoomOut,
        ]
    };

    /// Estimated icon size and click spot in that size.
    pub fn size_and_spot(&self, scale_factor: Factor) -> (PxSize, PxPoint) {
        fn splat(s: f32, rel_pt: f32) -> (DipSize, DipPoint) {
            size(s, s, rel_pt, rel_pt)
        }
        fn size(w: f32, h: f32, rel_x: f32, rel_y: f32) -> (DipSize, DipPoint) {
            (
                DipSize::new(Dip::new_f32(w), Dip::new_f32(h)),
                DipPoint::new(Dip::new_f32(w * rel_x), Dip::new_f32(h * rel_y)),
            )
        }

        let (size, spot) = match self {
            CursorIcon::Crosshair
            | CursorIcon::Move
            | CursorIcon::Wait
            | CursorIcon::NotAllowed
            | CursorIcon::NoDrop
            | CursorIcon::Cell
            | CursorIcon::Grab
            | CursorIcon::Grabbing
            | CursorIcon::AllScroll => splat(20.0, 0.5),
            CursorIcon::Text | CursorIcon::NResize | CursorIcon::SResize | CursorIcon::NsResize => size(8.0, 20.0, 0.5, 0.5),
            CursorIcon::VerticalText | CursorIcon::EResize | CursorIcon::WResize | CursorIcon::EwResize => size(20.0, 8.0, 0.5, 0.5),
            _ => splat(20.0, 0.0),
        };

        (size.to_px(scale_factor), spot.to_px(scale_factor))
    }
}

/// Defines a custom mouse cursor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CursorImage {
    /// Cursor image.
    pub img: ImageId,
    /// Exact point in the image that is the mouse position.
    ///
    /// This value is only used if the image format does not contain a hotspot.
    pub hotspot: PxPoint,
}
impl CursorImage {
    /// New cursor.
    pub fn new(img: ImageId, hotspot: PxPoint) -> Self {
        Self { img, hotspot }
    }
}

/// Defines the orientation that a window resize will be performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ResizeDirection {
    /// The east border will be moved.
    East,
    /// The north border will be moved.
    North,
    /// The north-east corner will be moved.
    NorthEast,
    /// The north-west corner will be moved.
    NorthWest,
    /// The south border will be moved.
    South,
    /// The south-east corner will be moved.
    SouthEast,
    /// The south-west corner will be moved.
    SouthWest,
    /// The west border will be moved.
    West,
}
impl From<ResizeDirection> for CursorIcon {
    fn from(direction: ResizeDirection) -> Self {
        use ResizeDirection::*;
        match direction {
            East => CursorIcon::EResize,
            North => CursorIcon::NResize,
            NorthEast => CursorIcon::NeResize,
            NorthWest => CursorIcon::NwResize,
            South => CursorIcon::SResize,
            SouthEast => CursorIcon::SeResize,
            SouthWest => CursorIcon::SwResize,
            West => CursorIcon::WResize,
        }
    }
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(some: ResizeDirection) -> Option<ResizeDirection>;
    fn from(some: ResizeDirection) -> Option<CursorIcon> {
        Some(some.into())
    }
}
impl ResizeDirection {
    /// All directions.
    pub const ALL: &'static [ResizeDirection] = {
        use ResizeDirection::*;
        &[East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West]
    };

    /// Gets if this resize represents two directions.
    pub const fn is_corner(self) -> bool {
        matches!(self, Self::NorthEast | Self::NorthWest | Self::SouthEast | Self::SouthWest)
    }

    /// Gets if this resize represents a single direction.
    pub const fn is_border(self) -> bool {
        !self.is_corner()
    }
}

/// Window state.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum WindowState {
    /// Window is visible, but does not fill the screen.
    #[default]
    Normal,
    /// Window is only visible as an icon in the taskbar.
    Minimized,
    /// Window fills the screen, but not the parts reserved by the system, like the taskbar.
    Maximized,
    /// Window is chromeless and completely fills the screen, including over parts reserved by the system.
    ///
    /// Also called borderless fullscreen.
    Fullscreen,
    /// Window has exclusive access to the monitor's video output, so only the window content is visible.
    Exclusive,
}
impl WindowState {
    /// Returns `true` if `self` matches [`Fullscreen`] or [`Exclusive`].
    ///
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    pub fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen | Self::Exclusive)
    }
}

/// [`Event::FrameRendered`] payload.
///
/// [`Event::FrameRendered`]: crate::Event::FrameRendered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EventFrameRendered {
    /// Window that was rendered.
    pub window: WindowId,
    /// Frame that was rendered.
    pub frame: FrameId,
    /// Frame image, if one was requested with the frame request.
    pub frame_image: Option<ImageLoadedData>,
}
impl EventFrameRendered {
    /// New response.
    pub fn new(window: WindowId, frame: FrameId, frame_image: Option<ImageLoadedData>) -> Self {
        Self {
            window,
            frame,
            frame_image,
        }
    }
}

/// [`Event::WindowChanged`] payload.
///
/// [`Event::WindowChanged`]: crate::Event::WindowChanged
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WindowChanged {
    // note that this payload is handled by `Event::coalesce`, add new fields there too.
    //
    /// Window that has changed state.
    pub window: WindowId,

    /// Window new state, is `None` if the window state did not change.
    pub state: Option<WindowStateAll>,

    /// Window new global position, is `None` if the window position did not change.
    ///
    /// The values are the global position and the position in the monitor.
    pub position: Option<(PxPoint, DipPoint)>,

    /// Window new monitor.
    ///
    /// The window's monitor change when it is moved enough so that most of the
    /// client area is in the new monitor screen.
    pub monitor: Option<MonitorId>,

    /// The window new size, is `None` if the window size did not change.
    pub size: Option<DipSize>,

    /// The window new safe padding, is `None` if the did not change.
    pub safe_padding: Option<DipSideOffsets>,

    /// If the view-process is blocking the event loop for a time waiting for a frame for the new `size` this
    /// ID must be send with the frame to signal that it is the frame for the new size.
    ///
    /// Event loop implementations can use this to resize without visible artifacts
    /// like the clear color flashing on the window corners, there is a timeout to this delay but it
    /// can be a noticeable stutter, a [`render`] or [`render_update`] request for the window unblocks the loop early
    /// to continue the resize operation.
    ///
    /// [`render`]: crate::Api::render
    /// [`render_update`]: crate::Api::render_update
    pub frame_wait_id: Option<FrameWaitId>,

    /// What caused the change, end-user/OS modifying the window or the app.
    pub cause: EventCause,
}
impl WindowChanged {
    /// New response.
    #[allow(clippy::too_many_arguments)] // already grouping stuff>
    pub fn new(
        window: WindowId,
        state: Option<WindowStateAll>,
        position: Option<(PxPoint, DipPoint)>,
        monitor: Option<MonitorId>,
        size: Option<DipSize>,
        safe_padding: Option<DipSideOffsets>,
        frame_wait_id: Option<FrameWaitId>,
        cause: EventCause,
    ) -> Self {
        Self {
            window,
            state,
            position,
            monitor,
            size,
            safe_padding,
            frame_wait_id,
            cause,
        }
    }

    /// Create an event that represents window move.
    pub fn moved(window: WindowId, global_position: PxPoint, position: DipPoint, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: Some((global_position, position)),
            monitor: None,
            size: None,
            safe_padding: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window parent monitor change.
    pub fn monitor_changed(window: WindowId, monitor: MonitorId, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: Some(monitor),
            size: None,
            safe_padding: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window resized.
    pub fn resized(window: WindowId, size: DipSize, cause: EventCause, frame_wait_id: Option<FrameWaitId>) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: None,
            size: Some(size),
            safe_padding: None,
            frame_wait_id,
            cause,
        }
    }

    /// Create an event that represents [`WindowStateAll`] change.
    pub fn state_changed(window: WindowId, state: WindowStateAll, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: Some(state),
            position: None,
            monitor: None,
            size: None,
            safe_padding: None,
            frame_wait_id: None,
            cause,
        }
    }
}

/// Identifier of a frame or frame update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, bytemuck::NoUninit)]
#[repr(C)]
pub struct FrameId(u32, u32);
impl FrameId {
    /// Dummy frame ID.
    pub const INVALID: FrameId = FrameId(u32::MAX, u32::MAX);

    /// Create first frame id of a window.
    pub fn first() -> FrameId {
        FrameId(0, 0)
    }

    /// Create the next full frame ID after the current one.
    pub fn next(self) -> FrameId {
        let mut id = self.0.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(id, 0)
    }

    /// Create the next update frame ID after the current one.
    pub fn next_update(self) -> FrameId {
        let mut id = self.1.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(self.0, id)
    }

    /// Get the raw ID.
    pub fn get(self) -> u64 {
        ((self.0 as u64) << 32) | (self.1 as u64)
    }

    /// Get the full frame ID.
    pub fn epoch(self) -> u32 {
        self.0
    }

    /// Get the frame update ID.
    pub fn update(self) -> u32 {
        self.1
    }
}

/// Cause of a window state change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventCause {
    /// Operating system or end-user affected the window.
    System,
    /// App affected the window.
    App,
}

bitflags::bitflags! {
    /// Window chrome buttons.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct WindowButton: u32 {
        /// Close button.
        const CLOSE = 1 << 0;
        /// Minimize button.
        const MINIMIZE = 1 << 1;
        /// Maximize/restore button.
        const MAXIMIZE = 1 << 2;
    }
}
