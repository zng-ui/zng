//! Commands that control focus and [`Command`] extensions.
//!
//! [`Command`]: zng_app::event::Command

use zng_app::{
    event::{Command, CommandHandle, CommandInfoExt, CommandNameExt, CommandScope, EventArgs, command},
    shortcut::{CommandShortcutExt, shortcut},
    update::EventUpdate,
    widget::info::WidgetInfo,
};
use zng_var::{BoxedVar, merge_var};

use super::*;

command! {
    /// Represents the **focus next** action.
    pub static FOCUS_NEXT_CMD = {
        l10n!: true,
        name: "Focus Next",
        info: "Focus next focusable",
        shortcut: shortcut!(Tab),
    };

    /// Represents the **focus previous** action.
    pub static FOCUS_PREV_CMD = {
        l10n!: true,
        name: "Focus Previous",
        info: "Focus previous focusable",
        shortcut: shortcut!(SHIFT+Tab),
    };

    /// Represents the **focus/escape alt** action.
    pub static FOCUS_ALT_CMD = {
        l10n!: true,
        name: "Focus Alt",
        info: "Focus alt scope",
        shortcut: shortcut!(Alt),
    };

    /// Represents the **focus enter** action.
    pub static FOCUS_ENTER_CMD = {
        l10n!: true,
        name: "Focus Enter",
        info: "Focus child focusable",
        shortcut: [shortcut!(Enter), shortcut!(ALT+Enter)],
    };

    /// Represents the **focus exit** action.
    pub static FOCUS_EXIT_CMD = {
        l10n!: true,
        name: "Focus Exit",
        info: "Focus parent focusable, or return focus",
        shortcut: [shortcut!(Escape), shortcut!(ALT+Escape)],
    };

    /// Represents the **focus up** action.
    pub static FOCUS_UP_CMD = {
        l10n!: true,
        name: "Focus Up",
        info: "Focus closest focusable up",
        shortcut: [shortcut!(ArrowUp), shortcut!(ALT+ArrowUp)],
    };

    /// Represents the **focus down** action.
    pub static FOCUS_DOWN_CMD = {
        l10n!: true,
        name: "Focus Down",
        info: "Focus closest focusable down",
        shortcut: [shortcut!(ArrowDown), shortcut!(ALT+ArrowDown)],
    };

    /// Represents the **focus left** action.
    pub static FOCUS_LEFT_CMD = {
        l10n!: true,
        name: "Focus Left",
        info: "Focus closest focusable left",
        shortcut: [shortcut!(ArrowLeft), shortcut!(ALT+ArrowLeft)],
    };

    /// Represents the **focus right** action.
    pub static FOCUS_RIGHT_CMD = {
        l10n!: true,
        name: "Focus Right",
        info: "Focus closest focusable right",
        shortcut: [shortcut!(ArrowRight), shortcut!(ALT+ArrowRight)],
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

    focus_handle: CommandHandle,
}
impl FocusCommands {
    pub fn new() -> Self {
        Self {
            next_handle: FOCUS_NEXT_CMD.subscribe(false),
            prev_handle: FOCUS_PREV_CMD.subscribe(false),

            alt_handle: FOCUS_ALT_CMD.subscribe(false),

            up_handle: FOCUS_UP_CMD.subscribe(false),
            down_handle: FOCUS_DOWN_CMD.subscribe(false),
            left_handle: FOCUS_LEFT_CMD.subscribe(false),
            right_handle: FOCUS_RIGHT_CMD.subscribe(false),

            exit_handle: FOCUS_EXIT_CMD.subscribe(false),
            enter_handle: FOCUS_ENTER_CMD.subscribe(false),

            focus_handle: FOCUS_CMD.subscribe(true),
        }
    }

    pub fn update_enabled(&mut self, nav: FocusNavAction) {
        self.next_handle.set_enabled(nav.contains(FocusNavAction::NEXT));
        self.prev_handle.set_enabled(nav.contains(FocusNavAction::PREV));

        self.alt_handle.set_enabled(nav.contains(FocusNavAction::ALT));

        self.up_handle.set_enabled(nav.contains(FocusNavAction::UP));
        self.down_handle.set_enabled(nav.contains(FocusNavAction::DOWN));
        self.left_handle.set_enabled(nav.contains(FocusNavAction::LEFT));
        self.right_handle.set_enabled(nav.contains(FocusNavAction::RIGHT));

        self.exit_handle.set_enabled(nav.contains(FocusNavAction::EXIT));
        self.enter_handle.set_enabled(nav.contains(FocusNavAction::ENTER));
    }

    pub fn event_preview(&mut self, update: &EventUpdate) {
        macro_rules! handle {
            ($($CMD:ident($handle:ident) => $method:ident,)+) => {$(
                if let Some(args) = $CMD.on(update) {
                    args.handle(|args| {
                        if args.enabled && self.$handle.is_enabled() {
                            FOCUS.$method();
                        } else {
                            FOCUS.on_disabled_cmd();
                        }
                    });
                    return;
                }
            )+};
        }
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

        if let Some(args) = FOCUS_CMD.on(update) {
            if let Some(req) = args.param::<FocusRequest>() {
                args.handle_enabled(&self.focus_handle, |_| {
                    FOCUS.focus(*req);
                });
            }
        }
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
    fn focus_scoped(self) -> BoxedVar<Command>;

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
    fn focus_scoped_with(self, map: impl FnMut(Option<WidgetInfo>) -> CommandScope + Send + 'static) -> BoxedVar<Command>;
}

impl CommandFocusExt for Command {
    fn focus_scoped(self) -> BoxedVar<Command> {
        let cmd = self.scoped(CommandScope::App);
        merge_var!(FOCUS.alt_return(), FOCUS.focused(), move |alt, f| {
            match alt.as_ref().or(f.as_ref()) {
                Some(p) => cmd.scoped(p.widget_id()),
                None => cmd,
            }
        })
        .boxed()
    }

    fn focus_scoped_with(self, mut map: impl FnMut(Option<WidgetInfo>) -> CommandScope + Send + 'static) -> BoxedVar<Command> {
        let cmd = self.scoped(CommandScope::App);
        merge_var!(FOCUS.alt_return(), FOCUS.focused(), |alt, f| {
            match alt.as_ref().or(f.as_ref()) {
                Some(p) => WINDOWS.widget_tree(p.window_id()).ok()?.get(p.widget_id()),
                None => None,
            }
        })
        .map(move |w| cmd.scoped(map(w.clone())))
        .boxed()
    }
}
