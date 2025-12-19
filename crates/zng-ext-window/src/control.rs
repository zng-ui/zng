//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use std::{mem, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    access::{ACCESS_DEINITED_EVENT, ACCESS_INITED_EVENT},
    event::{AnyEventArgs, CommandHandle},
    hn_once,
    render::{FrameBuilder, FrameUpdate},
    static_id,
    timer::TIMERS,
    update::{EventUpdate, InfoUpdates, LayoutUpdates, RenderUpdates, UPDATES, WidgetUpdates},
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewHeadless, ViewRenderer, ViewWindow,
        raw_events::{
            RAW_COLORS_CONFIG_CHANGED_EVENT, RAW_HEADLESS_OPEN_EVENT, RAW_IME_EVENT, RAW_WINDOW_CHANGED_EVENT, RAW_WINDOW_FOCUS_EVENT,
            RAW_WINDOW_OPEN_EVENT, RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT, RawWindowFocusArgs,
        },
    },
    widget::{
        VarLayout, WIDGET, WidgetCtx, WidgetId, WidgetUpdateMode,
        info::{WidgetInfoBuilder, WidgetInfoTree, WidgetLayout, WidgetMeasure, WidgetPath, access::AccessEnabled},
        node::{UiNode, UiNodeImpl, WidgetUiNodeImpl},
    },
    window::{WINDOW, WindowCtx, WindowId, WindowMode},
};
use zng_app_context::LocalContext;
use zng_clone_move::clmv;
use zng_color::{LightDark, Rgba, colors};
use zng_layout::{
    context::{DIRECTION_VAR, LAYOUT, LayoutMetrics, LayoutPassId},
    unit::{
        Dip, DipPoint, DipRect, DipSize, DipToPx, Factor, FactorUnits, Layout1d, Layout2d, Length, Px, PxConstraints, PxDensity, PxPoint,
        PxRect, PxSize, PxToDip, PxVector, TimeUnits,
    },
};
use zng_state_map::StateId;
use zng_task::channel::ChannelError;
use zng_var::{Var, VarHandles};
use zng_view_api::{
    DragDropId, FocusResult, Ime,
    config::{ColorScheme, FontAntiAliasing},
    drag_drop::{DragDropData, DragDropEffect, DragDropError},
    window::{
        EventCause, FrameCapture, FrameId, FrameRequest, FrameUpdateRequest, FrameWaitId, HeadlessRequest, RenderMode, WindowRequest,
        WindowState, WindowStateAll,
    },
};
use zng_wgt::prelude::WidgetInfo;

use crate::{
    AutoSize, HeadlessMonitor, MONITORS, MONITORS_CHANGED_EVENT, MonitorInfo, StartPosition, WINDOW_CHANGED_EVENT, WINDOW_Ext,
    WINDOW_FOCUS, WINDOWS, WINDOWS_DRAG_DROP, WidgetInfoImeArea, WindowChangedArgs, WindowRoot, WindowVars,
    cmd::{MINIMIZE_CMD, RESTORE_CMD, WindowCommands},
};

#[cfg(feature = "image")]
use zng_app::{Deadline, view_process::raw_events::RAW_FRAME_RENDERED_EVENT};

#[cfg(feature = "image")]
use zng_ext_image::{IMAGES, ImageRenderArgs, ImageSource, ImageVar, Img};

#[cfg(feature = "image")]
use crate::{FRAME_IMAGE_READY_EVENT, FrameCaptureMode, FrameImageReadyArgs};

#[cfg(feature = "image")]
use zng_var::VarHandle;

#[cfg(feature = "image")]
use crate::WindowIcon;

#[cfg(feature = "image")]
struct ImageResources {
    icon_var: Option<ImageVar>,
    cursor_var: Option<ImageVar>,
    icon_binding: VarHandle,
    cursor_binding: VarHandle,
    deadline: Deadline,
}
#[cfg(feature = "image")]
impl Default for ImageResources {
    fn default() -> Self {
        Self {
            icon_var: None,
            cursor_var: None,
            icon_binding: VarHandle::dummy(),
            cursor_binding: VarHandle::dummy(),
            deadline: Deadline::timeout(1.secs()),
        }
    }
}

struct ImeInfo {
    target: WidgetPath,
    has_preview: bool,
    area: DipRect,
}

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window: Option<ViewWindow>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewWindow) + Send>>,
    vars: WindowVars,
    respawned: bool,

    content: ContentCtrl,

    // init config.
    start_position: StartPosition,
    start_focused: bool,
    kiosk: Option<WindowState>, // Some(enforced_fullscreen)
    transparent: bool,
    render_mode: Option<RenderMode>,

    // current state.
    state: Option<WindowStateAll>, // None if not inited.
    monitor: Option<MonitorInfo>,
    resize_wait_id: Option<FrameWaitId>,
    #[cfg(feature = "image")]
    img_res: ImageResources,
    actual_state: Option<WindowState>, // for WindowChangedEvent
    parent_color_scheme: Option<Var<ColorScheme>>,
    parent_accent_color: Option<Var<LightDark>>,
    actual_parent: Option<WindowId>,
    root_font_size: Dip,
    render_access_update: Option<WidgetInfoTree>, // previous info tree
    ime_info: Option<ImeInfo>,
    cancel_ime_handle: CommandHandle,
    open_title_menu_handle: CommandHandle,
    drag_move_handle: CommandHandle,
}
impl HeadedCtrl {
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: WindowRoot) -> Self {
        Self {
            window: None,
            waiting_view: false,
            delayed_view_updates: vec![],

            start_position: content.start_position,
            start_focused: content.start_focused,
            kiosk: if content.kiosk { Some(WindowState::Fullscreen) } else { None },
            transparent: content.transparent,
            render_mode: content.render_mode,

            content: ContentCtrl::new(vars.clone(), commands, content),
            vars: vars.clone(),
            respawned: false,

            state: None,
            monitor: None,
            resize_wait_id: None,
            #[cfg(feature = "image")]
            img_res: ImageResources::default(),
            parent_color_scheme: None,
            parent_accent_color: None,
            actual_parent: None,
            actual_state: None,
            root_font_size: Dip::from_px(Length::pt_to_px(11.0, 1.fct()), 1.fct()),
            render_access_update: None,
            ime_info: None,
            cancel_ime_handle: CommandHandle::dummy(),
            open_title_menu_handle: CommandHandle::dummy(),
            drag_move_handle: CommandHandle::dummy(),
        }
    }

    fn update_gen(&mut self, update: impl FnOnce(&ViewWindow) + Send + 'static) {
        if let Some(view) = &self.window {
            // view is ready, just update.
            update(view);
        } else if self.waiting_view {
            // update after view requested, but still not ready. Will apply when the view is received
            // or be discarded if the view-process respawns.
            self.delayed_view_updates.push(Box::new(update));
        } else {
            // respawning or view-process not inited, will recreate entire window.
        }
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        if self.window.is_none() && !self.waiting_view {
            // we request a view on the first layout.
            UPDATES.layout_window(WINDOW.id());

            if let Some(enforced_fullscreen) = self.kiosk {
                // enforce kiosk in pre-init.

                if !self.vars.state().get().is_fullscreen() {
                    self.vars.state().set(enforced_fullscreen);
                }
            }
        }

        if let Some(query) = self.vars.monitor().get_new() {
            if self.monitor.is_none() {
                let monitor = query.select_fallback();
                let scale_factor = monitor.scale_factor().get();
                self.vars.0.scale_factor.set(scale_factor);
                self.monitor = Some(monitor);
                UPDATES.layout_window(WINDOW.id());
            } else if let Some(new) = query.select() {
                let current = self.vars.0.actual_monitor.get();
                if Some(new.id()) != current {
                    let scale_factor = new.scale_factor().get();
                    self.vars.0.scale_factor.set(scale_factor);
                    self.vars.0.actual_monitor.set(new.id());
                    self.monitor = Some(new);
                    UPDATES.layout_window(WINDOW.id());
                }
            }
        }
        if let Some(prev_state) = self.state.clone() {
            debug_assert!(self.window.is_some() || self.waiting_view || self.respawned);

            let mut new_state = prev_state.clone();

            if self.vars.chrome().is_new() || WINDOWS.system_chrome().is_new() {
                let mut chrome = self.vars.chrome().get();

                if self.kiosk.is_some() && !chrome {
                    tracing::error!("window in `kiosk` mode can not show chrome");
                    chrome = false;
                }

                new_state.chrome_visible = chrome && !WINDOWS.system_chrome().get().needs_custom();
            }

            if let Some(mut req_state) = self.vars.state().get_new() {
                if let Some(enforced_fullscreen) = &mut self.kiosk {
                    if !req_state.is_fullscreen() {
                        tracing::error!("window in `kiosk` mode can only be fullscreen");

                        req_state = *enforced_fullscreen;
                    } else {
                        *enforced_fullscreen = req_state;
                    }
                }

                new_state.set_state(req_state);
                self.vars.0.restore_state.set(new_state.restore_state);
            }

            if (self.vars.min_size().is_new() || self.vars.max_size().is_new())
                && let Some(m) = &self.monitor
            {
                let scale_factor = m.scale_factor().get();
                let screen_density = m.density().get();
                let screen_size = m.size().get();
                let (min_size, max_size) = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
                    let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
                    let max_size = self.vars.max_size().layout_dft(screen_size);

                    (min_size.to_dip(scale_factor), max_size.to_dip(scale_factor))
                });

                let size = new_state.restore_rect.size;

                new_state.restore_rect.size = size.min(max_size).max(min_size);
                new_state.min_size = min_size;
                new_state.max_size = max_size;
            }

            if let Some(auto) = self.vars.auto_size().get_new()
                && auto != AutoSize::DISABLED
            {
                UPDATES.layout_window(WINDOW.id());
            }

            if self.vars.size().is_new() {
                let auto_size = self.vars.auto_size().get();

                if auto_size != AutoSize::CONTENT
                    && let Some(m) = &self.monitor
                {
                    let scale_factor = m.scale_factor().get();
                    let screen_density = m.density().get();
                    let screen_size = m.size().get();
                    let size = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
                        self.vars.size().layout_dft(default_size(scale_factor)).to_dip(scale_factor)
                    });

                    let size = size.min(new_state.max_size).max(new_state.min_size);

                    if !auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        new_state.restore_rect.size.width = size.width;
                    }
                    if !auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        new_state.restore_rect.size.height = size.height;
                    }
                }
            }

            if let Some(pos) = self.vars.position().get_new()
                && let Some(m) = &self.monitor
            {
                let scale_factor = m.scale_factor().get();
                let screen_density = m.density().get();
                let screen_size = m.size().get();
                let pos = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
                    pos.layout_dft(PxPoint::new(Px(50), Px(50)))
                });
                new_state.restore_rect.origin = pos.to_dip(scale_factor);
            }

            if let Some(mut visible) = self.vars.visible().get_new() {
                if !visible && self.kiosk.is_some() {
                    tracing::error!("window in `kiosk` mode can not be hidden");
                    visible = true;
                }

                self.update_gen(move |view| {
                    let _: Ignore = view.set_visible(visible);
                });
            }

            if let Some(movable) = self.vars.movable().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_movable(movable);
                });
            }

            if let Some(resizable) = self.vars.resizable().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_resizable(resizable);
                });
            }

            if let Some(buttons) = self.vars.enabled_buttons().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_enabled_buttons(buttons);
                })
            }

            if let Some(reason) = self.vars.system_shutdown_warn().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_system_shutdown_warn(reason);
                })
            }

            if prev_state != new_state {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_state(new_state);
                })
            }
        }

        if let Some(font_size) = self.vars.font_size().get_new()
            && let Some(m) = &self.monitor
        {
            let scale_factor = m.scale_factor().get();
            let screen_density = m.density().get();
            let screen_size = m.size().get();
            let mut font_size_px = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
                font_size.layout_dft_x(Length::pt_to_px(11.0, scale_factor))
            });
            if font_size_px < Px(0) {
                tracing::error!("invalid font size {font_size:?} => {font_size_px:?}");
                font_size_px = Length::pt_to_px(11.0, scale_factor);
            }
            let font_size_dip = font_size_px.to_dip(scale_factor);

            if font_size_dip != self.root_font_size {
                self.root_font_size = font_size_dip;
                UPDATES.layout_window(WINDOW.id());
            }
        }

        #[cfg(feature = "image")]
        let mut img_res_loading = vec![];

        // icon:
        #[cfg(feature = "image")]
        let mut send_icon = false;
        #[cfg(feature = "image")]
        if let Some(ico) = self.vars.icon().get_new() {
            self.img_res.icon_var = match ico {
                WindowIcon::Default => None,
                WindowIcon::Image(ImageSource::Render(ico, _)) => {
                    Some(IMAGES.cache(ImageSource::Render(ico.clone(), Some(ImageRenderArgs::new(WINDOW.id())))))
                }
                WindowIcon::Image(source) => Some(IMAGES.cache(source)),
            };

            if let Some(ico) = &self.img_res.icon_var {
                self.img_res.icon_binding = ico.bind_map(&self.vars.0.actual_icon, |img| Some(img.clone()));

                if ico.get().is_loading() && self.window.is_none() && !self.waiting_view {
                    img_res_loading.push(ico.clone());
                }
            } else {
                self.vars.0.actual_icon.set(None);
                self.img_res.icon_binding = VarHandle::dummy();
            }

            send_icon = true;
        } else if self.img_res.icon_var.as_ref().map(|ico| ico.is_new()).unwrap_or(false) {
            send_icon = true;
        }
        #[cfg(feature = "image")]
        if send_icon {
            let icon = self.img_res.icon_var.as_ref().and_then(|ico| ico.get().view().cloned());
            self.update_gen(move |view| {
                let _: Ignore = view.set_icon(icon.as_ref());
            });
        }

        // cursor (image):
        if let Some(cursor) = self.vars.cursor().get_new() {
            match cursor {
                crate::CursorSource::Icon(ico) => {
                    #[cfg(feature = "image")]
                    {
                        self.img_res.cursor_var = None;
                    }
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_cursor(Some(ico));
                        #[cfg(feature = "image")]
                        let _: Ignore = view.set_cursor_image(None, PxPoint::zero());
                    });
                }
                #[cfg(feature = "image")]
                crate::CursorSource::Img(img) => {
                    self.img_res.cursor_var = Some(match img.source {
                        ImageSource::Render(cur, _) => {
                            IMAGES.cache(ImageSource::Render(cur.clone(), Some(ImageRenderArgs::new(WINDOW.id()))))
                        }
                        source => IMAGES.cache(source),
                    });

                    self.update_gen(move |view| {
                        let _: Ignore = view.set_cursor(Some(img.fallback));
                        let _: Ignore = view.set_cursor_image(None, PxPoint::zero());
                    });
                }
                crate::CursorSource::Hidden => {
                    #[cfg(feature = "image")]
                    {
                        self.img_res.cursor_var = None;
                    }
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_cursor(None);
                        #[cfg(feature = "image")]
                        let _: Ignore = view.set_cursor_image(None, PxPoint::zero());
                    });
                }
            }

            #[cfg(feature = "image")]
            if let Some(cur) = &self.img_res.cursor_var {
                let hotspot = self.vars.cursor().with(|i| i.hotspot().cloned().unwrap_or_default());

                let cursor_img_to_actual = move |img: &Img| -> Option<(Img, PxPoint)> {
                    let hotspot = if img.is_loaded() {
                        let mut metrics = LayoutMetrics::new(1.fct(), img.size(), Px(16));
                        if let Some(density) = img.density() {
                            metrics = metrics.with_screen_density(density.width);
                        }

                        LAYOUT.with_context(metrics, || hotspot.layout())
                    } else {
                        PxPoint::zero()
                    };

                    Some((img.clone(), hotspot))
                };
                self.vars.0.actual_cursor_img.set_from_map(cur, cursor_img_to_actual.clone());
                self.img_res.cursor_binding = cur.bind_map(&self.vars.0.actual_cursor_img, cursor_img_to_actual);

                if cur.get().is_loading() && self.window.is_none() && !self.waiting_view {
                    img_res_loading.push(cur.clone());
                }
            } else {
                self.vars.0.actual_cursor_img.set(None);
                self.img_res.cursor_binding = VarHandle::dummy();
            }
        }
        #[cfg(feature = "image")]
        if let Some(img_hotspot) = self.vars.0.actual_cursor_img.get_new() {
            self.update_gen(move |view| match img_hotspot {
                Some((img, hotspot)) => {
                    let _: Ignore = view.set_cursor_image(img.view(), hotspot);
                }
                None => {
                    let _: Ignore = view.set_cursor_image(None, PxPoint::zero());
                }
            })
        }

        // setup init wait for images
        #[cfg(feature = "image")]
        if !img_res_loading.is_empty() {
            if self.img_res.deadline.has_elapsed() {
                UPDATES.layout_window(WINDOW.id());
            } else {
                let window_id = WINDOW.id();
                TIMERS
                    .on_deadline(
                        self.img_res.deadline,
                        hn_once!(|_| {
                            if img_res_loading.iter().any(|i| i.get().is_loading()) {
                                // window maybe still waiting.
                                UPDATES.layout_window(window_id);
                            }
                        }),
                    )
                    .perm();
            }
        }

        if let Some(title) = self.vars.title().get_new() {
            self.update_gen(move |view| {
                let _: Ignore = view.set_title(title);
            });
        }

        if let Some(mode) = self.vars.video_mode().get_new() {
            self.update_gen(move |view| {
                let _: Ignore = view.set_video_mode(mode);
            });
        }

        if let Some(visible) = self.vars.taskbar_visible().get_new() {
            self.update_gen(move |view| {
                let _: Ignore = view.set_taskbar_visible(visible);
            });
        }

        if let Some(top) = self.vars.always_on_top().get_new() {
            self.update_gen(move |view| {
                let _: Ignore = view.set_always_on_top(top);
            });
        }

        #[cfg(feature = "image")]
        if let Some(mode) = self.vars.frame_capture_mode().get_new() {
            self.update_gen(move |view| {
                let _: Ignore = view.set_capture_mode(matches!(mode, FrameCaptureMode::All));
            });
        }

        if let Some(m) = &self.monitor {
            if let Some(fct) = m.scale_factor().get_new() {
                self.vars.0.scale_factor.set(fct);
            }
            if m.scale_factor().is_new() || m.size().is_new() || m.density().is_new() {
                UPDATES.layout_window(WINDOW.id());
            }
        }

        if let Some(indicator) = self.vars.focus_indicator().get_new() {
            if WINDOWS.is_focused(WINDOW.id()).unwrap_or(false) {
                self.vars.focus_indicator().set(None);
            } else if let Some(view) = &self.window {
                let _ = view.set_focus_indicator(indicator);
                // will be set to `None` once the window is focused.
            }
            // else indicator is send with init.
        }

        let mut update_colors = false;

        if update_parent(&mut self.actual_parent, &self.vars) {
            self.parent_color_scheme = self
                .actual_parent
                .and_then(|id| WINDOWS.vars(id).ok().map(|v| v.actual_color_scheme()));
            self.parent_accent_color = self
                .actual_parent
                .and_then(|id| WINDOWS.vars(id).ok().map(|v| v.actual_accent_color()));
            update_colors = true;
        }

        if update_colors || self.vars.color_scheme().is_new() || self.parent_color_scheme.as_ref().map(|t| t.is_new()).unwrap_or(false) {
            let scheme = self
                .vars
                .color_scheme()
                .get()
                .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                .unwrap_or_else(|| WINDOWS.system_colors_config().scheme);
            self.vars.0.actual_color_scheme.set(scheme);
        }
        if update_colors || self.vars.accent_color().is_new() || self.parent_accent_color.as_ref().map(|t| t.is_new()).unwrap_or(false) {
            let accent = self
                .vars
                .accent_color()
                .get()
                .or_else(|| self.parent_accent_color.as_ref().map(|t| t.get()))
                .unwrap_or_else(|| WINDOWS.system_colors_config().accent.into());
            self.vars.0.actual_accent_color.set(accent);
        }

        if self.vars.0.access_enabled.is_new() {
            UPDATES.update_info_window(WINDOW.id());
        } else if self.vars.0.access_enabled.get() == AccessEnabled::VIEW && WINDOW_FOCUS.focused().is_new() {
            self.update_access_focused();
        }

        if super::IME_EVENT.has_subscribers() && WINDOW_FOCUS.focused().is_new() {
            self.update_ime();
        }

        self.content.update(update_widgets);
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if let Some(args) = RAW_WINDOW_CHANGED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                let mut state_change = None;
                let mut pos_change = None;
                let mut size_change = None;

                if let Some(monitor) = args.monitor
                    && self.vars.0.actual_monitor.get().map(|m| m != monitor).unwrap_or(true)
                {
                    self.vars.0.actual_monitor.set(Some(monitor));
                    self.monitor = MONITORS.monitor(monitor);
                    if let Some(m) = &self.monitor {
                        let fct = m.scale_factor().get();
                        self.vars.0.scale_factor.set(fct);
                    }
                    UPDATES.layout_window(WINDOW.id());
                }

                if let Some(state) = args.state.clone() {
                    self.vars.state().set(state.state);
                    self.vars.0.restore_rect.set(state.restore_rect);
                    self.vars.0.restore_state.set(state.restore_state);

                    let new_state = state.state;
                    if self.actual_state != Some(new_state) {
                        let prev_state = self.actual_state.unwrap_or(WindowState::Normal);
                        state_change = Some((prev_state, new_state));
                        self.actual_state = Some(new_state);

                        match (prev_state, new_state) {
                            (_, WindowState::Minimized) => {
                                // minimized, minimize children.
                                self.vars.0.children.with(|c| {
                                    for &c in c.iter() {
                                        MINIMIZE_CMD.scoped(c).notify();
                                    }
                                });
                            }
                            (WindowState::Minimized, _) => {
                                // restored, restore children.
                                self.vars.0.children.with(|c| {
                                    for &c in c.iter() {
                                        RESTORE_CMD.scoped(c).notify();
                                    }
                                });

                                // we skip layout & render when minimized.
                                let w_id = WINDOW.id();
                                UPDATES.layout_window(w_id).render_window(w_id);
                            }
                            _ => {}
                        }
                    }

                    self.state = Some(state);
                }

                if let Some((global_pos, pos)) = args.position
                    && (self.vars.0.actual_position.get() != pos || self.vars.0.global_position.get() != global_pos)
                {
                    self.vars.0.actual_position.set(pos);
                    self.vars.0.global_position.set(global_pos);
                    pos_change = Some((global_pos, pos));
                }

                if let Some(size) = args.size
                    && self.vars.0.actual_size.get() != size
                {
                    self.vars.0.actual_size.set(size);
                    size_change = Some(size);

                    UPDATES.layout_window(WINDOW.id());

                    if args.cause == EventCause::System {
                        // resize by system (user)
                        self.vars.auto_size().set(AutoSize::DISABLED);
                    }
                }

                if let Some(padding) = args.safe_padding {
                    self.vars.0.safe_padding.set(padding);
                }

                if let Some(id) = args.frame_wait_id {
                    self.resize_wait_id = Some(id);

                    UPDATES.render_update_window(WINDOW.id());
                }

                if state_change.is_some() || pos_change.is_some() || size_change.is_some() {
                    let args = WindowChangedArgs::new(
                        args.timestamp,
                        args.propagation().clone(),
                        args.window_id,
                        state_change,
                        pos_change,
                        size_change,
                        args.cause,
                    );
                    WINDOW_CHANGED_EVENT.notify(args);
                }
            } else if self.actual_state.unwrap_or(WindowState::Normal) == WindowState::Minimized
                && args.state.as_ref().map(|s| s.state != WindowState::Minimized).unwrap_or(false)
                && self.vars.0.children.with(|c| c.contains(&args.window_id))
            {
                // child restored.
                RESTORE_CMD.scoped(WINDOW.id()).notify();
            }
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if args.new_focus == Some(WINDOW.id()) {
                self.vars.0.children.with(|c| {
                    for &c in c.iter() {
                        let _ = WINDOWS.bring_to_top(c);
                    }
                });
            } else if let Some(new_focus) = args.new_focus {
                self.vars.0.children.with(|c| {
                    if c.contains(&new_focus) {
                        let _ = WINDOWS.bring_to_top(WINDOW.id());

                        for c in c.iter() {
                            if *c != new_focus {
                                let _ = WINDOWS.bring_to_top(WINDOW.id());
                            }
                        }

                        let _ = WINDOWS.bring_to_top(new_focus);
                    }
                });
            }
        } else if let Some(args) = MONITORS_CHANGED_EVENT.on(update) {
            if let Some(m) = &self.monitor
                && args.removed.contains(&m.id())
            {
                self.monitor = None;
                self.vars.0.actual_monitor.set(None);
            }
            self.vars.monitor().update();
        } else if let Some(args) = RAW_WINDOW_OPEN_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.waiting_view = false;

                WINDOWS.set_view(args.window_id, args.window.clone().into());

                self.window = Some(args.window.clone());
                self.cancel_ime_handle = super::cmd::CANCEL_IME_CMD.scoped(WINDOW.id()).subscribe(true);
                self.open_title_menu_handle = super::cmd::OPEN_TITLE_BAR_CONTEXT_MENU_CMD.scoped(WINDOW.id()).subscribe(true);
                self.drag_move_handle = super::cmd::DRAG_MOVE_RESIZE_CMD.scoped(WINDOW.id()).subscribe(true);

                self.vars.0.render_mode.set(args.data.render_mode);
                self.vars.state().set(args.data.state.state);
                self.actual_state = Some(args.data.state.state);
                self.vars.0.restore_state.set(args.data.state.restore_state);
                self.vars.0.restore_rect.set(args.data.state.restore_rect);
                self.vars.0.global_position.set(args.data.position.0);
                self.vars.0.actual_position.set(args.data.position.1);
                self.vars.0.actual_size.set(args.data.size);
                self.vars.0.safe_padding.set(args.data.safe_padding);
                self.vars.0.actual_monitor.set(args.data.monitor);
                self.vars.0.scale_factor.set(args.data.scale_factor);

                self.state = Some(args.data.state.clone());

                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .unwrap_or_else(|| WINDOWS.system_colors_config().scheme);
                self.vars.0.actual_color_scheme.set(scheme);
                let accent = self
                    .vars
                    .accent_color()
                    .get()
                    .or_else(|| self.parent_accent_color.as_ref().map(|t| t.get()))
                    .unwrap_or_else(|| WINDOWS.system_colors_config().accent.into());
                self.vars.0.actual_accent_color.set(accent);

                UPDATES.layout_window(args.window_id).render_window(args.window_id);

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.window);
                }
            }
        } else if let Some(args) = RAW_COLORS_CONFIG_CHANGED_EVENT.on(update) {
            let scheme = self
                .vars
                .color_scheme()
                .get()
                .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                .unwrap_or(args.config.scheme);
            self.vars.0.actual_color_scheme.set(scheme);
            let color = self
                .vars
                .accent_color()
                .get()
                .or_else(|| self.parent_accent_color.as_ref().map(|t| t.get()))
                .unwrap_or_else(|| args.config.accent.into());
            self.vars.0.actual_accent_color.set(color);
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            let w_id = WINDOW.id();
            if args.window_id == w_id && self.window.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a window, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];
                self.respawned = true;

                UPDATES.layout_window(w_id).render_window(w_id);
            }
        } else if let Some(args) = RAW_IME_EVENT.on(update) {
            let w_id = WINDOW.id();
            if args.window_id == w_id {
                match &args.ime {
                    Ime::Preview(s, c) => {
                        if let Some(info) = &mut self.ime_info {
                            info.has_preview = !s.is_empty();
                            let args = super::ImeArgs::now(info.target.clone(), s.clone(), *c);
                            super::IME_EVENT.notify(args);
                        }
                    }
                    Ime::Commit(s) => {
                        if let Some(info) = &mut self.ime_info {
                            info.has_preview = false;
                            let args = super::ImeArgs::now(info.target.clone(), s.clone(), None);
                            super::IME_EVENT.notify(args);
                        }
                    }
                }
            }
        } else if let Some(args) = ACCESS_INITED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                tracing::info!("accessibility info enabled in view for {:?}", args.window_id);
                self.vars.0.access_enabled.set(AccessEnabled::VIEW);
            }
        } else if let Some(args) = ACCESS_DEINITED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                tracing::info!("accessibility info disabled in view for {:?}", args.window_id);
                self.vars.0.access_enabled.modify(|a| {
                    if a.is_enabled() {
                        **a = AccessEnabled::APP;
                    }
                });
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update)
            && self
                .window
                .as_ref()
                .map(|w| w.renderer().generation() != Ok(args.generation))
                .unwrap_or(args.is_respawn)
        {
            debug_assert!(args.is_respawn);

            self.window = None;
            self.cancel_ime_handle = CommandHandle::dummy();
            self.open_title_menu_handle = CommandHandle::dummy();
            self.drag_move_handle = CommandHandle::dummy();
            self.waiting_view = false;
            self.delayed_view_updates = vec![];
            self.respawned = true;

            let w_id = WINDOW.id();
            UPDATES.layout_window(w_id).render_window(w_id);
        }

        self.content.pre_event(update);

        if self.ime_info.is_some() && super::cmd::CANCEL_IME_CMD.scoped(WINDOW.id()).has(update) {
            let prev = self.ime_info.take().unwrap();
            if prev.has_preview {
                let args = super::ImeArgs::now(prev.target, "", (0, 0));
                super::IME_EVENT.notify(args);
            }
            if let Some(w) = &self.window {
                let _ = w.set_ime_area(None);
            }
        } else if let Some(args) = super::cmd::DRAG_MOVE_RESIZE_CMD.scoped(WINDOW.id()).on(update) {
            let r = args.handle_enabled(&self.drag_move_handle, |args| args.param::<crate::cmd::ResizeDirection>().copied());
            if let Some(r) = r {
                self.view_task(Box::new(move |w| {
                    let _ = if let Some(r) = r {
                        w.unwrap().drag_resize(r)
                    } else {
                        w.unwrap().drag_move()
                    };
                }));
            }
        } else if let Some(args) = super::cmd::OPEN_TITLE_BAR_CONTEXT_MENU_CMD.scoped(WINDOW.id()).on(update) {
            let pos = args.handle_enabled(&self.open_title_menu_handle, |args| {
                if let Some(p) = args.param::<DipPoint>() {
                    *p
                } else if let Some(p) = args.param::<PxPoint>() {
                    p.to_dip(self.vars.scale_factor().get())
                } else {
                    DipPoint::splat(Dip::new(24))
                }
            });
            if let Some(pos) = pos {
                self.view_task(Box::new(move |w| {
                    let _ = w.unwrap().open_title_bar_context_menu(pos);
                }));
            }
        };
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    #[must_use]
    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        let prev_tree = WINDOW.info();
        let info = self.content.info(info_widgets);
        if let Some(info) = &info
            && self.window.is_some()
        {
            // updated widget info and has view-process window
            if info.access_enabled() == AccessEnabled::VIEW && self.render_access_update.is_none() {
                // view window requires access info, next frame
                self.render_access_update = Some(prev_tree);
                UPDATES.render_window(WINDOW.id());
            } else if self.ime_info.is_some() {
                UPDATES.render_window(WINDOW.id());
            }
        }

        info
    }

    fn accessible_focused(&self, info: &WidgetInfoTree) -> Option<WidgetId> {
        if WINDOWS.is_focused(info.window_id()).unwrap_or(false) {
            WINDOW_FOCUS.focused().with(|p| {
                if let Some(p) = p
                    && p.window_id() == info.window_id()
                    && let Some(wgt) = info.get(p.widget_id())
                    && let Some(wgt) = wgt.access()
                    && wgt.is_accessible()
                {
                    // is focused accessible widget inside window
                    return Some(wgt.info().id());
                }
                None
            })
        } else {
            None
        }
    }

    fn update_access_focused(&mut self) {
        if self.render_access_update.is_some() {
            // will update next frame
            return;
        }
        if let Some(view) = &self.window {
            let info = WINDOW.info();
            if info.access_enabled().is_enabled() {
                let _ = view.access_update(zng_view_api::access::AccessTreeUpdate::new(
                    vec![],
                    None,
                    self.accessible_focused(&info).unwrap_or(info.root().id()).into(),
                ));
            }
        }
    }

    fn update_ime(&mut self) {
        WINDOW_FOCUS.focused().with(|f| {
            let mut ime_path = None;
            if let Some(f) = f
                && f.interactivity().is_enabled()
                && f.window_id() == WINDOW.id()
                && super::IME_EVENT.is_subscriber(f.widget_id())
            {
                ime_path = Some(f.as_path().clone());
            }

            if ime_path.as_ref() == self.ime_info.as_ref().map(|p| &p.target) {
                return;
            }

            if let Some(p) = ime_path {
                let info = WINDOW.info();
                if let Some(w) = info.get(p.widget_id()) {
                    if let Some(prev) = self.ime_info.take()
                        && prev.has_preview
                    {
                        // clear
                        let args = super::ImeArgs::now(prev.target, "", (0, 0));
                        super::IME_EVENT.notify(args);
                    }

                    self.ime_info = Some(ImeInfo {
                        target: p.clone(),
                        has_preview: false,
                        area: DipRect::zero(),
                    });

                    if let Some(win) = &self.window {
                        let area = w.ime_area().to_dip(info.scale_factor());
                        self.ime_info.as_mut().unwrap().area = area;

                        // set to `None` to force a refresh, some IME (MS Emoji) behave like
                        // they are in the same widget still if only the position changes
                        let _ = win.set_ime_area(None);
                        let _ = win.set_ime_area(Some(area));
                    }
                    return;
                }
            }

            if let Some(prev) = self.ime_info.take() {
                if let Some(w) = &self.window {
                    let _ = w.set_ime_area(None);
                }

                if prev.has_preview {
                    // clear
                    let args = super::ImeArgs::now(prev.target, "", (0, 0));
                    super::IME_EVENT.notify(args);
                }
            }
        });
    }

    pub fn layout(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        if !layout_widgets.delivery_list().enter_window(WINDOW.id()) {
            return;
        }

        if self.window.is_some() {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }
            self.layout_update(layout_widgets);
        } else if self.respawned && !self.waiting_view {
            self.layout_respawn();
        } else if !self.waiting_view {
            self.layout_init();
        }
    }

    /// First layout, opens the window.
    fn layout_init(&mut self) {
        // await images load up to 1s.
        #[cfg(feature = "image")]
        if self.img_res.deadline.has_elapsed() {
            if let Some(icon) = &self.img_res.icon_var
                && icon.get().is_loading()
            {
                return;
            }
            if let Some(cursor) = &self.img_res.cursor_var
                && cursor.get().is_loading()
            {
                return;
            }
        }
        // update window "load" state, `is_loaded` and the `WindowLoadEvent` happen here.
        if !WINDOWS.try_load(WINDOW.id()) {
            // block on loading handles.
            return;
        }

        self.monitor = Some(self.vars.monitor().get().select_fallback());
        let m = self.monitor.as_ref().unwrap();
        self.vars.0.scale_factor.set(m.scale_factor().get());

        let scale_factor = m.scale_factor().get();
        let screen_density = m.density().get();
        let screen_rect = m.px_rect();

        // Layout min, max and size in the monitor space.
        let (min_size, max_size, mut size, root_font_size) =
            self.content.outer_layout(scale_factor, screen_density, screen_rect.size, || {
                let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
                let max_size = self.vars.max_size().layout_dft(screen_rect.size);
                let size = self.vars.size().layout_dft(default_size(scale_factor));

                let font_size = self.vars.font_size().get();
                let mut root_font_size = font_size.layout_dft_x(Length::pt_to_px(11.0, scale_factor));
                if root_font_size < Px(0) {
                    tracing::error!("invalid font size {font_size:?} => {root_font_size:?}");
                    root_font_size = Length::pt_to_px(11.0, scale_factor);
                }

                (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
            });

        self.root_font_size = root_font_size.to_dip(scale_factor);

        let state = self.vars.state().get();
        if state == WindowState::Normal && self.vars.auto_size().get() != AutoSize::DISABLED {
            // layout content to get auto-size size.
            size = self.content.layout(
                Arc::default(),
                scale_factor,
                screen_density,
                min_size,
                max_size,
                size,
                root_font_size,
                false,
            );
        }

        // Layout initial position in the monitor space.
        let mut system_pos = false;
        let position = match self.start_position {
            StartPosition::Default => {
                let pos = self.vars.position().get();
                if pos.x.is_default() || pos.y.is_default() {
                    system_pos = true;
                    screen_rect.origin + PxVector::splat(Px(40))
                } else {
                    self.content.outer_layout(scale_factor, screen_density, screen_rect.size, || {
                        pos.layout() + screen_rect.origin.to_vector()
                    })
                }
            }
            StartPosition::CenterMonitor => {
                PxPoint::new(
                    (screen_rect.size.width - size.width) / Px(2),
                    (screen_rect.size.height - size.height) / Px(2),
                ) + screen_rect.origin.to_vector()
            }
            StartPosition::CenterParent => {
                // center monitor if no parent
                let mut parent_rect = screen_rect;

                if let Some(parent) = self.vars.parent().get()
                    && let Ok(w) = WINDOWS.vars(parent)
                {
                    let factor = w.scale_factor().get();
                    let pos = w.actual_position().get().to_px(factor);
                    let size = w.actual_size().get().to_px(factor);

                    parent_rect = PxRect::new(pos, size);
                }

                PxPoint::new(
                    (parent_rect.size.width - size.width) / Px(2),
                    (parent_rect.size.height - size.height) / Px(2),
                ) + parent_rect.origin.to_vector()
            }
        };

        // send view window request:

        let m_position = (position - screen_rect.origin.to_vector()).to_dip(scale_factor);
        let size = size.to_dip(scale_factor);

        let state = WindowStateAll::new(
            state,
            position,
            DipRect::new(m_position, size),
            WindowState::Normal,
            min_size.to_dip(scale_factor),
            max_size.to_dip(scale_factor),
            self.vars.chrome().get() && !WINDOWS.system_chrome().get().needs_custom(),
        );

        let window_id = WINDOW.id();

        let request = WindowRequest::new(
            zng_view_api::window::WindowId::from_raw(window_id.get()),
            self.vars.title().get(),
            state.clone(),
            self.kiosk.is_some(),
            system_pos,
            self.vars.video_mode().get(),
            self.vars.visible().get(),
            self.vars.taskbar_visible().get(),
            self.vars.always_on_top().get(),
            self.vars.movable().get(),
            self.vars.resizable().get(),
            {
                #[cfg(feature = "image")]
                {
                    self.img_res
                        .icon_var
                        .as_ref()
                        .and_then(|ico| ico.get().view().map(|ico| ico.id()))
                        .flatten()
                }
                #[cfg(not(feature = "image"))]
                None
            },
            self.vars.cursor().with(|c| c.icon()),
            {
                #[cfg(feature = "image")]
                {
                    self.vars
                        .actual_cursor_img()
                        .get()
                        .and_then(|(i, h)| i.view().and_then(|i| i.id()).map(|i| (i, h)))
                }
                #[cfg(not(feature = "image"))]
                None
            },
            self.transparent,
            {
                #[cfg(feature = "image")]
                {
                    matches!(self.vars.frame_capture_mode().get(), FrameCaptureMode::All)
                }

                #[cfg(not(feature = "image"))]
                false
            },
            self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
            self.vars.focus_indicator().get(),
            self.start_focused,
            self.ime_info.as_ref().and_then(|a| {
                let area = WINDOW.info().get(a.target.widget_id())?.ime_area().to_dip(scale_factor);
                Some(area)
            }),
            self.vars.enabled_buttons().get(),
            self.vars.system_shutdown_warn().get(),
            WINDOWS.take_view_extensions_init(window_id),
        );

        if let Ok(()) = VIEW_PROCESS.open_window(request) {
            self.state = Some(state);
            self.waiting_view = true;
        } // else respawn
    }

    /// Layout for already open window.
    fn layout_update(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        let mut state = match self.state.clone() {
            Some(s) => s,
            None => {
                tracing::warn!("layout update ignored due to respawn");
                return;
            }
        };

        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().get();
        let screen_density = m.density().get();

        let current_size = self.vars.0.actual_size.get().to_px(scale_factor);
        let mut size = current_size;
        let min_size = state.min_size.to_px(scale_factor);
        let max_size = state.max_size.to_px(scale_factor);
        let root_font_size = self.root_font_size.to_px(scale_factor);

        let skip_auto_size = !matches!(state.state, WindowState::Normal);

        if !skip_auto_size {
            let auto_size = self.vars.auto_size().get();

            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                size.width = max_size.width;
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                size.height = max_size.height;
            }
        }

        let size = self.content.layout(
            layout_widgets,
            scale_factor,
            screen_density,
            min_size,
            max_size,
            size,
            root_font_size,
            skip_auto_size,
        );

        if size != current_size {
            assert!(!skip_auto_size);

            let auto_size_origin = self.vars.auto_size_origin().get();
            let auto_size_origin = |size| {
                let metrics = LayoutMetrics::new(scale_factor, size, root_font_size)
                    .with_screen_density(screen_density)
                    .with_direction(DIRECTION_VAR.get());
                LAYOUT.with_context(metrics, || auto_size_origin.layout().to_dip(scale_factor))
            };
            let prev_origin = auto_size_origin(current_size);
            let new_origin = auto_size_origin(size);

            let size = size.to_dip(scale_factor);

            state.restore_rect.size = size;
            state.restore_rect.origin += prev_origin - new_origin;

            if let Some(view) = &self.window {
                let _: Ignore = view.set_state(state);
            } else {
                debug_assert!(self.respawned);
                self.state = Some(state);
            }
        }
    }

    /// First layout after respawn, opens the window but used previous sizes.
    fn layout_respawn(&mut self) {
        if self.monitor.is_none() {
            self.monitor = Some(self.vars.monitor().get().select_fallback());
            let m = self.monitor.as_ref().unwrap();
            self.vars.0.scale_factor.set(m.scale_factor().get());
        }

        self.layout_update(Arc::default());

        let window_id = WINDOW.id();

        let request = WindowRequest::new(
            zng_view_api::window::WindowId::from_raw(window_id.get()),
            self.vars.title().get(),
            self.state.clone().unwrap(),
            self.kiosk.is_some(),
            false,
            self.vars.video_mode().get(),
            self.vars.visible().get(),
            self.vars.taskbar_visible().get(),
            self.vars.always_on_top().get(),
            self.vars.movable().get(),
            self.vars.resizable().get(),
            {
                #[cfg(feature = "image")]
                {
                    self.img_res
                        .icon_var
                        .as_ref()
                        .and_then(|ico| ico.get().view().map(|ico| ico.id()))
                        .flatten()
                }
                #[cfg(not(feature = "image"))]
                None
            },
            self.vars.cursor().with(|c| c.icon()),
            {
                #[cfg(feature = "image")]
                {
                    self.vars
                        .actual_cursor_img()
                        .get()
                        .and_then(|(i, h)| i.view().and_then(|i| i.id()).map(|i| (i, h)))
                }
                #[cfg(not(feature = "image"))]
                None
            },
            self.transparent,
            {
                #[cfg(feature = "image")]
                {
                    matches!(self.vars.frame_capture_mode().get(), FrameCaptureMode::All)
                }
                #[cfg(not(feature = "image"))]
                false
            },
            self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
            self.vars.focus_indicator().get(),
            WINDOWS.is_focused(WINDOW.id()).unwrap_or(false),
            self.ime_info.as_ref().and_then(|a| {
                let info = WINDOW.info();
                let area = info.get(a.target.widget_id())?.ime_area().to_dip(info.scale_factor());
                Some(area)
            }),
            self.vars.enabled_buttons().get(),
            self.vars.system_shutdown_warn().get(),
            WINDOWS.take_view_extensions_init(window_id),
        );

        if let Ok(()) = VIEW_PROCESS.open_window(request) {
            self.waiting_view = true
        }
    }

    pub fn render(&mut self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        let w_id = WINDOW.id();
        if !render_widgets.delivery_list().enter_window(w_id) && !render_update_widgets.delivery_list().enter_window(w_id) {
            return;
        }

        if let Some(view) = &self.window {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }

            let scale_factor = self.monitor.as_ref().unwrap().scale_factor().get();
            self.content.render(
                Some(view.renderer()),
                scale_factor,
                self.resize_wait_id.take(),
                render_widgets,
                render_update_widgets,
            );

            if let Some(prev_tree) = self.render_access_update.take() {
                let info = WINDOW.info();
                // info was rebuild before this frame
                if let Some(mut update) = info.to_access_updates(&prev_tree) {
                    // updated access info
                    update.focused = self.accessible_focused(&info).unwrap_or_else(|| info.root().id()).into();
                    let _ = view.access_update(update);
                }
            } else {
                let info = WINDOW.info();
                if info.access_enabled() == AccessEnabled::VIEW
                    && let Some(mut update) = info.to_access_updates_bounds()
                {
                    // updated transforms or visibility access info
                    update.focused = self.accessible_focused(&info).unwrap_or_else(|| info.root().id()).into();
                    let _ = view.access_update(update);
                }
            }

            if let Some(ime) = &mut self.ime_info
                && let Some(w) = &self.window
            {
                let info = WINDOW.info();
                if let Some(wgt) = info.get(ime.target.widget_id()) {
                    let area = wgt.ime_area().to_dip(scale_factor);
                    if ime.area != area {
                        ime.area = area;
                        let _ = w.set_ime_area(Some(area));
                    }
                }
            }
        }
    }

    pub fn focus(&mut self) {
        self.update_gen(|view| {
            let r = view.focus();
            if let Ok(FocusResult::AlreadyFocused) = r {
                let prev = WINDOWS.focused_window_id();
                let new = Some(WINDOW.id());
                if prev != new {
                    // probably prev is a nested window
                    RAW_WINDOW_FOCUS_EVENT.notify(RawWindowFocusArgs::now(prev, new));
                }
            }
        });
    }

    pub fn start_drag_drop(&mut self, data: Vec<DragDropData>, allowed_effects: DragDropEffect) -> Result<DragDropId, DragDropError> {
        if let Some(view) = &self.window
            && let Ok(r) = view.start_drag_drop(data, allowed_effects)
        {
            return r;
        }
        Err(DragDropError::CannotStart("view not available".into()))
    }

    pub fn drag_dropped(&mut self, drop_id: DragDropId, applied: DragDropEffect) {
        if let Some(view) = &self.window {
            let _ = view.drag_dropped(drop_id, applied);
        }
    }

    pub fn bring_to_top(&mut self) {
        self.update_gen(|view| {
            let _ = view.bring_to_top();
        });
    }

    pub fn close(&mut self) {
        self.content.close();
        self.window = None;
        self.cancel_ime_handle = CommandHandle::dummy();
        self.cancel_ime_handle = CommandHandle::dummy();
    }

    fn view_task(&mut self, task: Box<dyn FnOnce(Option<&ViewWindow>) + Send>) {
        if let Some(view) = &self.window {
            task(Some(view));
        } else if self.waiting_view {
            self.delayed_view_updates.push(Box::new(move |v| task(Some(v))));
        } else {
            task(None);
        }
    }
}

/// Respond to `parent_var` updates, returns `true` if the `parent` value has changed.
fn update_parent(parent: &mut Option<WindowId>, vars: &WindowVars) -> bool {
    let parent_var = vars.parent();
    if let Some(parent_id) = parent_var.get_new() {
        if parent_id == *parent {
            return false;
        }

        match parent_id {
            Some(mut parent_id) => {
                if parent_id == WINDOW.id() {
                    tracing::error!("cannot set `{:?}` as it's own parent", parent_id);
                    parent_var.set(*parent);
                    return false;
                }
                if !vars.0.children.with(|c| c.is_empty()) {
                    tracing::error!("cannot set parent for `{:?}` because it already has children", WINDOW.id());
                    parent_var.set(*parent);
                    return false;
                }

                if let Ok(parent_vars) = WINDOWS.vars(parent_id) {
                    // redirect to parent's parent.
                    if let Some(grand) = parent_vars.parent().get() {
                        tracing::debug!("using `{grand:?}` as parent, because it is the parent of requested `{parent_id:?}`");
                        parent_var.set(grand);

                        parent_id = grand;
                        if Some(parent_id) == *parent {
                            return false;
                        }
                    }

                    // remove previous
                    if let Some(parent_id) = parent.take()
                        && let Ok(parent_vars) = WINDOWS.vars(parent_id)
                    {
                        let id = WINDOW.id();
                        parent_vars.0.children.modify(move |c| {
                            c.remove(&id);
                        });
                    }

                    // insert new
                    *parent = Some(parent_id);
                    let id = WINDOW.id();
                    parent_vars.0.children.modify(move |c| {
                        c.insert(id);
                    });

                    true
                } else {
                    tracing::error!("cannot use `{:?}` as a parent because it does not exist", parent_id);
                    parent_var.set(*parent);
                    false
                }
            }
            None => {
                if let Some(parent_id) = parent.take() {
                    if let Ok(parent_vars) = WINDOWS.vars(parent_id) {
                        let id = WINDOW.id();
                        parent_vars.0.children.modify(move |c| {
                            c.remove(&id);
                        });
                    }
                    true
                } else {
                    false
                }
            }
        }
    } else {
        false
    }
}

/// Implementer of `App <-> View` sync in a headless window.
struct HeadlessWithRendererCtrl {
    surface: Option<ViewHeadless>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewHeadless) + Send>>,
    vars: WindowVars,
    content: ContentCtrl,

    // init config.
    render_mode: Option<RenderMode>,
    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,

    // current state.
    size: DipSize,

    actual_parent: Option<WindowId>,
    /// actual_color_scheme and scale_factor binding.
    var_bindings: VarHandles,
}
impl HeadlessWithRendererCtrl {
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: WindowRoot) -> Self {
        Self {
            surface: None,
            waiting_view: false,
            delayed_view_updates: vec![],
            vars: vars.clone(),

            render_mode: content.render_mode,
            headless_monitor: content.headless_monitor,
            headless_simulator: HeadlessSimulator::new(),

            content: ContentCtrl::new(vars.clone(), commands, content),

            actual_parent: None,
            size: DipSize::zero(),
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        if self.vars.0.access_enabled.is_new() {
            UPDATES.update_info_window(WINDOW.id());
        }

        if self.surface.is_some() {
            if self.vars.size().is_new()
                || self.vars.min_size().is_new()
                || self.vars.max_size().is_new()
                || self.vars.auto_size().is_new()
                || self.vars.font_size().is_new()
            {
                UPDATES.layout_window(WINDOW.id());
            }
        } else {
            // we init on the first layout.
            UPDATES.layout_window(WINDOW.id());
        }

        if update_parent(&mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(self.headless_monitor.scale_factor, &self.vars);
        }

        self.content.update(update_widgets);
    }

    #[must_use]
    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        self.content.info(info_widgets)
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if let Some(args) = RAW_HEADLESS_OPEN_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.waiting_view = false;

                WINDOWS.set_view(args.window_id, args.surface.clone().into());

                self.surface = Some(args.surface.clone());
                self.vars.0.render_mode.set(args.data.render_mode);

                UPDATES.render_window(args.window_id);

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.surface);
                }
            }
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            if args.window_id == WINDOW.id() && self.surface.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a headless surface, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];

                UPDATES.layout_window(args.window_id).render_window(args.window_id);
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update)
            && let Some(view) = &self.surface
            && view.renderer().generation() != Ok(args.generation)
        {
            debug_assert!(args.is_respawn);

            self.surface = None;

            let w_id = WINDOW.id();
            UPDATES.layout_window(w_id).render_window(w_id);
        }

        self.content.pre_event(update);

        self.headless_simulator.pre_event(update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    pub fn layout(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        if !layout_widgets.delivery_list().enter_window(WINDOW.id()) {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_density = self.headless_monitor.density;
        let screen_size = self.headless_monitor.size.to_px(scale_factor);

        let (min_size, max_size, size, root_font_size) = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
            let max_size = self.vars.max_size().layout_dft(screen_size);
            let size = self.vars.size().layout_dft(default_size(scale_factor));
            let root_font_size = self.vars.font_size().layout_dft_x(Length::pt_to_px(11.0, scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
        });

        let size = self.content.layout(
            layout_widgets,
            scale_factor,
            screen_density,
            min_size,
            max_size,
            size,
            root_font_size,
            false,
        );
        let size = size.to_dip(scale_factor);

        if let Some(view) = &self.surface {
            // already has surface, maybe resize:
            if self.size != size {
                self.size = size;
                let _: Ignore = view.set_size(size, scale_factor);
            }
        } else if !self.waiting_view {
            // (re)spawn the view surface:

            if !WINDOWS.try_load(WINDOW.id()) {
                return;
            }

            let render_mode = self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get());

            let window_id = WINDOW.id();

            let r = VIEW_PROCESS.open_headless(HeadlessRequest::new(
                zng_view_api::window::WindowId::from_raw(window_id.get()),
                scale_factor,
                size,
                render_mode,
                WINDOWS.take_view_extensions_init(window_id),
            ));

            if let Ok(()) = r {
                self.waiting_view = true
            }
        }

        self.headless_simulator.layout();
    }

    pub fn render(&mut self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        let w_id = WINDOW.id();
        if !render_widgets.delivery_list().enter_window(w_id) && !render_update_widgets.delivery_list().enter_window(w_id) {
            return;
        }

        if let Some(view) = &self.surface {
            let fct = self.vars.0.scale_factor.get();
            self.content
                .render(Some(view.renderer()), fct, None, render_widgets, render_update_widgets);
        }
    }

    pub fn focus(&mut self) {
        self.headless_simulator.focus();
    }

    pub fn bring_to_top(&mut self) {
        self.headless_simulator.bring_to_top();
    }

    pub fn close(&mut self) {
        self.content.close();
        self.surface = None;
    }

    pub fn start_drag_drop(&mut self, data: Vec<DragDropData>, allowed_effects: DragDropEffect) -> Result<DragDropId, DragDropError> {
        let _ = (data, allowed_effects);
        Err(DragDropError::CannotStart("cannot start drag&drop from headless window".into()))
    }

    pub fn drag_dropped(&mut self, drop_id: DragDropId, applied: DragDropEffect) {
        let _ = (drop_id, applied);
    }

    fn view_task(&mut self, task: Box<dyn FnOnce(Option<&ViewWindow>) + Send>) {
        task(None)
    }
}

fn update_headless_vars(m_factor: Option<Factor>, h_vars: &WindowVars) -> VarHandles {
    let mut handles = VarHandles::dummy();

    if let Some(f) = m_factor {
        h_vars.0.scale_factor.set(f);
    }

    if let Some(parent_vars) = h_vars.parent().get().and_then(|id| WINDOWS.vars(id).ok()) {
        // bind parent factor
        if m_factor.is_none() {
            h_vars.0.scale_factor.set_from(&parent_vars.0.scale_factor);
            handles.push(parent_vars.0.scale_factor.bind(&h_vars.0.scale_factor));
        }

        // merge bind color scheme.
        let user = h_vars.color_scheme();
        let parent = &parent_vars.0.actual_color_scheme;
        let actual = &h_vars.0.actual_color_scheme;

        handles.push(user.hook(clmv!(parent, actual, |args| {
            let value = *args.value();
            let scheme = value.unwrap_or_else(|| parent.get());
            actual.set(scheme);
            true
        })));

        handles.push(parent.hook(clmv!(user, actual, |args| {
            let scheme = user.get().unwrap_or_else(|| *args.value());
            actual.set(scheme);
            true
        })));

        actual.modify(clmv!(user, parent, |a| {
            let value = user.get().unwrap_or_else(|| parent.get());
            a.set(value);
        }));
    } else {
        // set-bind color scheme
        let from = h_vars.color_scheme();
        let to = &h_vars.0.actual_color_scheme;

        to.set_from_map(&from, |&s| s.unwrap_or_default());
        handles.push(from.bind_map(to, |&s| s.unwrap_or_default()));
    }

    handles
}

/// implementer of `App` only content management.
struct HeadlessCtrl {
    vars: WindowVars,
    content: ContentCtrl,

    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,

    actual_parent: Option<WindowId>,
    /// actual_color_scheme and scale_factor binding.
    var_bindings: VarHandles,
}
impl HeadlessCtrl {
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: WindowRoot) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(vars.clone(), commands, content),
            headless_simulator: HeadlessSimulator::new(),
            actual_parent: None,
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        if self.vars.0.access_enabled.is_new() {
            UPDATES.update_info_window(WINDOW.id());
        }

        if self.vars.size().is_new() || self.vars.min_size().is_new() || self.vars.max_size().is_new() || self.vars.auto_size().is_new() {
            UPDATES.layout_window(WINDOW.id());
        }

        if matches!(self.content.init_state, InitState::Init) {
            let w_id = WINDOW.id();
            UPDATES.layout_window(w_id).render_window(w_id);
        }

        if update_parent(&mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(self.headless_monitor.scale_factor, &self.vars);
        }

        self.content.update(update_widgets);
    }

    #[must_use]
    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        self.content.info(info_widgets)
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        self.content.pre_event(update);
        self.headless_simulator.pre_event(update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    pub fn layout(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        let w_id = WINDOW.id();
        if !layout_widgets.delivery_list().enter_window(w_id) {
            return;
        }

        if !WINDOWS.try_load(w_id) {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_density = self.headless_monitor.density;
        let screen_size = self.headless_monitor.size.to_px(scale_factor);

        let (min_size, max_size, size, root_font_size) = self.content.outer_layout(scale_factor, screen_density, screen_size, || {
            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
            let max_size = self.vars.max_size().layout_dft(screen_size);
            let size = self.vars.size().layout_dft(default_size(scale_factor));
            let root_font_size = self.vars.font_size().layout_dft_x(Length::pt_to_px(11.0, scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
        });

        let _surface_size = self.content.layout(
            layout_widgets,
            scale_factor,
            screen_density,
            min_size,
            max_size,
            size,
            root_font_size,
            false,
        );

        self.headless_simulator.layout();
    }

    pub fn render(&mut self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        let w_id = WINDOW.id();
        if !render_widgets.delivery_list().enter_window(w_id) && !render_update_widgets.delivery_list().enter_window(w_id) {
            return;
        }

        // layout and render cannot happen yet
        if !WINDOWS.try_load(w_id) {
            return;
        }

        let fct = self.vars.0.scale_factor.get();
        self.content.render(None, fct, None, render_widgets, render_update_widgets);
    }

    pub fn focus(&mut self) {
        self.headless_simulator.focus();
    }

    pub fn bring_to_top(&mut self) {
        self.headless_simulator.bring_to_top();
    }

    pub fn close(&mut self) {
        self.content.close();
    }

    pub fn start_drag_drop(&mut self, data: Vec<DragDropData>, allowed_effects: DragDropEffect) -> Result<DragDropId, DragDropError> {
        let _ = (data, allowed_effects);
        Err(DragDropError::CannotStart("cannot start drag&drop from headless window".into()))
    }

    pub fn drag_dropped(&mut self, drop_id: DragDropId, applied: DragDropEffect) {
        let _ = (drop_id, applied);
    }

    fn view_task(&mut self, task: Box<dyn FnOnce(Option<&ViewWindow>) + Send>) {
        task(None);
    }
}

/// Implementer of headless apps simulation of headed events for tests.
struct HeadlessSimulator {
    is_enabled: Option<bool>,
    is_open: bool,
}
impl HeadlessSimulator {
    fn new() -> Self {
        HeadlessSimulator {
            is_enabled: None,
            is_open: false,
        }
    }

    fn enabled(&mut self) -> bool {
        *self.is_enabled.get_or_insert_with(|| zng_app::APP.window_mode().is_headless())
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if self.enabled() && self.is_open && VIEW_PROCESS_INITED_EVENT.on(update).map(|a| a.is_respawn).unwrap_or(false) {
            self.is_open = false;
        }
    }

    pub fn layout(&mut self) {
        if self.enabled() && !self.is_open {
            self.is_open = true;
            self.focus();
        }
    }

    pub fn focus(&mut self) {
        let args = RawWindowFocusArgs::now(WINDOWS.focused_window_id(), Some(WINDOW.id()));
        RAW_WINDOW_FOCUS_EVENT.notify(args);
    }

    pub fn bring_to_top(&mut self) {
        // we don't have "bring-to-top" event.
    }
}

#[derive(Clone, Copy)]
enum InitState {
    /// We let one update cycle happen before init
    /// to let the constructor closure setup vars
    /// that are read on init.
    SkipOne,
    Init,
    Inited,
}

/// Implementer of window UI node tree initialization and management.
struct ContentCtrl {
    vars: WindowVars,
    commands: WindowCommands,

    root_ctx: WidgetCtx,
    root: UiNode,
    layout_pass: LayoutPassId,

    init_state: InitState,
    frame_id: FrameId,
    clear_color: Rgba,
}
impl ContentCtrl {
    pub fn new(vars: WindowVars, commands: WindowCommands, window: WindowRoot) -> Self {
        Self {
            vars,
            commands,

            root_ctx: WidgetCtx::new(window.id),
            root: window.child,

            layout_pass: LayoutPassId::new(),

            init_state: InitState::SkipOne,
            frame_id: FrameId::INVALID,
            clear_color: colors::BLACK,
        }
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        match self.init_state {
            InitState::Inited => {
                self.commands.update(&self.vars);

                update_widgets.with_window(|| {
                    if self.root_ctx.take_reinit() {
                        // like WidgetBase, pending reinit cancels update
                        WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                            self.root.deinit();
                            self.root.init();
                        });
                        let _ = self.root_ctx.take_reinit(); // ignore after init
                    } else {
                        // no pending reinit, can update
                        WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                            update_widgets.with_widget(|| {
                                self.root.update(update_widgets);
                            });
                        });

                        // update requested reinit
                        if self.root_ctx.take_reinit() {
                            WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                                self.root.deinit();
                                self.root.init();
                            });
                        }
                    }
                });
            }

            InitState::SkipOne => {
                UPDATES.update(None);
                self.init_state = InitState::Init;
            }
            InitState::Init => {
                self.commands.init(&self.vars);
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                    self.root.init();
                    // requests info, layout and render just in case `root` is a blank.
                    WIDGET.update_info().layout().render();

                    super::WINDOW_OPEN_EVENT.notify(super::WindowOpenArgs::now(WINDOW.id()));
                });
                self.init_state = InitState::Inited;
                self.root_ctx.take_reinit(); // ignore reinit request (same as WidgetBase).
            }
        }
    }

    #[must_use]
    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        let win_id = WINDOW.id();
        if info_widgets.delivery_list().enter_window(win_id) && matches!(self.init_state, InitState::Inited) {
            let mut info = WidgetInfoBuilder::new(
                info_widgets,
                win_id,
                self.vars.0.access_enabled.get(),
                self.root_ctx.id(),
                self.root_ctx.bounds(),
                self.root_ctx.border(),
                self.vars.0.scale_factor.get(),
            );

            WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                self.root.info(&mut info);
            });

            let info = info.finalize(Some(WINDOW.info()), true);

            WINDOWS.set_widget_tree(info.clone());

            if self.root_ctx.is_pending_reinit() {
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
            }

            Some(info)
        } else {
            None
        }
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        #[cfg(feature = "image")]
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                let image = args.frame_image.as_ref().cloned().map(Img::new);

                let args = FrameImageReadyArgs::new(args.timestamp, args.propagation().clone(), args.window_id, args.frame_id, image);
                FRAME_IMAGE_READY_EVENT.notify(args);
            }
            return;
        }

        self.commands.event(&self.vars, update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        update.with_window(|| {
            if !matches!(self.init_state, InitState::Inited) {
                tracing::error!("cannot deliver `{:?}`, window `{}` is not inited", update.event(), WINDOW.id());
                return;
            }

            if self.root_ctx.take_reinit() {
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                    self.root.deinit();
                    self.root.init();
                });
                let _ = self.root_ctx.take_reinit(); // ignore after init
            }

            WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                update.with_widget(|| {
                    self.root.event(update);
                })
            });

            if self.root_ctx.take_reinit() {
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                    self.root.deinit();
                    self.root.init();
                });
            }
        });
    }

    pub fn close(&mut self) {
        WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
            self.root.deinit();
        });

        self.vars.0.is_open.set(false);
        self.root_ctx.deinit(false);
    }

    /// Run an `action` in the context of a monitor screen that is parent of this content.
    pub fn outer_layout<R>(
        &mut self,
        scale_factor: Factor,
        screen_density: PxDensity,
        screen_size: PxSize,
        action: impl FnOnce() -> R,
    ) -> R {
        let metrics = LayoutMetrics::new(scale_factor, screen_size, Length::pt_to_px(11.0, scale_factor))
            .with_screen_density(screen_density)
            .with_direction(DIRECTION_VAR.get());
        LAYOUT.with_context(metrics, action)
    }

    /// Layout content if there was a pending request, returns `Some(final_size)`.
    #[expect(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        layout_widgets: Arc<LayoutUpdates>,
        scale_factor: Factor,
        screen_density: PxDensity,
        min_size: PxSize,
        max_size: PxSize,
        size: PxSize,
        root_font_size: Px,
        skip_auto_size: bool,
    ) -> PxSize {
        if !matches!(self.init_state, InitState::Inited) {
            return PxSize::zero();
        }

        let _s = tracing::trace_span!("window.on_layout", window = %WINDOW.id().sequential()).entered();

        let auto_size = self.vars.auto_size().get();

        self.layout_pass = self.layout_pass.next();

        let final_size = WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
            let metrics = LayoutMetrics::new(scale_factor, size, root_font_size)
                .with_screen_density(screen_density)
                .with_direction(DIRECTION_VAR.get());
            LAYOUT.with_root_context(self.layout_pass, metrics, || {
                let mut root_cons = LAYOUT.constraints();
                if !skip_auto_size {
                    if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        root_cons.x = PxConstraints::new_range(min_size.width, max_size.width);
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        root_cons.y = PxConstraints::new_range(min_size.height, max_size.height);
                    }
                }
                let desired_size = LAYOUT.with_constraints(root_cons, || {
                    WidgetLayout::with_root_widget(layout_widgets, |wl| self.root.layout(wl))
                });

                let mut final_size = size;
                if !skip_auto_size {
                    if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        final_size.width = desired_size.width.max(min_size.width).min(max_size.width);
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        final_size.height = desired_size.height.max(min_size.height).min(max_size.height);
                    }
                }

                final_size
            })
        });

        if self.root_ctx.is_pending_reinit() {
            WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
        }

        final_size
    }

    pub fn render(
        &mut self,
        renderer: Option<ViewRenderer>,
        scale_factor: Factor,
        wait_id: Option<FrameWaitId>,
        render_widgets: Arc<RenderUpdates>,
        render_update_widgets: Arc<RenderUpdates>,
    ) {
        if !matches!(self.init_state, InitState::Inited) {
            return;
        }

        let w_id = WINDOW.id();
        if render_widgets.delivery_list().enter_window(w_id) {
            // RENDER FULL FRAME
            let _s = tracing::trace_span!("window.on_render", window = %WINDOW.id().sequential()).entered();

            self.frame_id = self.frame_id.next();

            let mut frame = FrameBuilder::new(
                render_widgets,
                render_update_widgets,
                self.frame_id,
                self.root_ctx.id(),
                &self.root_ctx.bounds(),
                &WINDOW.info(),
                renderer.clone(),
                scale_factor,
                FontAntiAliasing::Default,
            );

            let frame = WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                self.root.render(&mut frame);
                frame.finalize(&WINDOW.info())
            });

            if self.root_ctx.is_pending_reinit() {
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
            }

            self.clear_color = frame.clear_color;

            #[cfg(feature = "image")]
            let capture = self.take_frame_capture();
            #[cfg(not(feature = "image"))]
            let capture = FrameCapture::None;

            if let Some(renderer) = renderer {
                let _: Ignore = renderer.render(FrameRequest::new(
                    self.frame_id,
                    self.clear_color,
                    frame.display_list,
                    capture,
                    wait_id,
                ));
            } else {
                // simulate frame in headless
                #[cfg(feature = "image")]
                FRAME_IMAGE_READY_EVENT.notify(FrameImageReadyArgs::now(WINDOW.id(), self.frame_id, None));
            }
        } else if render_update_widgets.delivery_list().enter_window(w_id) {
            // RENDER UPDATE
            let _s = tracing::trace_span!("window.on_render_update", window = %WINDOW.id().sequential()).entered();

            self.frame_id = self.frame_id.next_update();

            let mut update = FrameUpdate::new(
                render_update_widgets,
                self.frame_id,
                self.root_ctx.id(),
                self.root_ctx.bounds(),
                self.clear_color,
            );

            let update = WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || {
                self.root.render_update(&mut update);
                update.finalize(&WINDOW.info())
            });

            if let Some(c) = update.clear_color {
                self.clear_color = c;
            }

            if self.root_ctx.is_pending_reinit() {
                WIDGET.with_context(&mut self.root_ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
            }

            #[cfg(feature = "image")]
            let capture = self.take_frame_capture();
            #[cfg(not(feature = "image"))]
            let capture = FrameCapture::None;

            if let Some(renderer) = renderer {
                let _: Ignore = renderer.render_update(FrameUpdateRequest::new(
                    self.frame_id,
                    update.transforms,
                    update.floats,
                    update.colors,
                    update.clear_color,
                    capture,
                    wait_id,
                    update.extensions,
                ));
            } else {
                // simulate frame in headless
                #[cfg(feature = "image")]
                FRAME_IMAGE_READY_EVENT.notify(FrameImageReadyArgs::now(WINDOW.id(), self.frame_id, None));
            }
        }
    }
    #[cfg(feature = "image")]
    fn take_frame_capture(&self) -> FrameCapture {
        match self.vars.frame_capture_mode().get() {
            FrameCaptureMode::Sporadic => FrameCapture::None,
            FrameCaptureMode::Next => {
                self.vars.frame_capture_mode().set(FrameCaptureMode::Sporadic);
                FrameCapture::Full
            }
            FrameCaptureMode::All => FrameCapture::Full,
            FrameCaptureMode::NextMask(m) => {
                self.vars.frame_capture_mode().set(FrameCaptureMode::Sporadic);
                FrameCapture::Mask(m)
            }
            FrameCaptureMode::AllMask(m) => FrameCapture::Mask(m),
        }
    }
}

/// Management of window content and synchronization of WindowVars and View-Process.
pub(super) struct WindowCtrl(WindowCtrlMode);
#[allow(clippy::large_enum_variant)] // headed control is the largest, but also the most common
enum WindowCtrlMode {
    Headed(HeadedCtrl),
    Headless(HeadlessCtrl),
    HeadlessWithRenderer(HeadlessWithRendererCtrl),
    Nested(NestedCtrl),
}
impl WindowCtrl {
    pub fn new(vars: &WindowVars, commands: WindowCommands, mode: WindowMode, content: WindowRoot) -> Self {
        WindowCtrl(match mode {
            WindowMode::Headed => WindowCtrlMode::Headed(HeadedCtrl::new(vars, commands, content)),
            WindowMode::Headless => WindowCtrlMode::Headless(HeadlessCtrl::new(vars, commands, content)),
            WindowMode::HeadlessWithRenderer => {
                WindowCtrlMode::HeadlessWithRenderer(HeadlessWithRendererCtrl::new(vars, commands, content))
            }
        })
    }

    pub fn new_nested(c: Arc<Mutex<NestedContentCtrl>>) -> Self {
        WindowCtrl(WindowCtrlMode::Nested(NestedCtrl::new(c)))
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.update(update_widgets),
            WindowCtrlMode::Headless(c) => c.update(update_widgets),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.update(update_widgets),
            WindowCtrlMode::Nested(c) => c.update(update_widgets),
        }
    }

    #[must_use]
    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.info(info_widgets),
            WindowCtrlMode::Headless(c) => c.info(info_widgets),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.info(info_widgets),
            WindowCtrlMode::Nested(c) => c.info(info_widgets),
        }
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.pre_event(update),
            WindowCtrlMode::Headless(c) => c.pre_event(update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.pre_event(update),
            WindowCtrlMode::Nested(c) => c.pre_event(update),
        }
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.ui_event(update),
            WindowCtrlMode::Headless(c) => c.ui_event(update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.ui_event(update),
            WindowCtrlMode::Nested(c) => c.ui_event(update),
        }
    }

    pub fn layout(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.layout(layout_widgets),
            WindowCtrlMode::Headless(c) => c.layout(layout_widgets),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.layout(layout_widgets),
            WindowCtrlMode::Nested(c) => c.layout(layout_widgets),
        }
    }

    pub fn render(&mut self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.render(render_widgets, render_update_widgets),
            WindowCtrlMode::Headless(c) => c.render(render_widgets, render_update_widgets),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.render(render_widgets, render_update_widgets),
            WindowCtrlMode::Nested(c) => c.render(render_widgets, render_update_widgets),
        }
    }

    pub fn focus(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.focus(),
            WindowCtrlMode::Headless(c) => c.focus(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.focus(),
            WindowCtrlMode::Nested(c) => c.focus(),
        }
    }

    pub fn start_drag_drop(&mut self, data: Vec<DragDropData>, allowed_effects: DragDropEffect) -> Result<DragDropId, DragDropError> {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.start_drag_drop(data, allowed_effects),
            WindowCtrlMode::Headless(c) => c.start_drag_drop(data, allowed_effects),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.start_drag_drop(data, allowed_effects),
            WindowCtrlMode::Nested(c) => c.start_drag_drop(data, allowed_effects),
        }
    }

    pub fn drag_dropped(&mut self, drop_id: DragDropId, applied: DragDropEffect) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.drag_dropped(drop_id, applied),
            WindowCtrlMode::Headless(c) => c.drag_dropped(drop_id, applied),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.drag_dropped(drop_id, applied),
            WindowCtrlMode::Nested(c) => c.drag_dropped(drop_id, applied),
        }
    }

    pub fn bring_to_top(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.bring_to_top(),
            WindowCtrlMode::Headless(c) => c.bring_to_top(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.bring_to_top(),
            WindowCtrlMode::Nested(c) => c.bring_to_top(),
        }
    }

    pub fn close(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.close(),
            WindowCtrlMode::Headless(c) => c.close(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.close(),
            WindowCtrlMode::Nested(c) => c.close(),
        }
    }

    pub(crate) fn view_task(&mut self, task: Box<dyn FnOnce(Option<&ViewWindow>) + Send>) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.view_task(task),
            WindowCtrlMode::Headless(c) => c.view_task(task),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.view_task(task),
            WindowCtrlMode::Nested(c) => c.view_task(task),
        }
    }
}

fn default_min_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(192), Dip::new(48)).to_px(scale_factor)
}

fn default_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor)
}

/// Respawned error is ok here, because we recreate the window/surface on respawn.
type Ignore = Result<(), ChannelError>;

pub(crate) struct NestedContentCtrl {
    content: ContentCtrl,
    pending_layout: Option<Arc<LayoutUpdates>>,
    pending_render: Option<[Arc<RenderUpdates>; 2]>,
    ctx: WindowCtx,
    host: Option<(WindowId, WidgetId)>,
    #[cfg(feature = "image")]
    pending_frame_capture: FrameCapture,
}

/// Implementer of an endpoint to an `WindowRoot` being used as an widget.
struct NestedCtrl {
    c: Arc<Mutex<NestedContentCtrl>>,
    actual_parent: Option<WindowId>,
    // actual_color_scheme and scale_factor binding.
    var_bindings: VarHandles,
}
impl NestedCtrl {
    pub fn new(c: Arc<Mutex<NestedContentCtrl>>) -> Self {
        Self {
            c,
            actual_parent: None,
            var_bindings: VarHandles::dummy(),
        }
    }

    fn update(&mut self, update_widgets: &WidgetUpdates) {
        let mut c = self.c.lock();
        c.content.update(update_widgets);

        let vars = &c.content.vars;

        if update_parent(&mut self.actual_parent, vars) || self.var_bindings.is_dummy() {
            let m_scale_factor = if let Some(p) = self.actual_parent.and_then(|p| WINDOWS.vars(p).ok()) {
                p.actual_monitor()
                    .get()
                    .and_then(|m| MONITORS.monitor(m))
                    .map(|m| m.scale_factor().get())
            } else {
                None
            };
            self.var_bindings = update_headless_vars(m_scale_factor, vars);
        }
    }

    fn info(&mut self, info_widgets: Arc<InfoUpdates>) -> Option<WidgetInfoTree> {
        self.c.lock().content.info(info_widgets)
    }

    fn pre_event(&mut self, update: &EventUpdate) {
        #[cfg(feature = "image")]
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            let mut c = self.c.lock();
            let c = &mut *c;
            if let Some((win, _)) = c.host
                && args.window_id == win
            {
                let image = match mem::take(&mut c.pending_frame_capture) {
                    FrameCapture::None => None,
                    FrameCapture::Full => Some(WINDOWS.frame_image(win, None).get()),
                    FrameCapture::Mask(m) => Some(WINDOWS.frame_image(win, Some(m)).get()),
                    _ => None,
                };
                let args = FrameImageReadyArgs::new(args.timestamp, args.propagation().clone(), win, args.frame_id, image);
                FRAME_IMAGE_READY_EVENT.notify(args);
            }
            return;
        }
        self.c.lock().content.pre_event(update)
    }

    fn ui_event(&mut self, update: &EventUpdate) {
        self.c.lock().content.ui_event(update)
    }

    fn layout(&self, layout_widgets: Arc<LayoutUpdates>) {
        if layout_widgets.delivery_list().enter_window(WINDOW.id()) {
            let mut c = self.c.lock();
            let c = &mut *c;
            if let Some((_, wgt_id)) = &c.host {
                c.pending_layout = Some(layout_widgets);
                UPDATES.layout(*wgt_id);
            }
        }
    }

    fn render(&self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        let id = WINDOW.id();
        if render_widgets.delivery_list().enter_window(id) || render_update_widgets.delivery_list().enter_window(id) {
            let mut c = self.c.lock();
            let c = &mut *c;
            if let Some((_, wgt_id)) = &c.host {
                c.pending_render = Some([render_widgets, render_update_widgets]);
                UPDATES.render(*wgt_id);
            }
        }
    }

    fn focus(&self) {
        self.bring_to_top();
        // many services track window focus with this event.
        let args = RawWindowFocusArgs::now(WINDOWS.focused_window_id(), Some(WINDOW.id()));
        RAW_WINDOW_FOCUS_EVENT.notify(args);
    }

    pub fn start_drag_drop(&mut self, data: Vec<DragDropData>, allowed_effects: DragDropEffect) -> Result<DragDropId, DragDropError> {
        if let Some((win_id, _)) = &self.c.lock().host {
            return WINDOWS_DRAG_DROP.start_drag_drop(*win_id, data, allowed_effects);
        }
        Err(DragDropError::CannotStart("nested window host unavailable".into()))
    }

    pub fn drag_dropped(&mut self, drop_id: DragDropId, applied: DragDropEffect) {
        let _ = (drop_id, applied);
    }

    fn bring_to_top(&self) {
        if let Some((win_id, _)) = &self.c.lock().host {
            let _ = WINDOWS.bring_to_top(*win_id);
        }
    }

    fn close(&mut self) {
        let mut c = self.c.lock();
        c.content.close();
        c.pending_layout = None;
        c.pending_render = None;
        if let Some((_, wgt_id)) = &c.host {
            // NestedWindowNode collapses on close
            UPDATES.layout(*wgt_id);
        }
    }

    fn view_task(&self, task: Box<dyn FnOnce(Option<&ViewWindow>)>) {
        task(None)
    }
}

/// UI node implementation that presents a [`WindowRoot`] as embedded content.
pub struct NestedWindowNode {
    c: Arc<Mutex<NestedContentCtrl>>,
}
impl NestedWindowNode {
    fn layout_impl(&mut self, is_measure: bool, measure_layout: impl FnOnce(&mut UiNode) -> PxSize) -> PxSize {
        let mut c = self.c.lock();
        let c = &mut *c;

        if !c.content.vars.0.is_open.get() {
            return PxSize::zero();
        }

        let auto_size = c.content.vars.auto_size().get();
        let constraints = LAYOUT.constraints();

        let metrics = LayoutMetrics::new(LAYOUT.scale_factor(), PxSize::splat(Px::MAX), LAYOUT.root_font_size())
            .with_constraints(constraints)
            .with_screen_density(LAYOUT.screen_density())
            .with_direction(DIRECTION_VAR.get());

        // only the same app_local!, APP.id
        LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only()).with_context(|| {
            WINDOW.with_context(&mut c.ctx, || {
                WIDGET.with_context(&mut c.content.root_ctx, WidgetUpdateMode::Bubble, || {
                    LAYOUT.with_root_context(c.content.layout_pass, metrics, || {
                        let mut root_cons = LAYOUT.constraints();

                        // equivalent of with_fill_metrics used by `max_size` property
                        let dft = root_cons.fill_size();
                        let (min_size, max_size, pref_size) =
                            LAYOUT.with_constraints(root_cons.with_fill_vector(root_cons.is_bounded()), || {
                                let max = c.content.vars.max_size().layout_dft(dft);
                                (c.content.vars.min_size().layout(), max, c.content.vars.size().layout_dft(max))
                            });

                        let min_size = min_size.max(root_cons.min_size());
                        let max_size = max_size.min(root_cons.max_size_or(PxSize::splat(Px::MAX)));
                        let pref_size = pref_size.clamp(min_size, max_size);

                        if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                            root_cons.x = PxConstraints::new_range(min_size.width, max_size.width);
                        } else {
                            root_cons.x = PxConstraints::new_exact(pref_size.width);
                        }
                        if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                            root_cons.y = PxConstraints::new_range(min_size.height, max_size.height);
                        } else {
                            root_cons.y = PxConstraints::new_exact(pref_size.height);
                        }

                        if auto_size.is_empty() && is_measure {
                            pref_size
                        } else {
                            LAYOUT.with_constraints(root_cons, || measure_layout(&mut c.content.root))
                        }
                    })
                })
            })
        })
    }
}
impl UiNodeImpl for NestedWindowNode {
    fn children_len(&self) -> usize {
        1
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index == 0 {
            visitor(&mut self.c.lock().content.root)
        }
    }

    fn init(&mut self) {
        let mut c = self.c.lock();
        let parent_id = WINDOW.id();
        c.content.vars.parent().set(parent_id);
        let nest_parent = WIDGET.id();
        c.content.vars.0.nest_parent.set(nest_parent);
        c.host = Some((parent_id, nest_parent));
        // init handled by // NestedCtrl::update
    }

    fn deinit(&mut self) {
        // this can be a parent reinit or node move, if not inited after 100ms close the window.
        let mut c = self.c.lock();
        c.host = None;
        let c = &self.c;
        TIMERS
            .on_deadline(
                100.ms(),
                hn_once!(c, |_| {
                    let c = c.lock();
                    if c.host.is_none() {
                        let _ = WINDOWS.close(c.ctx.id());
                    }
                }),
            )
            .perm();

        // deinit handled by NestedCtrl::close
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        info.set_meta(*NESTED_WINDOW_INFO_ID, self.c.lock().ctx.id());
    }

    fn event(&mut self, _: &EventUpdate) {
        // event handled by NestedCtrl::ui_event
    }

    fn update(&mut self, _: &WidgetUpdates) {
        // update handled by NestedCtrl::update
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.layout_impl(true, |r| wm.with_widget(|wm| r.measure(wm)))
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let pending = self.c.lock().pending_layout.take();
        let size = self.layout_impl(false, |r| {
            if let Some(p) = pending {
                wl.with_layout_updates(p, |wl| wl.with_widget(|wl| r.layout(wl)))
            } else {
                wl.with_widget(|wl| r.layout(wl))
            }
        });
        let c = self.c.lock();
        let factor = LAYOUT.scale_factor();
        c.content.vars.0.scale_factor.set(factor);
        c.content.vars.0.actual_size.set(size.to_dip(factor));
        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        let mut c = self.c.lock();
        let c = &mut *c;

        if !c.content.vars.0.is_open.get() {
            return;
        }

        let [render_widgets, render_update_widgets] = c.pending_render.take().unwrap_or_default();
        // only the same app_local!, APP.id
        LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only()).with_context(|| {
            WINDOW.with_context(&mut c.ctx, || {
                let root_id = c.content.root_ctx.id();
                let root_bounds = c.content.root_ctx.bounds();
                WIDGET.with_context(&mut c.content.root_ctx, WidgetUpdateMode::Bubble, || {
                    frame.with_nested_window(
                        render_widgets,
                        render_update_widgets,
                        root_id,
                        &root_bounds,
                        &WINDOW.info(),
                        FontAntiAliasing::Default,
                        |frame| {
                            c.content.root.render(frame);
                        },
                    );
                });
                #[cfg(feature = "image")]
                {
                    c.pending_frame_capture = c.content.take_frame_capture();
                }
            })
        })
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        let mut c = self.c.lock();
        let c = &mut *c;

        if !c.content.vars.0.is_open.get() {
            return;
        }

        let [_, render_update_widgets] = c.pending_render.take().unwrap_or_default();
        // only the same app_local!, APP.id
        LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only()).with_context(|| {
            WINDOW.with_context(&mut c.ctx, || {
                WIDGET.with_context(&mut c.content.root_ctx, WidgetUpdateMode::Bubble, || {
                    update.with_nested_window(render_update_widgets, WIDGET.id(), WIDGET.bounds(), |update| {
                        c.content.root.render_update(update);
                    })
                })
            })
        })
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        if self.c.lock().content.root.as_widget().is_some() {
            Some(self)
        } else {
            None
        }
    }
}
impl WidgetUiNodeImpl for NestedWindowNode {
    fn with_context(&mut self, update_mode: WidgetUpdateMode, visitor: &mut dyn FnMut()) {
        let mut lock = self.c.lock();
        if let Some(mut w) = lock.content.root.as_widget() {
            w.with_context(update_mode, visitor)
        }
    }
}

static_id! {
    static ref NESTED_WINDOW_INFO_ID: StateId<WindowId>;
}

/// Extension methods for widget info about a node that hosts a nested window.
pub trait NestedWindowWidgetInfoExt {
    /// Gets the hosted window ID if the widget hosts a nested window.
    fn nested_window(&self) -> Option<WindowId>;

    /// Gets the hosted window info tree if the widget hosts a nested window that is open.
    fn nested_window_tree(&self) -> Option<WidgetInfoTree> {
        WINDOWS.widget_tree(self.nested_window()?).ok()
    }
}

impl NestedWindowWidgetInfoExt for WidgetInfo {
    fn nested_window(&self) -> Option<WindowId> {
        self.meta().get_clone(*NESTED_WINDOW_INFO_ID)
    }
}

#[allow(clippy::large_enum_variant)] // Normal is the largest, but also most common
enum OpenNestedHandlerArgsValue {
    Normal {
        ctx: WindowCtx,
        vars: WindowVars,
        commands: WindowCommands,
        window: WindowRoot,
    },
    Nested {
        ctx: WindowCtx,
        node: Arc<Mutex<NestedContentCtrl>>,
    },
    TempNone,
}

/// Arguments for the [`WINDOWS.register_open_nested_handler`] handler.
///
/// [`WINDOWS.register_open_nested_handler`]: WINDOWS::register_open_nested_handler
pub struct OpenNestedHandlerArgs {
    c: OpenNestedHandlerArgsValue,
}
impl OpenNestedHandlerArgs {
    pub(crate) fn new(ctx: WindowCtx, vars: WindowVars, commands: WindowCommands, window: WindowRoot) -> Self {
        Self {
            c: OpenNestedHandlerArgsValue::Normal {
                ctx,
                vars,
                commands,
                window,
            },
        }
    }

    /// New window context.
    pub fn ctx(&self) -> &WindowCtx {
        match &self.c {
            OpenNestedHandlerArgsValue::Normal { ctx, .. } | OpenNestedHandlerArgsValue::Nested { ctx, .. } => ctx,
            OpenNestedHandlerArgsValue::TempNone => unreachable!(),
        }
    }

    /// Window vars.
    pub fn vars(&mut self) -> WindowVars {
        let ctx = match &mut self.c {
            OpenNestedHandlerArgsValue::Normal { ctx, .. } | OpenNestedHandlerArgsValue::Nested { ctx, .. } => ctx,
            OpenNestedHandlerArgsValue::TempNone => unreachable!(),
        };
        WINDOW.with_context(ctx, || WINDOW.vars())
    }

    /// Instantiate a node that layouts and renders the window content.
    ///
    /// Calling this will stop the normal window chrome from opening, the caller is responsible for inserting the node into the
    /// main window layout.
    ///
    /// Note that the window will notify *open* like normal, but it will only be visible on this node.
    pub fn nest(&mut self) -> NestedWindowNode {
        match mem::replace(&mut self.c, OpenNestedHandlerArgsValue::TempNone) {
            OpenNestedHandlerArgsValue::Normal {
                mut ctx,
                vars,
                commands,
                window,
            } => {
                let node = NestedWindowNode {
                    c: Arc::new(Mutex::new(NestedContentCtrl {
                        content: ContentCtrl::new(vars, commands, window),
                        pending_layout: None,
                        pending_render: None,
                        #[cfg(feature = "image")]
                        pending_frame_capture: FrameCapture::None,
                        ctx: ctx.share(),
                        host: None,
                    })),
                };
                self.c = OpenNestedHandlerArgsValue::Nested { ctx, node: node.c.clone() };
                node
            }
            _ => panic!("already nesting"),
        }
    }

    pub(crate) fn has_nested(&self) -> bool {
        matches!(&self.c, OpenNestedHandlerArgsValue::Nested { .. })
    }

    pub(crate) fn take_normal(
        self,
    ) -> Result<(WindowCtx, WindowVars, WindowCommands, WindowRoot), (WindowCtx, Arc<Mutex<NestedContentCtrl>>)> {
        match self.c {
            OpenNestedHandlerArgsValue::Normal {
                ctx,
                vars,
                commands,
                window,
            } => Ok((ctx, vars, commands, window)),
            OpenNestedHandlerArgsValue::Nested { ctx, node } => Err((ctx, node)),
            OpenNestedHandlerArgsValue::TempNone => unreachable!(),
        }
    }
}
