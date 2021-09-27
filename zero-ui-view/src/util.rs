use std::{cell::Cell, rc::Rc};

use zero_ui_view_api::{units::*, WinId};

/// Manages the "current" `glutin` OpenGL context.
///
/// If this manager is in use all OpenGL contexts created in the process must be managed by it.
#[derive(Default)]
pub(crate) struct GlContextManager {
    current: Rc<Cell<Option<WinId>>>,
}
impl GlContextManager {
    /// Start managing a "headed" glutin context.
    pub fn manage_headed(&self, id: WinId, ctx: glutin::RawContext<glutin::NotCurrent>) -> GlContext {
        GlContext {
            id,
            ctx: Some(unsafe { ctx.treat_as_current() }),
            current: Rc::clone(&self.current),
        }
    }

    /// Start managing a headless glutin context.
    pub fn manage_headless(&self, id: WinId, ctx: glutin::Context<glutin::NotCurrent>) -> GlHeadlessContext {
        GlHeadlessContext {
            id,
            ctx: Some(unsafe { ctx.treat_as_current() }),
            current: Rc::clone(&self.current),
        }
    }
}

/// Managed headless Open-GL context.
pub(crate) struct GlHeadlessContext {
    id: WinId,
    ctx: Option<glutin::Context<glutin::PossiblyCurrent>>,
    current: Rc<Cell<Option<WinId>>>,
}
impl GlHeadlessContext {
    /// Gets the context as current.
    ///
    /// It can already be current or it is made current.
    pub fn make_current(&mut self) -> &mut glutin::Context<glutin::PossiblyCurrent> {
        let id = Some(self.id);
        if self.current.get() != id {
            self.current.set(id);
            let c = self.ctx.take().unwrap();
            // glutin docs says that calling `make_not_current` is not necessary and
            // that "If you call make_current on some context, you should call treat_as_not_current as soon
            // as possible on the previously current context."
            //
            // As far as the glutin code goes `treat_as_not_current` just changes the state tag, so we can call
            // `treat_as_not_current` just to get access to the `make_current` when we know it is not current
            // anymore, and just ignore the whole "current state tag" thing.
            let c = unsafe { c.treat_as_not_current().make_current() }.expect("failed to make current");
            self.ctx = Some(c);
        }
        self.ctx.as_mut().unwrap()
    }
}
impl Drop for GlHeadlessContext {
    fn drop(&mut self) {
        if self.current.get() == Some(self.id) {
            let _ = unsafe { self.ctx.take().unwrap().make_not_current() };
            self.current.set(None);
        } else {
            let _ = unsafe { self.ctx.take().unwrap().treat_as_not_current() };
        }
    }
}

/// Managed headed Open-GL context.
pub(crate) struct GlContext {
    id: WinId,
    ctx: Option<glutin::ContextWrapper<glutin::PossiblyCurrent, ()>>,
    current: Rc<Cell<Option<WinId>>>,
}
impl GlContext {
    /// Gets the context as current.
    ///
    /// It can already be current or it is made current.
    pub fn make_current(&mut self) -> &mut glutin::ContextWrapper<glutin::PossiblyCurrent, ()> {
        let id = Some(self.id);
        if self.current.get() != id {
            self.current.set(id);
            let c = self.ctx.take().unwrap();
            // glutin docs says that calling `make_not_current` is not necessary and
            // that "If you call make_current on some context, you should call treat_as_not_current as soon
            // as possible on the previously current context."
            //
            // As far as the glutin code goes `treat_as_not_current` just changes the state tag, so we can call
            // `treat_as_not_current` just to get access to the `make_current` when we know it is not current
            // anymore, and just ignore the whole "current state tag" thing.
            let c = unsafe { c.treat_as_not_current().make_current() }.expect("failed to make current");
            self.ctx = Some(c);
        }
        self.ctx.as_mut().unwrap()
    }

    /// Glutin requires that the context is [dropped before the window][1], calling this
    /// function safely disposes of the context, the winit window should be dropped immediately after.
    ///
    /// [1]: https://docs.rs/glutin/0.27.0/glutin/type.WindowedContext.html#method.split
    pub fn drop_before_winit(&mut self) {
        if self.current.get() == Some(self.id) {
            let _ = unsafe { self.ctx.take().unwrap().make_not_current() };
            self.current.set(None);
        } else {
            let _ = unsafe { self.ctx.take().unwrap().treat_as_not_current() };
        }
    }
}
impl Drop for GlContext {
    fn drop(&mut self) {
        if self.ctx.is_some() {
            panic!("call `drop_before_winit` before dropping")
        }
    }
}

/// Sets a window subclass that calls a raw event handler.
///
/// Use this to receive Windows OS events not covered in [`raw_events`].
///
/// Returns if adding a subclass handler succeeded.
///
/// # Handler
///
/// The handler inputs are the first 4 arguments of a [`SUBCLASSPROC`].
/// You can use closure capture to include extra data.
///
/// The handler must return `Some(LRESULT)` to stop the propagation of a specific message.
///
/// The handler is dropped after it receives the `WM_DESTROY` message.
///
/// # Panics
///
/// Panics in headless mode.
///
/// [`raw_events`]: crate::app::raw_events
/// [`SUBCLASSPROC`]: https://docs.microsoft.com/en-us/windows/win32/api/commctrl/nc-commctrl-subclassproc
#[cfg(windows)]
pub fn set_raw_windows_event_handler<
    H: FnMut(
            winapi::shared::windef::HWND,
            winapi::shared::minwindef::UINT,
            winapi::shared::minwindef::WPARAM,
            winapi::shared::minwindef::LPARAM,
        ) -> Option<winapi::shared::minwindef::LRESULT>
        + 'static,
>(
    window: &glutin::window::Window,
    subclass_id: winapi::shared::basetsd::UINT_PTR,
    handler: H,
) -> bool {
    use glutin::platform::windows::WindowExtWindows;

    let hwnd = window.hwnd() as winapi::shared::windef::HWND;
    let data = Box::new(handler);
    unsafe {
        winapi::um::commctrl::SetWindowSubclass(
            hwnd,
            Some(subclass_raw_event_proc::<H>),
            subclass_id,
            Box::into_raw(data) as winapi::shared::basetsd::DWORD_PTR,
        ) != 0
    }
}
#[cfg(windows)]
unsafe extern "system" fn subclass_raw_event_proc<
    H: FnMut(
            winapi::shared::windef::HWND,
            winapi::shared::minwindef::UINT,
            winapi::shared::minwindef::WPARAM,
            winapi::shared::minwindef::LPARAM,
        ) -> Option<winapi::shared::minwindef::LRESULT>
        + 'static,
>(
    hwnd: winapi::shared::windef::HWND,
    msg: winapi::shared::minwindef::UINT,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
    _id: winapi::shared::basetsd::UINT_PTR,
    data: winapi::shared::basetsd::DWORD_PTR,
) -> winapi::shared::minwindef::LRESULT {
    use winapi::um::winuser::WM_DESTROY;
    match msg {
        WM_DESTROY => {
            // last call and cleanup.
            let mut handler = Box::from_raw(data as *mut H);
            handler(hwnd, msg, wparam, lparam).unwrap_or_default()
        }

        msg => {
            let handler = &mut *(data as *mut H);
            if let Some(r) = handler(hwnd, msg, wparam, lparam) {
                r
            } else {
                winapi::um::commctrl::DefSubclassProc(hwnd, msg, wparam, lparam)
            }
        }
    }
}

/// Conversion from `winit` logical units to [`Dip`].
///
/// All conversions are 1 to 1.
pub(crate) trait WinitToDip {
    /// `Self` equivalent in [`Dip`] units.
    type AsDip;

    /// Returns `self` in [`Dip`] units.
    fn to_dip(self) -> Self::AsDip;
}

/// Conversion from `winit` physical units to [`Dip`].
///
/// All conversions are 1 to 1.

pub(crate) trait WinitToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Returns `self` in [`Px`] units.
    fn to_px(self) -> Self::AsPx;
}

/// Conversion from [`Dip`] to `winit` logical units.

pub(crate) trait DipToWinit {
    /// `Self` equivalent in `winit` logical units.
    type AsWinit;

    /// Returns `self` in `winit` logical units.
    fn to_winit(self) -> Self::AsWinit;
}

impl DipToWinit for DipPoint {
    type AsWinit = glutin::dpi::LogicalPosition<f32>;

    fn to_winit(self) -> Self::AsWinit {
        glutin::dpi::LogicalPosition::new(self.x.to_f32(), self.y.to_f32())
    }
}

impl WinitToDip for glutin::dpi::LogicalPosition<f64> {
    type AsDip = DipPoint;

    fn to_dip(self) -> Self::AsDip {
        DipPoint::new(Dip::new_f32(self.x as f32), Dip::new_f32(self.y as f32))
    }
}

impl WinitToPx for glutin::dpi::PhysicalPosition<i32> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x), Px(self.y))
    }
}

impl WinitToPx for glutin::dpi::PhysicalPosition<f64> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x as i32), Px(self.y as i32))
    }
}

impl DipToWinit for DipSize {
    type AsWinit = glutin::dpi::LogicalSize<f32>;

    fn to_winit(self) -> Self::AsWinit {
        glutin::dpi::LogicalSize::new(self.width.to_f32(), self.height.to_f32())
    }
}

impl WinitToDip for glutin::dpi::LogicalSize<f64> {
    type AsDip = DipSize;

    fn to_dip(self) -> Self::AsDip {
        DipSize::new(Dip::new_f32(self.width as f32), Dip::new_f32(self.height as f32))
    }
}

impl WinitToPx for glutin::dpi::PhysicalSize<u32> {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width as i32), Px(self.height as i32))
    }
}
