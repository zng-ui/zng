//! Commands that control the scoped window.

use zng_app::{
    event::{CommandHandle, CommandInfoExt, CommandNameExt, EventArgs, command},
    hn,
    shortcut::{CommandShortcutExt, shortcut},
    window::WindowId,
};

use zng_view_api::window::WindowState;
use zng_wgt::{CommandIconExt as _, ICONS, wgt_fn};

use crate::{WINDOWS, WindowVars};

pub use zng_view_api::window::ResizeDirection;

command! {
    /// Represents the window **close** action.
    pub static CLOSE_CMD = {
        l10n!: true,
        name: "Close",
        info: "Close the window",
        shortcut: [shortcut!(ALT + F4), shortcut!(CTRL + 'W')],
        icon: wgt_fn!(|_| ICONS.get(["window-close", "close"])),
    };

    /// Represents the window **minimize** action.
    pub static MINIMIZE_CMD = {
        l10n!: true,
        name: "Minimize",
        info: "Minimize the window",
        icon: wgt_fn!(|_| ICONS.get(["window-minimize"])),
    };

    /// Represents the window **maximize** action.
    pub static MAXIMIZE_CMD = {
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
    pub static FULLSCREEN_CMD = {
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
    pub static EXCLUSIVE_FULLSCREEN_CMD = {
        l10n!: true,
        name: "Exclusive Fullscreen",
        info: "Toggle exclusive fullscreen mode on the window",
        icon: wgt_fn!(|_| ICONS.get(["window-exclusive-fullscreen", "window-fullscreen", "fullscreen"])),
    };

    /// Represents the window **restore** action.
    ///
    /// Restores the window to its previous non-minimized state or normal state.
    pub static RESTORE_CMD = {
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
    /// Setup command handlers, handles live in the WindowVars::state var.
    pub fn init(window_id: WindowId, window_vars: &WindowVars) {
        let state = window_vars.state();
        let restore_state = window_vars.restore_state();
        let s = state.get();
        let c = WindowCommands {
            maximize_handle: MAXIMIZE_CMD.scoped(window_id).on_event(
                !matches!(s, WindowState::Maximized),
                true,
                false,
                hn!(state, |args| {
                    args.propagation().stop();
                    state.set(WindowState::Maximized);
                }),
            ),
            minimize_handle: MINIMIZE_CMD.scoped(window_id).on_event(
                !matches!(s, WindowState::Minimized),
                true,
                false,
                hn!(state, |args| {
                    args.propagation().stop();
                    state.set(WindowState::Minimized);
                }),
            ),
            restore_handle: RESTORE_CMD.scoped(window_id).on_event(
                !matches!(s, WindowState::Normal),
                true,
                false,
                hn!(state, restore_state, |args| {
                    args.propagation().stop();
                    state.set(restore_state.get());
                }),
            ),
            fullscreen_handle: FULLSCREEN_CMD.scoped(window_id).on_event(
                true,
                true,
                false,
                hn!(state, restore_state, |args| {
                    if let WindowState::Fullscreen = state.get() {
                        state.set(restore_state.get());
                    } else {
                        state.set(WindowState::Fullscreen);
                    }
                }),
            ),
            exclusive_handle: EXCLUSIVE_FULLSCREEN_CMD.scoped(window_id).on_event(
                true,
                true,
                false,
                hn!(state, |args| {
                    if let WindowState::Exclusive = state.get() {
                        state.set(restore_state.get());
                    } else {
                        state.set(WindowState::Exclusive);
                    }
                }),
            ),
            close_handle: CLOSE_CMD.scoped(window_id).on_event(
                true,
                true,
                false,
                hn!(|args| {
                    args.propagation().stop();
                    let _ = WINDOWS.close(window_id);
                }),
            ),
        };

        state
            .hook(move |a| {
                let state = *a.value();
                c.restore_handle.enabled().set(state != WindowState::Normal);
                c.maximize_handle.enabled().set(state != WindowState::Maximized);
                c.minimize_handle.enabled().set(state != WindowState::Minimized);
                true
            })
            .perm();
    }
}
