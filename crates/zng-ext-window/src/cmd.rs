//! Commands that control the scoped window.

use std::sync::Arc;

use zng_app::{
    event::{CommandHandle, CommandInfoExt, CommandNameExt, CommandScope, command},
    hn,
    shortcut::{CommandShortcutExt, shortcut},
    view_process::VIEW_PROCESS,
    window::WindowId,
};

use zng_layout::unit::{Dip, DipPoint, PxPoint, PxToDip as _};
use zng_view_api::window::{WindowCapability, WindowState};
use zng_wgt::{CommandIconExt as _, ICONS, prelude::clmv, wgt_fn};

use crate::{IME_EVENT, ImeArgs, WINDOWS, WINDOWS_SV, WindowInstanceState, WindowVars};

pub use zng_view_api::window::ResizeDirection;

command! {
    /// Represents the window **close** action.
    pub static CLOSE_CMD {
        l10n!: true,
        name: "Close",
        info: "Close the window",
        shortcut: [shortcut!(ALT + F4), shortcut!(CTRL + 'W')],
        icon: wgt_fn!(|_| ICONS.get(["window-close", "close"])),
    };

    /// Represents the window **minimize** action.
    pub static MINIMIZE_CMD {
        l10n!: true,
        name: "Minimize",
        info: "Minimize the window",
        icon: wgt_fn!(|_| ICONS.get(["window-minimize"])),
    };

    /// Represents the window **maximize** action.
    pub static MAXIMIZE_CMD {
        l10n!: true,
        name: "Maximize",
        info: "Maximize the window",
        icon: wgt_fn!(|_| ICONS.get(["window-maximize"])),
    };

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Behavior
    ///
    /// This command is about the *windowed* fullscreen state ([`WindowState::Fullscreen`]),
    /// use the [`EXCLUSIVE_FULLSCREEN_CMD`] to toggle *exclusive* video mode fullscreen.
    pub static FULLSCREEN_CMD {
        l10n!: true,
        name: "Fullscreen",
        info: "Toggle fullscreen mode on the window",
        shortcut: {
            let a = if cfg!(target_os = "macos") {
                shortcut!(CTRL | SHIFT + 'F')
            } else {
                shortcut!(F11)
            };
            [a, shortcut!(ZoomToggle)]
        },
        icon: wgt_fn!(|_| ICONS.get(["window-windowed-fullscreen", "window-fullscreen", "fullscreen"])),
    };

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Behavior
    ///
    /// This command is about the *exclusive* fullscreen state ([`WindowState::Exclusive`]),
    /// use the [`FULLSCREEN_CMD`] to toggle *windowed* fullscreen.
    pub static EXCLUSIVE_FULLSCREEN_CMD {
        l10n!: true,
        name: "Exclusive Fullscreen",
        info: "Toggle exclusive fullscreen mode on the window",
        icon: wgt_fn!(|_| ICONS.get(["window-exclusive-fullscreen", "window-fullscreen", "fullscreen"])),
    };

    /// Represents the window **restore** action.
    ///
    /// Restores the window to its previous non-minimized state or normal state.
    pub static RESTORE_CMD {
        l10n!: true,
        name: "Restore",
        info: "Restores the window to its previous non-minimized state or normal state",
        icon: wgt_fn!(|_| ICONS.get(["window-restore"])),
    };

    /// Represents the **close IME** action.
    ///
    /// If any IME preview is active close it without committing.
    pub static CANCEL_IME_CMD;

    /// Represents the window **drag-move** and **drag-resize** actions.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed immediately before this command is called.
    ///
    /// # Parameter
    ///
    /// If this command is called without parameter the window will drag-move, if it is called with a [`ResizeDirection`] the
    /// window will drag-resize.
    pub static DRAG_MOVE_RESIZE_CMD;

    /// Represents the window **open title bar context menu** action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`DipPoint`] or [`PxPoint`] that defines
    /// the menu position.
    ///
    /// [`DipPoint`]: zng_layout::unit::DipPoint
    /// [`PxPoint`]: zng_layout::unit::PxPoint
    pub static OPEN_TITLE_BAR_CONTEXT_MENU_CMD;
}

pub(super) struct WindowCommands {
    maximize_handle: CommandHandle,
    minimize_handle: CommandHandle,
    restore_handle: CommandHandle,

    fullscreen_handle: CommandHandle,
    exclusive_handle: CommandHandle,

    close_handle: CommandHandle,
}
impl WindowCommands {
    /// Setup command handlers, handles live in the WindowVars hooks.
    pub fn init(id: WindowId, vars: &WindowVars) {
        let s = vars.0.state.get();
        let c = WindowCommands {
            maximize_handle: MAXIMIZE_CMD.scoped(id).on_event(
                !matches!(s, WindowState::Maximized) && vars.0.can_maximize.get(),
                true,
                false,
                hn!(|args| {
                    args.propagation.stop();
                    WINDOWS.vars(id).unwrap().0.state.set(WindowState::Maximized);
                }),
            ),
            minimize_handle: MINIMIZE_CMD.scoped(id).on_event(
                !matches!(s, WindowState::Minimized) && vars.0.can_minimize.get(),
                true,
                false,
                hn!(|args| {
                    args.propagation.stop();
                    WINDOWS.vars(id).unwrap().0.state.set(WindowState::Minimized);
                }),
            ),
            restore_handle: RESTORE_CMD.scoped(id).on_event(
                !matches!(s, WindowState::Normal),
                true,
                false,
                hn!(|args| {
                    args.propagation.stop();
                    let vars = WINDOWS.vars(id).unwrap();
                    vars.0.state.set(vars.0.restore_state.get());
                }),
            ),
            fullscreen_handle: FULLSCREEN_CMD.scoped(id).on_event(
                matches!(s, WindowState::Fullscreen) || vars.0.can_fullscreen.get(),
                true,
                false,
                hn!(|args| {
                    let vars = WINDOWS.vars(id).unwrap();
                    if let WindowState::Fullscreen = vars.0.state.get() {
                        vars.0.state.set(vars.0.restore_state.get());
                    } else {
                        vars.0.state.set(WindowState::Fullscreen);
                    }
                }),
            ),
            exclusive_handle: EXCLUSIVE_FULLSCREEN_CMD.scoped(id).on_event(
                matches!(s, WindowState::Exclusive) || vars.0.can_fullscreen.get(),
                true,
                false,
                hn!(|args| {
                    let vars = WINDOWS.vars(id).unwrap();
                    if let WindowState::Exclusive = vars.0.state.get() {
                        vars.0.state.set(vars.0.restore_state.get());
                    } else {
                        vars.0.state.set(WindowState::Exclusive);
                    }
                }),
            ),
            close_handle: CLOSE_CMD.scoped(id).on_event(
                vars.0.can_close.get(),
                true,
                false,
                hn!(|args| {
                    args.propagation.stop();
                    let _ = WINDOWS.close(id);
                }),
            ),
        };

        vars.0.can_close.bind(c.close_handle.enabled()).perm();

        let c = Arc::new(c);
        vars.0
            .state
            .hook(clmv!(c, |a| {
                if let Some(vars) = WINDOWS.vars(id) {
                    let s = *a.value();
                    c.maximize_handle
                        .enabled()
                        .set(!matches!(s, WindowState::Maximized) && vars.0.can_maximize.get());
                    c.minimize_handle
                        .enabled()
                        .set(!matches!(s, WindowState::Minimized) && vars.0.can_minimize.get());
                    c.restore_handle.enabled().set(!matches!(s, WindowState::Normal));
                    c.fullscreen_handle
                        .enabled()
                        .set(matches!(s, WindowState::Fullscreen) || vars.0.can_fullscreen.get());
                    c.exclusive_handle
                        .enabled()
                        .set(matches!(s, WindowState::Exclusive) || vars.0.can_fullscreen.get());
                    true
                } else {
                    false
                }
            }))
            .perm();
        vars.0
            .can_maximize
            .hook(clmv!(c, |a| {
                if let Some(vars) = WINDOWS.vars(id) {
                    let s = vars.0.state.get();
                    c.maximize_handle.enabled().set(!matches!(s, WindowState::Maximized) && *a.value());
                    true
                } else {
                    false
                }
            }))
            .perm();
        vars.0
            .can_minimize
            .hook(clmv!(c, |a| {
                if let Some(vars) = WINDOWS.vars(id) {
                    let s = vars.0.state.get();
                    c.minimize_handle.enabled().set(!matches!(s, WindowState::Minimized) && *a.value());
                    true
                } else {
                    false
                }
            }))
            .perm();
        vars.0
            .can_fullscreen
            .hook(move |a| {
                if let Some(vars) = WINDOWS.vars(id) {
                    let s = vars.0.state.get();
                    c.fullscreen_handle
                        .enabled()
                        .set(matches!(s, WindowState::Fullscreen) || *a.value());
                    c.exclusive_handle.enabled().set(matches!(s, WindowState::Exclusive) || *a.value());
                    true
                } else {
                    false
                }
            })
            .perm();

        fn can_open_ctx_menu(state: WindowInstanceState) -> bool {
            matches!(state, WindowInstanceState::Loaded { has_view: true })
                && VIEW_PROCESS.info().window.contains(WindowCapability::OPEN_TITLE_BAR_CONTEXT_MENU)
        }
        let handle = OPEN_TITLE_BAR_CONTEXT_MENU_CMD.scoped(id).on_event(
            can_open_ctx_menu(vars.0.instance_state.get()),
            true,
            false,
            hn!(|args| {
                if let Some(w) = WINDOWS_SV.read().windows.get(&id)
                    && let Some(vars) = &w.vars
                    && let Some(v) = &w.view_window
                {
                    args.propagation.stop();

                    let pos = if let Some(p) = args.param::<DipPoint>() {
                        *p
                    } else if let Some(p) = args.param::<PxPoint>() {
                        p.to_dip(vars.0.scale_factor.get())
                    } else {
                        DipPoint::splat(Dip::new(24))
                    };

                    let _ = v.open_title_bar_context_menu(pos);
                }
            }),
        );
        vars.0
            .instance_state
            .hook(move |a| {
                handle.enabled().set(can_open_ctx_menu(a.value().clone()));
                true
            })
            .perm();

        let handle = CANCEL_IME_CMD.scoped(id).on_event(
            false,
            false,
            false,
            hn!(|args| {
                let s = WINDOWS_SV.read();
                if let Some(w) = s.windows.get(&id)
                    && let Some(v) = &w.view_window
                    && let Some(f) = s.focused.get()
                    && f.window_id() == id
                    && (matches!(args.scope, CommandScope::Window(w) if w == id)
                        || matches!(args.scope, CommandScope::Widget(w) if w == f.widget_id()))
                {
                    args.propagation.stop();

                    let _ = v.set_ime_area(None);

                    IME_EVENT.notify(ImeArgs::now(f, "", None));
                }
            }),
        );
        vars.0
            .focused
            .hook(move |a| {
                handle.enabled().set(*a.value());
                true
            })
            .perm();

        let handle = DRAG_MOVE_RESIZE_CMD.scoped(id).on_event(
            vars.0.resizable.get(),
            true,
            false,
            hn!(|args| {
                if let Some(w) = WINDOWS_SV.read().windows.get(&id)
                    && let Some(v) = &w.view_window
                {
                    args.propagation.stop();
                    let _ = match args.param::<crate::cmd::ResizeDirection>() {
                        Some(r) => v.drag_resize(*r),
                        None => v.drag_move(),
                    };
                }
            }),
        );
        vars.0
            .resizable
            .hook(move |a| {
                handle.enabled().set(*a.value());
                true
            })
            .perm();
    }
}
