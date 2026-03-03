use zng_app::{
    EXIT_REQUESTED_EVENT,
    access::{ACCESS_DEINITED_EVENT, ACCESS_INITED_EVENT},
    hn_once,
    update::UPDATES,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewWindow,
        raw_events::{
            RAW_COLORS_CONFIG_CHANGED_EVENT, RAW_IME_EVENT, RAW_WINDOW_CHANGED_EVENT, RAW_WINDOW_CLOSE_EVENT,
            RAW_WINDOW_CLOSE_REQUESTED_EVENT, RAW_WINDOW_FOCUS_EVENT, RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT,
        },
    },
    widget::{
        WidgetId,
        info::{WIDGET_TREE_CHANGED_EVENT, access::AccessEnabled},
    },
    window::WindowId,
};
use zng_color::LightDark;
use zng_layout::{
    context::LayoutPassId,
    unit::{
        Dip, DipRect, DipSize, DipToPx, Factor, FactorUnits as _, Layout2d, Length, Px, PxDensity, PxPoint, PxRect, PxSize, PxToDip as _,
    },
};
use zng_var::VarHandle;
use zng_view_api::window::{WindowCapability, WindowState};
use zng_wgt::prelude::{DIRECTION_VAR, InteractionPath, LAYOUT, LayoutMetrics};

use crate::{
    AutoSize, CursorSource, IME_EVENT, ImeArgs, MONITORS, SetFromViewTag, WINDOW_CHANGED_EVENT, WINDOW_CLOSE_REQUESTED_EVENT,
    WINDOW_FOCUS_CHANGED_EVENT, WINDOWS, WINDOWS_SV, WidgetInfoImeArea, WindowChangedArgs, WindowCloseRequestedArgs,
    WindowFocusChangedArgs, WindowInstance, WindowInstanceState, WindowNode, WindowVars, cmd::WindowCommands,
};

/// Hooks always active for the lifetime of the app.
pub(crate) fn hook_events() {
    VIEW_PROCESS_INITED_EVENT
        .hook(move |args| {
            // layout all windows in `Loaded` state, view open requests happen during layout

            let mut s = WINDOWS_SV.write();
            for (id, w) in s.windows.iter_mut() {
                if let Some(vars) = &w.vars
                    && let WindowInstanceState::Loaded { has_view } = vars.0.instance_state.get()
                {
                    UPDATES.layout_window(*id);

                    // cleanup old view handles
                    if has_view {
                        debug_assert!(args.is_respawn);

                        let r = w.root.as_mut().unwrap();
                        r.renderer = None;
                        r.view_headless = None;
                        r.view_window = None;

                        vars.0.instance_state.set(WindowInstanceState::Loaded { has_view: false });
                    }
                }
            }

            true
        })
        .perm();

    RAW_WINDOW_CLOSE_REQUESTED_EVENT
        .hook(move |args| {
            // WINDOWS.close will make a `WINDOW_CLOSE_REQUESTED_EVENT` for the window and any children window.

            WINDOWS.close(args.window_id);
            true
        })
        .perm();

    RAW_WINDOW_CLOSE_EVENT
        .hook(move |args| {
            let mut s = WINDOWS_SV.write();
            if let Some(w) = s.windows.get_mut(&args.window_id) {
                // view-process closed a window without crashing, treat like a respawn anyway

                tracing::error!(
                    "window {} closed in view-process without request, will try to reopen",
                    args.window_id
                );
                if let Some(r) = &mut w.root {
                    if r.renderer.is_some() {
                        w.vars
                            .as_ref()
                            .unwrap()
                            .0
                            .instance_state
                            .set(WindowInstanceState::Loaded { has_view: false });
                        r.renderer = None;
                        r.view_window = None;
                        r.view_headless = None;
                    }
                    r.view_opening = VarHandle::dummy();
                }
            }

            true
        })
        .perm();

    RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT
        .hook(|args| {
            // fatal error if view-process fails to open, there is no way to recover from this,
            // its not a view-process crash as that causes a respawn, its an invalid request, maybe
            // the implementation only supports one window or something like that

            if WINDOWS_SV.read().windows.contains_key(&args.window_id) {
                panic!("view-process failed to open {}, {}", args.window_id, args.error);
            }

            true
        })
        .perm();
    // RAW_WINDOW_OPEN_EVENT and RAW_HEADLESS_OPEN_EVENT are implemented in the window layout

    RAW_WINDOW_FOCUS_EVENT
        .hook(|args| {
            let mut prev = None;
            let mut new = None;
            let s = WINDOWS_SV.read();
            for (id, w) in s.windows.iter() {
                if let Some(v) = &w.vars {
                    if v.0.focused.get() {
                        prev = Some(*id);
                    }
                    let is_focused = args.new_focus == Some(*id);
                    if is_focused {
                        new = Some(*id);

                        // bring children to front
                        v.0.children.with(|c| {
                            for id in c {
                                if let Some(w) = s.windows.get(id)
                                    && let Some(r) = &w.root
                                    && let Some(v) = &r.view_window
                                {
                                    let _ = v.bring_to_top();
                                }
                            }
                        });
                    }
                    v.set_from_view(|v| &v.0.focused, is_focused);
                }
            }
            let closed = prev.is_none() && args.prev_focus.is_some();
            if closed {
                prev = args.prev_focus;
            }
            if prev != new {
                WINDOW_FOCUS_CHANGED_EVENT.notify(WindowFocusChangedArgs::new(
                    args.timestamp,
                    args.propagation.clone(),
                    prev,
                    new,
                    closed,
                ));
            }
            true
        })
        .perm();

    RAW_WINDOW_CHANGED_EVENT
        .hook(|args| {
            let s = WINDOWS_SV.read();
            if let Some(w) = s.windows.get(&args.window_id)
                && let Some(vars) = &w.vars
            {
                let mut any = false;
                let mut state_change = None;
                if let Some(s) = &args.state {
                    let prev_state = vars.0.state.get();
                    if prev_state != s.state {
                        any = true;
                        state_change = Some((prev_state, s.state));
                    }

                    vars.set_from_view(|v| &v.0.state, s.state);
                    vars.set_from_view(|v| &v.0.global_position, s.global_position);
                    vars.set_from_view(|v| &v.0.restore_rect, s.restore_rect);
                    vars.set_from_view(|v| &v.0.restore_state, s.restore_state);
                    vars.set_from_view(|v| &v.0.chrome, s.chrome_visible);
                }
                if let Some(id) = &args.monitor {
                    tracing::trace!("window {:?} moved to {:?}", args.window_id, id);
                    vars.set_from_view(|v| &v.0.actual_monitor, Some(*id));
                }
                if let Some(size) = &args.size {
                    any = true;
                    vars.set_from_view(|v| &v.0.actual_size, *size);
                    if let zng_view_api::window::EventCause::System = args.cause {
                        vars.set_from_view(|v| &v.0.auto_size, AutoSize::DISABLED);
                    }
                }
                if let Some((g_pos, pos)) = &args.position {
                    any = true;
                    vars.set_from_view(|v| &v.0.global_position, *g_pos);
                    vars.set_from_view(|v| &v.0.actual_position, *pos);
                }
                if let Some(p) = &args.safe_padding {
                    vars.set_from_view(|v| &v.0.safe_padding, *p);
                }
                if let Some(f) = args.scale_factor {
                    vars.set_from_view(|v| &v.0.scale_factor, f);
                }
                if let Some(f) = args.refresh_rate {
                    vars.set_from_view(|v| &v.0.refresh_rate, f);
                }

                if let Some(id) = &args.frame_wait_id {
                    // signal will be send with next frame
                    drop(s);
                    WINDOWS_SV
                        .write()
                        .windows
                        .get_mut(&args.window_id)
                        .unwrap()
                        .root
                        .as_mut()
                        .unwrap()
                        .frame_wait_id = Some(*id);

                    // request in case the size did not actually change (not causing frame update)
                    UPDATES.render_window(args.window_id);
                }

                if any {
                    WINDOW_CHANGED_EVENT.notify(WindowChangedArgs::now(
                        args.window_id,
                        state_change,
                        args.position,
                        args.size,
                        args.cause,
                    ));
                }
            }
            true
        })
        .perm();

    ACCESS_INITED_EVENT
        .hook(|args| {
            let s = WINDOWS_SV.read();
            if let Some(w) = s.windows.get(&args.window_id)
                && let Some(vars) = &w.vars
            {
                vars.0.access_enabled.set(AccessEnabled::VIEW);
            }
            true
        })
        .perm();
    ACCESS_DEINITED_EVENT
        .hook(|args| {
            let s = WINDOWS_SV.read();
            if let Some(w) = s.windows.get(&args.window_id)
                && let Some(vars) = &w.vars
            {
                vars.0.access_enabled.modify(|a| {
                    if a.is_enabled() {
                        **a = AccessEnabled::APP;
                    }
                });
            }
            true
        })
        .perm();

    RAW_COLORS_CONFIG_CHANGED_EVENT
        .hook(|args| {
            let s = WINDOWS_SV.read();
            for w in s.windows.values() {
                if let Some(vars) = &w.vars {
                    // system color is used when there is no window pref and no parent window
                    if vars.0.color_scheme.get().is_none() && vars.0.parent.get().is_none() {
                        vars.0.actual_color_scheme.set(args.config.scheme);
                    }

                    // same for accent color
                    if vars.0.accent_color.get().is_none() && vars.0.parent.get().is_none() {
                        vars.0.actual_accent_color.set(args.config.accent);
                    }
                }
            }
            true
        })
        .perm();

    // on exit request, if there are windows open cancel, request close and if all close request exit again
    EXIT_REQUESTED_EVENT
        .hook(|args| {
            if !WINDOWS_SV.read().windows.is_empty() {
                args.propagation.stop();
                WINDOWS.close_all();
                WINDOW_CLOSE_REQUESTED_EVENT
                    .on_event(
                        true,
                        hn_once!(|args: &WindowCloseRequestedArgs| {
                            if !args.propagation.is_stopped() {
                                zng_app::APP.exit();
                            }
                        }),
                    )
                    .perm();
            }
            true
        })
        .perm();

    // propagate IME event if the window has a focused IME area widget
    RAW_IME_EVENT
        .hook(|args| {
            let s = WINDOWS_SV.read();
            if let Some(focus) = s.focused.get()
                && focus.window_id() == args.window_id
                && focus.interactivity().is_enabled()
                && let Some(w) = s.windows.get(&args.window_id)
                && let Some(info) = &w.info
                && let Some(info) = info.get(focus.widget_id())
                && info.ime_area().is_some()
            {
                let mut preview_caret = None;
                let txt;
                match &args.ime {
                    zng_view_api::Ime::Preview(t, caret) => {
                        txt = t.clone();
                        preview_caret = Some(*caret);
                    }
                    zng_view_api::Ime::Commit(t) => txt = t.clone(),
                }

                IME_EVENT.notify(ImeArgs::new(args.timestamp, args.propagation.clone(), focus, txt, preview_caret));
            }
            true
        })
        .perm();

    #[cfg(feature = "image")]
    zng_app::view_process::raw_events::RAW_FRAME_RENDERED_EVENT
        .hook(|args| {
            if let Some(img) = &args.frame_image
                && let Some(img) = img.upgrade()
                && let Some(mode) = WINDOWS.mode(args.window_id)
                && mode.has_renderer()
            {
                let img = zng_ext_image::IMAGES.register(None, (**img).clone());
                crate::FRAME_IMAGE_READY_EVENT.notify(crate::FrameImageReadyArgs::new(
                    args.timestamp,
                    args.propagation.clone(),
                    args.window_id,
                    args.frame_id,
                    zng_var::WeakVarEq(img.downgrade()),
                ));
                UPDATES.once_next_update("", move || {
                    let _hold = &img;
                });
            }
            true
        })
        .perm();
}

pub(crate) fn hook_window_vars_cmds(id: WindowId, vars: &WindowVars) {
    WindowCommands::init(id, vars);

    // update view state
    vars.0.state.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.global_position.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.restore_rect.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.restore_state.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.actual_min_size.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.actual_max_size.as_any().hook(move |s| on_state_changed(id, s)).perm();
    vars.0.chrome.as_any().hook(move |s| on_state_changed(id, s)).perm();

    vars.0
        .scale_factor
        .hook(move |_| {
            UPDATES.layout_window(id);
            true
        })
        .perm();
    vars.0
        .refresh_rate
        .hook(move |_| {
            WINDOWS_SV.read().set_frame_duration();
            true
        })
        .perm();

    // move monitors, only active after layout selected first actual_monitor
    vars.0
        .monitor
        .hook(move |args| {
            let sv = WINDOWS_SV.read();
            if let Some(w) = sv.windows.get(&id)
                && let Some(vars) = &w.vars
                && matches!(vars.0.instance_state.get(), WindowInstanceState::Loaded { has_view: true })
                && let Some(new_monitor) = args.value().select(id)
                && Some(new_monitor.id()) != vars.0.actual_monitor.get()
            {
                tracing::trace!("moving {id:?} from {:?} to {:?}", vars.0.actual_monitor.get(), new_monitor.id());

                // * **Maximized**: The window is maximized in the new monitor.
                // * **Fullscreen**: The window is fullscreen in the new monitor.
                // * **Normal**: The window is centered in the new monitor, keeping the same size.
                // * **Minimized/Hidden**: The window remains hidden, the restore position and size are defined like **Normal**.

                // center restore info on new monitor
                let screen_rect = new_monitor.px_rect();
                let new_scale_factor = new_monitor.scale_factor().get();
                let new_window_size = vars
                    .0
                    .restore_rect
                    .get()
                    .size
                    .to_px(new_scale_factor)
                    .min(screen_rect.size - PxSize::splat(Px(20)));

                let pos_in_new_monitor = PxPoint::new(
                    (screen_rect.size.width - new_window_size.width) / Px(2),
                    (screen_rect.size.height - new_window_size.height) / Px(2),
                );
                vars.0.global_position.set(screen_rect.origin + pos_in_new_monitor.to_vector());
                vars.0
                    .restore_rect
                    .set(PxRect::new(pos_in_new_monitor, new_window_size).to_dip(new_scale_factor));

                let state = vars.0.state.get();
                if matches!(state, WindowState::Maximized | WindowState::Fullscreen | WindowState::Exclusive) {
                    // restore to actually move
                    vars.0.state.set(WindowState::Normal);

                    // once moved return to maximized/fullscreen
                    let new_monitor_id = new_monitor.id();
                    RAW_WINDOW_CHANGED_EVENT
                        .hook(move |args| {
                            if args.window_id == id
                                && let Some(m_id) = args.monitor
                            {
                                if m_id == new_monitor_id
                                    && let Some(vars) = WINDOWS.vars(id)
                                {
                                    vars.0.state.modify(move |s| {
                                        if matches!(&**s, WindowState::Normal) {
                                            **s = state;
                                        }
                                    });
                                }
                                false
                            } else {
                                true
                            }
                        })
                        .perm();
                }
            }
            true
        })
        .perm();

    #[cfg(feature = "image")]
    {
        // load/cache icon image and bind it to `actual_icon`
        let actual_icon = vars.0.actual_icon.clone();
        let mut _bind_handle = VarHandle::dummy();
        vars.0
            .icon
            .hook(move |args| {
                use crate::WindowIcon;

                match args.value() {
                    WindowIcon::Default => {
                        actual_icon.set(None);
                        _bind_handle = VarHandle::dummy();
                    }
                    WindowIcon::Image(img) => {
                        _bind_handle = load_bind_ico(id, img.clone(), &actual_icon, WindowCapability::SET_ICON, "window-icon", |i| {
                            Some(i.clone())
                        });
                        if _bind_handle.is_dummy() {
                            // VIEW_PROCESS does not support SET_ICON
                            actual_icon.set(None);
                            return false;
                        }
                    }
                }
                true
            })
            .perm();
        vars.0
            .actual_icon
            .hook(move |args| {
                with_view(id, |_, _, v| {
                    let _ = v.set_icon(args.value().as_ref().map(|i| i.view_handle()));
                });
                true
            })
            .perm();
    }

    // set cursor icon and manage icon image loading
    #[cfg(feature = "image")]
    let mut _bind_handle = VarHandle::dummy();
    #[cfg(feature = "image")]
    let actual_cursor_img = vars.0.actual_cursor_img.clone();
    vars.0
        .cursor
        .hook(move |args| {
            match args.value() {
                CursorSource::Icon(ico) => {
                    // only built-in cursor, clean custom image
                    with_view(id, |_, _, v| {
                        let _ = v.set_cursor(Some(*ico));
                    });
                    #[cfg(feature = "image")]
                    {
                        _bind_handle = VarHandle::dummy();
                        actual_cursor_img.set(None);
                    }
                }
                #[cfg(feature = "image")]
                CursorSource::Img(img) => {
                    // load image cursor, set fallback immediately
                    with_view(id, |_, _, v| {
                        let _ = v.set_cursor(Some(img.fallback));
                    });
                    {
                        let hotspot = img.hotspot.clone();
                        _bind_handle = load_bind_ico(
                            id,
                            img.source.clone(),
                            &actual_cursor_img,
                            WindowCapability::SET_CURSOR_IMAGE,
                            "cursor-img",
                            move |i| {
                                // layout hotspot relative to the image size
                                use zng_wgt::prelude::*;
                                let metrics = LayoutMetrics::new(1.fct(), i.size(), i.size().width);
                                let hotspot =
                                    LAYOUT.with_root_context(zng_layout::context::LayoutPassId::new(), metrics, || hotspot.layout());
                                Some((i.clone(), hotspot))
                            },
                        );
                        if _bind_handle.is_dummy() {
                            // VIEW_PROCESS does not support SET_CURSOR_IMAGE
                            actual_cursor_img.set(None);
                            return false;
                        }
                    }
                }
                CursorSource::Hidden => {
                    // remove built-in and image cursor
                    with_view(id, |_, _, v| {
                        let _ = v.set_cursor(None);
                        #[cfg(feature = "image")]
                        {
                            _bind_handle = VarHandle::dummy();
                            actual_cursor_img.set(None);
                        }
                    });
                }
            }
            true
        })
        .perm();
    #[cfg(feature = "image")]
    vars.0
        .actual_cursor_img
        .hook(move |args| {
            with_view(id, |_, _, v| {
                let _ = match args.value() {
                    Some((img, hotspot)) => v.set_cursor_image(Some(img.view_handle()), *hotspot),
                    None => v.set_cursor_image(None, zng_layout::unit::PxPoint::zero()),
                };
            });
            true
        })
        .perm();

    // set title
    vars.0
        .title
        .hook(move |args| {
            with_view(id, |_, _, v| {
                let _ = v.set_title(args.value().clone());
            });
            true
        })
        .perm();

    // set focus indicator
    vars.0
        .focus_indicator
        .hook(move |args| {
            with_view(id, |_, _, v| {
                let _ = v.set_focus_indicator(*args.value());
            });
            true
        })
        .perm();

    // move window
    let mut check_view_caps = true;
    vars.0
        .position
        .hook(move |args| {
            // some systems (Wayland) do not allow moving the window, worth checking once
            if check_view_caps && VIEW_PROCESS.is_connected() {
                check_view_caps = false;
                if !VIEW_PROCESS.info().window.contains(WindowCapability::SET_POSITION) {
                    tracing::warn!("view-process cannot SET_POSITION in the current system");
                    return false;
                }
            }

            with_monitor_layout(id, |_, vars, monitor_rect, scale_factor, _| {
                let pos = args.value().layout();

                // this updates the view window, if there is one
                vars.0.global_position.set(monitor_rect.origin + pos.to_vector());
                let pos = pos.to_dip(scale_factor);
                vars.0.restore_rect.modify(move |a| {
                    if a.origin != pos {
                        a.origin = pos;
                    }
                });
            });

            true
        })
        .perm();

    // resize window
    let mut check_view_caps = true;
    vars.0
        .size
        .hook(move |args| {
            // some systems (Android) do not allow resizing the window, worth checking once
            if check_view_caps && VIEW_PROCESS.is_connected() {
                check_view_caps = false;
                if !VIEW_PROCESS.info().window.contains(WindowCapability::SET_SIZE) {
                    tracing::warn!("view-process cannot SET_SIZE in the current system");
                    return false;
                }
            }

            with_monitor_layout(id, |_, vars, _, scale_factor, _| {
                let size = args
                    .value()
                    .layout_dft(DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor))
                    .to_dip(scale_factor);
                vars.0.restore_rect.modify(move |a| {
                    if a.size != size {
                        a.size = size;
                    }
                });
            });

            true
        })
        .perm();

    // update exclusive fullscreen video mode
    vars.0
        .video_mode
        .hook(move |args| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::EXCLUSIVE) {
                // can't take exclusive video output so don't need to update mode
                return false;
            }

            with_view(id, |_, _, v| {
                let _ = v.set_video_mode(*args.value());
            });

            true
        })
        .perm();

    // update min size
    vars.0
        .min_size
        .hook(move |a| {
            with_monitor_layout(id, |_, vars, _, scale_factor, _| {
                let min_size = a.value().layout().to_dip(scale_factor);
                // view state is updated by a hook to this
                vars.0.actual_min_size.set(min_size);
                vars.0.restore_rect.modify(move |a| {
                    let new_size = a.size.max(min_size);
                    if new_size != a.size {
                        a.size = new_size;
                    }
                })
            });
            true
        })
        .perm();

    // update max size
    vars.0
        .max_size
        .hook(move |a| {
            with_monitor_layout(id, |_, vars, _, scale_factor, _| {
                let max_size = a.value().layout().to_dip(scale_factor);
                // view state is updated by a hook to this
                vars.0.actual_max_size.set(max_size);
                vars.0.restore_rect.modify(move |a| {
                    let new_size = a.size.min(max_size);
                    if new_size != a.size {
                        a.size = new_size;
                    }
                })
            });
            true
        })
        .perm();

    // update size
    vars.0
        .size
        .hook(move |a| {
            with_monitor_layout(id, |_, vars, _, scale_factor, _| {
                let size = a.value().layout_dft(DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor));
                let min = vars.0.actual_min_size.get();
                let max = vars.0.actual_max_size.get();
                let size = size.to_dip(scale_factor).max(min).min(max);
                // view state is updated by a hook to this
                vars.0.restore_rect.modify(move |a| {
                    if a.size != size {
                        a.size = size;
                    }
                });
                vars.0.auto_size.set(AutoSize::DISABLED);
            });
            true
        })
        .perm();
    vars.0
        .auto_size
        .hook(move |a| {
            if *a.value() != AutoSize::DISABLED {
                UPDATES.layout_window(id);
            }
            true
        })
        .perm();

    // update layout on external resize
    vars.0
        .actual_size
        .hook(move |a| {
            if a.contains_tag(&SetFromViewTag) {
                UPDATES.layout_window(id);
            }
            true
        })
        .perm();

    // update root font size
    vars.0
        .font_size
        .hook(move |_| {
            UPDATES.layout_window(id);
            true
        })
        .perm();

    // set enabled window buttons
    vars.0
        .enabled_buttons
        .hook(move |a| {
            if VIEW_PROCESS.is_connected()
                && !VIEW_PROCESS.info().window.intersects(
                    WindowCapability::DISABLE_CLOSE_BUTTON
                        | WindowCapability::DISABLE_MINIMIZE_BUTTON
                        | WindowCapability::DISABLE_MAXIMIZE_BUTTON,
                )
            {
                tracing::warn!("view-process cannot affect window chrome buttons in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_enabled_buttons(*a.value());
            });
            true
        })
        .perm();

    // enable/disable resizable
    vars.0
        .resizable
        .hook(move |a| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::SET_RESIZABLE) {
                tracing::warn!("view-process cannot SET_RESIZABLE in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_resizable(*a.value());
            });
            true
        })
        .perm();

    // enable/disable movable
    vars.0
        .movable
        .hook(move |a| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::SET_MOVABLE) {
                tracing::warn!("view-process cannot SET_MOVABLE in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_movable(*a.value());
            });
            true
        })
        .perm();

    // enable/disable always_on_top
    vars.0
        .always_on_top
        .hook(move |a| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::SET_ALWAYS_ON_TOP) {
                tracing::warn!("view-process cannot SET_ALWAYS_ON_TOP in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_always_on_top(*a.value());
            });
            true
        })
        .perm();

    // show/hide window
    vars.0
        .visible
        .hook(move |a| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::SET_VISIBLE) {
                tracing::warn!("view-process cannot SET_VISIBLE in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_visible(*a.value());
            });
            true
        })
        .perm();

    // show/hide window icon in system taskbar
    vars.0
        .taskbar_visible
        .hook(move |a| {
            if VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(WindowCapability::SET_TASKBAR_VISIBLE) {
                tracing::warn!("view-process cannot SET_TASKBAR_VISIBLE in the current system");
                return false;
            }
            with_view(id, |_, _, v| {
                let _ = v.set_taskbar_visible(*a.value());
            });
            true
        })
        .perm();

    // set system shutdown warning message
    vars.0
        .system_shutdown_warn
        .hook(move |a| {
            with_view(id, |_, _, v| {
                let _ = v.set_system_shutdown_warn(a.value().clone());
            });
            true
        })
        .perm();

    // apply local color_scheme preference
    vars.0
        .color_scheme
        .hook(move |a| {
            let vars = WINDOWS.vars(id).unwrap();
            match *a.value() {
                Some(c) => vars.0.actual_color_scheme.set(c),
                None => {
                    // reset to fallback value, other hooks will keep updating (when color_scheme is None)
                    let c = if let Some(p) = vars.0.parent.get()
                        && let Some(parent_vars) = WINDOWS.vars(p)
                    {
                        parent_vars.0.actual_color_scheme.get()
                    } else {
                        RAW_COLORS_CONFIG_CHANGED_EVENT
                            .with(|e| e.latest().map(|a| a.config.scheme))
                            .unwrap_or_default()
                    };
                    vars.0.actual_color_scheme.set(c);
                }
            }
            true
        })
        .perm();

    // apply local accent_color preference
    vars.0
        .accent_color
        .hook(move |a| {
            let vars = WINDOWS.vars(id).unwrap();
            match *a.value() {
                Some(c) => vars.0.actual_accent_color.set(c),
                None => {
                    // reset to fallback value
                    let c = if let Some(p) = vars.0.parent.get()
                        && let Some(parent_vars) = WINDOWS.vars(p)
                    {
                        Some(parent_vars.0.actual_accent_color.get())
                    } else {
                        RAW_COLORS_CONFIG_CHANGED_EVENT
                            .with(|e| e.latest().map(|a| a.config.accent))
                            .map(LightDark::from)
                    };
                    if let Some(c) = c {
                        vars.0.actual_accent_color.set(c);
                    }
                }
            }
            true
        })
        .perm();

    // insert/remove self in parent children set
    let mut prev_parent = vars.0.parent.get();
    vars.0
        .parent
        .hook(move |a| {
            let s = WINDOWS_SV.read();
            let vars = s.windows.get(&id).unwrap().vars.as_ref().unwrap();

            if let Some(prev_p_id) = prev_parent
                && let Some(prev_p_vars) = s.windows.get(&prev_p_id).and_then(|w| w.vars.as_ref())
            {
                prev_p_vars.0.children.modify(move |a| {
                    a.remove(&id);
                });
            }
            if let Some(new_p_id) = *a.value()
                && let Some(new_p_vars) = s.windows.get(&new_p_id).and_then(|w| w.vars.as_ref())
            {
                new_p_vars.0.children.modify(move |a| {
                    a.insert(id);
                });

                if vars.0.color_scheme.get().is_none() {
                    vars.0.actual_color_scheme.set(new_p_vars.0.actual_color_scheme.get());
                }
                if vars.0.accent_color.get().is_none() {
                    vars.0.actual_accent_color.set(new_p_vars.0.actual_accent_color.get());
                }
            } else {
                let dft = RAW_COLORS_CONFIG_CHANGED_EVENT
                    .with(|e| e.latest().map(|l| l.config))
                    .unwrap_or_default();
                if vars.0.color_scheme.get().is_none() {
                    vars.0.actual_color_scheme.set(dft.scheme);
                }
                if vars.0.accent_color.get().is_none() {
                    vars.0.actual_accent_color.set(dft.accent);
                }
            }
            prev_parent = *a.value();
            true
        })
        .perm();

    // update children that do not override
    vars.0
        .actual_color_scheme
        .hook(move |a| {
            if let Some(vars) = WINDOWS.vars(id) {
                vars.0.children.with(|c| {
                    for &id in c {
                        if let Some(vars) = WINDOWS.vars(id)
                            && vars.0.color_scheme.get().is_none()
                        {
                            vars.0.actual_color_scheme.set(*a.value());
                        }
                    }
                });
                true
            } else {
                false
            }
        })
        .perm();
    // update children that do not override
    vars.0
        .actual_accent_color
        .hook(move |a| {
            if let Some(vars) = WINDOWS.vars(id) {
                vars.0.children.with(|c| {
                    for &id in c {
                        if let Some(vars) = WINDOWS.vars(id)
                            && vars.0.accent_color.get().is_none()
                        {
                            vars.0.actual_accent_color.set(*a.value());
                        }
                    }
                });
                true
            } else {
                false
            }
        })
        .perm();

    #[cfg(feature = "image")]
    vars.0
        .frame_capture_mode
        .hook(move |a| {
            with_view(id, |_, _, v| {
                let _ = v.set_capture_mode(matches!(
                    *a.value(),
                    crate::FrameCaptureMode::All | crate::FrameCaptureMode::AllMask(_)
                ));
            });
            true
        })
        .perm();
}

// this matches the window layout monitor root
fn with_monitor_layout(id: WindowId, f: impl FnOnce(&WindowInstance, &WindowVars, PxRect, Factor, PxDensity)) {
    if let Some(w) = WINDOWS_SV.read().windows.get(&id) {
        let vars = w.vars.as_ref().unwrap();

        // define monitor context
        let mut monitor_ctx = None;
        if let Some(m) = vars.0.actual_monitor.get().and_then(|id| MONITORS.monitor(id)) {
            monitor_ctx = Some((m.px_rect(), m.scale_factor().get(), m.density().get()));
        } else if w.mode.is_headless()
            && let Some(r) = &w.root
        {
            let m = &r.root.lock().headless_monitor;
            let f = m.scale_factor.unwrap_or(1.fct());
            monitor_ctx = Some((PxRect::from_size(m.size.to_px(f)), f, m.density));
        }

        if let Some((monitor_rect, scale_factor, monitor_density)) = monitor_ctx {
            // layout in the monitor context
            let metrics = LayoutMetrics::new(scale_factor, monitor_rect.size, Length::pt_to_px(11.0, scale_factor))
                .with_screen_density(monitor_density)
                .with_direction(DIRECTION_VAR.get());

            LAYOUT.with_root_context(LayoutPassId::new(), metrics, || {
                f(w, vars, monitor_rect, scale_factor, monitor_density)
            });
        }
    }
}

fn on_state_changed(id: WindowId, s: &zng_var::AnyVarHookArgs<'_>) -> bool {
    if !s.contains_tag(&SetFromViewTag)
        && let Some(vars) = WINDOWS.vars(id)
        && !vars.0.pending_state_update.swap(true, std::sync::atomic::Ordering::Relaxed)
    {
        // multiple state change requests can be made during hook updates, we want to update view-process once
        UPDATES
            .run_hn_once(hn_once!(|_| {
                if let Some(w) = WINDOWS_SV.read().windows.get(&id)
                    && let Some(vars) = &w.vars
                    && vars.0.pending_state_update.swap(false, std::sync::atomic::Ordering::Relaxed)
                    && let Some(r) = &w.root
                    && let Some(v) = &r.view_window
                {
                    let state = vars.window_state_all();
                    let _ = v.set_state(state);
                }
            }))
            .perm();
    }
    true
}

fn with_view(id: WindowId, f: impl FnOnce(&WindowInstance, &WindowNode, &ViewWindow)) {
    if let Some(w) = WINDOWS_SV.read().windows.get(&id)
        && let Some(r) = &w.root
        && let Some(v) = &r.view_window
    {
        f(w, r, v)
    }
}

/// Load icon images for window and cursor
#[cfg(feature = "image")]
fn load_bind_ico<T: zng_var::VarValue>(
    id: WindowId,
    mut source: zng_ext_image::ImageSource,
    target: &zng_var::Var<T>,
    system_cap: WindowCapability,
    debug_name: &'static str,
    mut map: impl FnMut(&zng_ext_image::ImageEntry) -> T + Send + 'static,
) -> VarHandle {
    use zng_app::view_process::VIEW_PROCESS;
    use zng_ext_image::{ImageRenderArgs, ImageSource};
    use zng_layout::unit::TimeUnits as _;

    // if VIEW_PROCESS is connected can check if system can change window icon,
    // if it cannot can skip loading icon images
    let skip = VIEW_PROCESS.is_connected() && !VIEW_PROCESS.info().window.contains(system_cap);
    if skip {
        tracing::warn!("view-process cannot {system_cap:?} in the current system");
        return VarHandle::dummy();
    }

    // load image and "reduced" alternates, common in ICO files
    let mut opt = zng_ext_image::ImageOptions::cache();
    opt.entries = zng_ext_image::ImageEntriesMode::REDUCED;
    if let ImageSource::Render(_, a) = &mut source {
        *a = Some(ImageRenderArgs::new(id));
    }
    let icon = zng_ext_image::IMAGES.image(source, opt, None);

    // hold window open up to 1s to show the icon from the start
    let mut _load_handle = WINDOWS.loading_handle(id, 1.secs(), debug_name);
    icon.set_bind_map(target, move |i| {
        if i.is_loaded() {
            _load_handle = None;
        }
        map(i)
    })
}

/// FOCUS service focused hook
pub(crate) fn focused_widget_handler() -> impl FnMut(&Option<InteractionPath>) + Send + 'static {
    let mut prev_ime_area = None::<(WindowId, WidgetId)>;
    let mut _render_handle = VarHandle::dummy();
    move |focused| {
        // focus service calls WINDOWS.focus, this is the focused
        let s = WINDOWS_SV.read();

        // update IME exclusion area

        // get new ime area
        let mut new_ime_area = None;
        let mut area = DipRect::zero();
        if let Some(p) = focused
            && p.interactivity().is_enabled()
            && let Some(w) = s.windows.get(&p.window_id())
            && let Some(tree) = &w.info
            && let Some(info) = tree.get(p.widget_id())
            && let Some(r) = info.ime_area()
        {
            new_ime_area = Some((p.window_id(), p.widget_id()));
            area = r.to_dip(tree.scale_factor());
        }

        if prev_ime_area == new_ime_area {
            return;
        }

        // clear previous ime area
        if let Some((win, _)) = prev_ime_area
            && let Some(w) = s.windows.get(&win)
            && let Some(r) = &w.root
            && let Some(v) = &r.view_window
        {
            if new_ime_area.map(|(i, _)| i) == Some(win) {
                // or replace it, if is same window
                let _ = v.set_ime_area(Some(area));
                _render_handle = hook_ime_area_update(win, new_ime_area.unwrap().1);
                prev_ime_area = new_ime_area;
                return;
            }

            let _ = v.set_ime_area(None);
        } else {
            prev_ime_area = None;
            _render_handle = VarHandle::dummy();
        }

        if let Some((win, wgt)) = new_ime_area
            && let Some(w) = s.windows.get(&win)
            && let Some(r) = &w.root
            && let Some(v) = &r.view_window
        {
            let _ = v.set_ime_area(Some(area));
            prev_ime_area = new_ime_area;
            _render_handle = hook_ime_area_update(win, wgt);
        }
    }
}
fn hook_ime_area_update(window_id: WindowId, area_id: WidgetId) -> VarHandle {
    WIDGET_TREE_CHANGED_EVENT.hook(move |args| {
        if args.tree.window_id() == window_id {
            if let Some(area) = args.tree.get(area_id)
                && let Some(area) = area.ime_area()
                && let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
                && let Some(r) = &w.root
                && let Some(v) = &r.view_window
            {
                let _ = v.set_ime_area(Some(area.to_dip(args.tree.scale_factor())));
            } else {
                return false;
            }
        }
        true
    })
}
