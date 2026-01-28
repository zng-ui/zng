//! Commands that control focus and [`Command`] extensions.
//!
//! [`Command`]: zng_app::event::Command

use zng_app::{
    event::{Command, CommandHandle, CommandInfoExt, CommandNameExt, CommandScope, command},
    hn,
    shortcut::{CommandShortcutExt, shortcut},
    widget::info::WidgetInfo,
};
use zng_ext_window::WINDOWS;
use zng_var::{Var, merge_var};

use super::*;

command! {
    /// Represents the **focus next** action.
    pub static FOCUS_NEXT_CMD {
        l10n!: true,
        name: "Focus Next",
        info: "Focus next focusable",
        shortcut: shortcut!(Tab),
    };

    /// Represents the **focus previous** action.
    pub static FOCUS_PREV_CMD {
        l10n!: true,
        name: "Focus Previous",
        info: "Focus previous focusable",
        shortcut: shortcut!(SHIFT + Tab),
    };

    /// Represents the **focus/escape alt** action.
    pub static FOCUS_ALT_CMD {
        l10n!: true,
        name: "Focus Alt",
        info: "Focus alt scope",
        shortcut: shortcut!(Alt),
    };

    /// Represents the **focus enter** action.
    pub static FOCUS_ENTER_CMD {
        l10n!: true,
        name: "Focus Enter",
        info: "Focus child focusable",
        shortcut: [shortcut!(Enter), shortcut!(ALT + Enter)],
    };

    /// Represents the **focus exit** action.
    pub static FOCUS_EXIT_CMD {
        l10n!: true,
        name: "Focus Exit",
        info: "Focus parent focusable, or return focus",
        shortcut: [shortcut!(Escape), shortcut!(ALT + Escape)],
    };

    /// Represents the **focus up** action.
    pub static FOCUS_UP_CMD {
        l10n!: true,
        name: "Focus Up",
        info: "Focus closest focusable up",
        shortcut: [shortcut!(ArrowUp), shortcut!(ALT + ArrowUp)],
    };

    /// Represents the **focus down** action.
    pub static FOCUS_DOWN_CMD {
        l10n!: true,
        name: "Focus Down",
        info: "Focus closest focusable down",
        shortcut: [shortcut!(ArrowDown), shortcut!(ALT + ArrowDown)],
    };

    /// Represents the **focus left** action.
    pub static FOCUS_LEFT_CMD {
        l10n!: true,
        name: "Focus Left",
        info: "Focus closest focusable left",
        shortcut: [shortcut!(ArrowLeft), shortcut!(ALT + ArrowLeft)],
    };

    /// Represents the **focus right** action.
    pub static FOCUS_RIGHT_CMD {
        l10n!: true,
        name: "Focus Right",
        info: "Focus closest focusable right",
        shortcut: [shortcut!(ArrowRight), shortcut!(ALT + ArrowRight)],
    };

    /// Represents a [`FocusRequest`] action.
    ///
    /// If this command parameter is a [`FocusRequest`] the request is made.
    pub static FOCUS_CMD;
}

pub(super) struct FocusCommands {
    next_handle: CommandHandle,
    prev_handle: CommandHandle,

    alt_handle: CommandHandle,

    up_handle: CommandHandle,
    down_handle: CommandHandle,
    left_handle: CommandHandle,
    right_handle: CommandHandle,

    exit_handle: CommandHandle,
    enter_handle: CommandHandle,

    _focus_handle: CommandHandle,
}
impl FocusCommands {
    pub fn new() -> Self {
        macro_rules! handle {
            ($($CMD:ident($handle:ident) => $method:ident,)+) => {Self {
                $($handle: $CMD.on_event(false, true, false, hn!(|args| {
                    args.propagation.stop();
                    FOCUS.$method();
                })),)+
                _focus_handle: FOCUS_CMD.on_event(true, true, false, hn!(|args| {
                    if let Some(req) = args.param::<FocusRequest>() {
                        args.propagation.stop();
                        FOCUS.focus(*req);
                    }
                })),
            }};
        }

        #[rustfmt::skip] // for zng fmt
        handle! {
            FOCUS_NEXT_CMD(next_handle) => focus_next,
            FOCUS_PREV_CMD(prev_handle) => focus_prev,
            FOCUS_ALT_CMD(alt_handle) => focus_alt,
            FOCUS_UP_CMD(up_handle) => focus_up,
            FOCUS_DOWN_CMD(down_handle) => focus_down,
            FOCUS_LEFT_CMD(left_handle) => focus_left,
            FOCUS_RIGHT_CMD(right_handle) => focus_right,
            FOCUS_ENTER_CMD(enter_handle) => focus_enter,
            FOCUS_EXIT_CMD(exit_handle) => focus_exit,
        }
    }

    pub fn update_enabled(&mut self, nav: FocusNavAction) {
        self.next_handle.enabled().set(nav.contains(FocusNavAction::NEXT));
        self.prev_handle.enabled().set(nav.contains(FocusNavAction::PREV));

        self.alt_handle.enabled().set(nav.contains(FocusNavAction::ALT));

        self.up_handle.enabled().set(nav.contains(FocusNavAction::UP));
        self.down_handle.enabled().set(nav.contains(FocusNavAction::DOWN));
        self.left_handle.enabled().set(nav.contains(FocusNavAction::LEFT));
        self.right_handle.enabled().set(nav.contains(FocusNavAction::RIGHT));

        self.exit_handle.enabled().set(nav.contains(FocusNavAction::EXIT));
        self.enter_handle.enabled().set(nav.contains(FocusNavAction::ENTER));
    }
}

/// Focus extension methods for commands.
pub trait CommandFocusExt {
    /// Gets a command variable with `self` scoped to the focused (non-alt) widget or app.
    ///
    /// The scope is [`alt_return`] if is set, otherwise it is [`focused`], otherwise the
    /// command is not scoped (app scope). This means that you can bind the command variable to
    /// a menu or toolbar button inside an *alt-scope* without losing track of the intended target
    /// of the command.
    ///
    /// [`alt_return`]: FOCUS::alt_return
    /// [`focused`]: FOCUS::focused
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # zng_app::command! { pub static PASTE_CMD; }
    /// # use zng_ext_input::focus::cmd::CommandFocusExt as _;
    /// # use zng_var::*;
    /// # fn main() {
    /// let paste_in_focused_cmd = PASTE_CMD.focus_scoped();
    /// let is_enabled = paste_in_focused_cmd.flat_map(|c| c.is_enabled());
    /// paste_in_focused_cmd.get().notify();
    /// # }
    /// ```
    fn focus_scoped(self) -> Var<Command>;

    /// Gets a command variable with `self` scoped to the output of `map`.
    ///
    /// The `map` closure is called every time the non-alt focused widget changes, that is the [`alt_return`] or
    /// the [`focused`]. The closure input is the [`WidgetInfo`] for the focused widget and the output must be
    /// a [`CommandScope`] for the command.
    ///
    /// [`alt_return`]: FOCUS::alt_return
    /// [`focused`]: FOCUS::focused
    /// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
    /// [`CommandScope`]: zng_app::event::CommandScope
    fn focus_scoped_with(self, map: impl FnMut(Option<WidgetInfo>) -> CommandScope + Send + 'static) -> Var<Command>;
}

impl CommandFocusExt for Command {
    fn focus_scoped(self) -> Var<Command> {
        let cmd = self.scoped(CommandScope::App);
        merge_var!(FOCUS.alt_return(), FOCUS.focused(), move |alt, f| {
            match alt.as_ref().or(f.as_ref()) {
                Some(p) => cmd.scoped(p.widget_id()),
                None => cmd,
            }
        })
    }

    fn focus_scoped_with(self, mut map: impl FnMut(Option<WidgetInfo>) -> CommandScope + Send + 'static) -> Var<Command> {
        let cmd = self.scoped(CommandScope::App);
        merge_var!(FOCUS.alt_return(), FOCUS.focused(), |alt, f| {
            match alt.as_ref().or(f.as_ref()) {
                Some(p) => WINDOWS.widget_tree(p.window_id())?.get(p.widget_id()),
                None => None,
            }
        })
        .map(move |w| cmd.scoped(map(w.clone())))
    }
}
