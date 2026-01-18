use std::{mem, pin::Pin};

use parking_lot::Mutex;
use zng_app::{Deadline, update::UPDATES, view_process::raw_events::RAW_COLORS_CONFIG_CHANGED_EVENT, widget::{WIDGET, WidgetCtx, info::WidgetInfoTree}, window::{WINDOW, WindowCtx, WindowId, WindowMode}};
use zng_layout::{context::LayoutPassId, unit::FactorUnits as _};
use zng_var::{ResponderVar, ResponseVar, Var, var};
use zng_view_api::config::ColorsConfig;
use zng_wgt::prelude::UiNode;

use crate::{CloseWindowResult, MONITORS, WINDOWS, WINDOWS_SV, WindowInstanceState, WindowLoadingHandle, WindowRoot, WindowRootExtenderArgs, WindowVars};


/// Extensions methods for [`WINDOW`] contexts of windows open by [`WINDOWS`].
///
/// [`WINDOW`]: zng_app::window::WINDOW
#[expect(non_camel_case_types)]
pub trait WINDOW_Ext {
    /// Clone a reference to the variables that get and set window properties.
    fn vars(&self) -> WindowVars {
        WindowVars::req()
    }

    /// Enable accessibility info.
    ///
    /// If access is not already enabled, enables it in the app-process only.
    fn enable_access(&self) {
        let vars = WINDOW.vars();
        let access_enabled = &vars.0.access_enabled;
        if access_enabled.get().is_disabled() {
            access_enabled.modify(|e| **e |= zng_app::widget::info::access::AccessEnabled::APP);
        }
    }

    /// Gets a handle that stops the window from loading while the handle is alive.
    ///
    /// A window is only opened in the view-process after it is loaded, without any loading handles the window is considered loaded
    /// after the first layout pass. Nodes in the window can request a loading handle to delay the view opening to after all async resources
    /// it requires are loaded.
    ///
    /// Note that a window is only loaded after all handles are dropped or expired, you should set a reasonable `deadline`,  
    /// it is best to partially render a window after a short time than not show anything.
    ///
    /// Returns `None` if the window has already loaded.
    fn loading_handle(&self, deadline: impl Into<Deadline>) -> Option<WindowLoadingHandle> {
        WINDOWS.loading_handle(WINDOW.id(), deadline)
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    #[cfg(feature = "image")]
    fn frame_image(&self, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        WINDOWS.frame_image(WINDOW.id(), mask)
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    #[cfg(feature = "image")]
    fn frame_image_rect(&self, rect: zng_layout::unit::PxRect, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        WINDOWS.frame_image_rect(WINDOW.id(), rect, mask)
    }

    /// Move the window to the front of the operating system Z stack.
    ///
    /// See [`WINDOWS.bring_to_top`] for more details.
    ///
    /// [`WINDOWS.bring_to_top`]: WINDOWS::bring_to_top
    fn bring_to_top(&self) {
        WINDOWS.bring_to_top(WINDOW.id());
    }

    /// Starts closing the window, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If the window has children they are closed together.
    ///
    /// Returns a response var that will update once with the result of the operation.
    ///
    /// See [`WINDOWS.close`] for more details.
    ///
    /// [`WINDOWS.close`]: WINDOWS::close
    fn close(&self) -> ResponseVar<CloseWindowResult> {
        WINDOWS.close(WINDOW.id())
    }
}
impl WINDOW_Ext for WINDOW {}

pub(crate) struct WindowInstance {
    pub(crate) mode: WindowMode,
    pub(crate) pending_loading: Option<Var<usize>>,
    pub(crate) vars: Option<WindowVars>,
    pub(crate) info: Option<WidgetInfoTree>,
    pub(crate) root: Option<WindowNode>,
}
impl WindowInstance {
    pub(crate) fn new(
        id: WindowId,
        mode: WindowMode,
        new_window: Pin<Box<dyn Future<Output = WindowRoot> + Send + 'static>>,
        r: ResponderVar<WindowVars>,
    ) -> Self {
        let w = Self {
            mode,
            pending_loading: Some(var(0)),
            vars: None,
            info: None,
            root: None,
        };
        UPDATES.run(async move {
            // init WINDOW context, vars
            let primary_scale_factor = match mode {
                WindowMode::Headed => MONITORS
                    .primary_monitor()
                    .get()
                    .map(|m| m.scale_factor().get())
                    .unwrap_or_else(|| 1.fct()),
                WindowMode::Headless | WindowMode::HeadlessWithRenderer => 1.fct(),
            };

            let system_colors = match mode {
                WindowMode::Headed => RAW_COLORS_CONFIG_CHANGED_EVENT
                    .var_latest()
                    .get()
                    .map(|a| a.config)
                    .unwrap_or_default(),
                WindowMode::Headless | WindowMode::HeadlessWithRenderer => ColorsConfig::default(),
            };

            let vars = {
                let mut s = WINDOWS_SV.write();
                let vars = WindowVars::new(s.default_render_mode.get(), primary_scale_factor, system_colors);
                r.respond(vars.clone());
                s.windows.get_mut(&id).unwrap().vars = Some(vars.clone());
                vars
            };

            // poll `new_window` with context
            struct CtxFut {
                f: Pin<Box<dyn Future<Output = WindowRoot> + Send + 'static>>,
                ctx: Option<WindowCtx>,
            }
            impl Future for CtxFut {
                type Output = (WindowRoot, WindowCtx);

                fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                    let s = &mut *self.as_mut();
                    let r = WINDOW.with_context(s.ctx.as_mut().unwrap(), || s.f.as_mut().poll(cx));
                    match r {
                        std::task::Poll::Ready(w) => std::task::Poll::Ready((w, s.ctx.take().unwrap())),
                        std::task::Poll::Pending => std::task::Poll::Pending,
                    }
                }
            }
            let new_window = CtxFut {
                f: new_window,
                ctx: Some(WindowCtx::new(id, mode)),
            };
            let (mut root, win_ctx) = new_window.await;

            // apply root extenders
            let mut extenders = mem::take(&mut WINDOWS_SV.write().root_extenders).into_inner();
            for ext in &mut extenders {
                root.child = ext(WindowRootExtenderArgs { root: root.child });
            }
            {
                let mut s = WINDOWS_SV.write();
                extenders.append(s.root_extenders.get_mut());
                *s.root_extenders.get_mut() = extenders;
            }

            // init and request first info update
            let mut root = WindowNode {
                win_ctx,
                wgt_ctx: WidgetCtx::new(root.id),
                layout_pass: LayoutPassId::new(),
                root: Mutex::new(root),
            };
            root.with_root(|n| n.init());
            UPDATES.update_info_window(id);
            UPDATES.layout_window(id);
            UPDATES.render_window(id);
            WINDOWS_SV.write().windows.get_mut(&id).unwrap().root = Some(root);
            vars.instance_state().set(WindowInstanceState::Loading);

            // will continue in WindowsService::update_info, called by app loop
        }).perm();
        w
    }
}

pub(crate) struct WindowNode {
    win_ctx: WindowCtx,
    pub(crate) wgt_ctx: WidgetCtx,
    pub(crate) layout_pass: LayoutPassId,
    // Mutex for Sync only
    root: Mutex<WindowRoot>,
}
impl WindowNode {
    pub(crate) fn with_root<R>(&mut self, f: impl FnOnce(&mut UiNode) -> R) -> R {
        WINDOW.with_context(&mut self.win_ctx, || {
            WIDGET.with_context(&mut self.wgt_ctx, zng_app::widget::WidgetUpdateMode::Bubble, || {
                f(&mut self.root.get_mut().child)
            })
        })
    }
}