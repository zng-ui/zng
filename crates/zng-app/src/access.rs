//! Accessibility/automation events.

use zng_txt::Txt;
use zng_view_api::access::AccessCmd;

pub use zng_view_api::access::ScrollCmd;

use crate::{
    event::{event, event_args},
    widget::{WidgetId, info::WidgetPath},
    window::{WINDOWS_APP, WindowId},
};

pub(super) fn on_access_init(window_id: WindowId) {
    let args = AccessInitedArgs::now(window_id);
    ACCESS_INITED_EVENT.notify(args)
}

pub(super) fn on_access_deinit(window_id: WindowId) {
    let args = AccessDeinitedArgs::now(window_id);
    ACCESS_DEINITED_EVENT.notify(args)
}

fn find_wgt(window_id: WindowId, widget_id: WidgetId) -> Option<WidgetPath> {
    WINDOWS_APP.widget_tree(window_id)?.get(widget_id).map(|w| w.path())
}

pub(super) fn on_access_command(window_id: WindowId, widget_id: WidgetId, command: AccessCmd) {
    let widget = match find_wgt(window_id, widget_id) {
        Some(w) => w,
        None => return,
    };
    match command {
        AccessCmd::Click(primary) => {
            let args = AccessClickArgs::now(widget, primary);
            ACCESS_CLICK_EVENT.notify(args)
        }
        AccessCmd::Focus(focus) => {
            let args = AccessFocusArgs::now(widget, focus);
            ACCESS_FOCUS_EVENT.notify(args)
        }
        AccessCmd::FocusNavOrigin => {
            let args = AccessFocusNavOriginArgs::now(widget);
            ACCESS_FOCUS_NAV_ORIGIN_EVENT.notify(args)
        }
        AccessCmd::SetExpanded(expanded) => {
            let args = AccessExpanderArgs::now(widget, expanded);
            ACCESS_EXPANDER_EVENT.notify(args)
        }
        AccessCmd::Increment(inc) => {
            let args = AccessIncrementArgs::now(widget, inc);
            ACCESS_INCREMENT_EVENT.notify(args)
        }
        AccessCmd::SetToolTipVis(vis) => {
            let args = AccessToolTipArgs::now(widget, vis);
            ACCESS_TOOLTIP_EVENT.notify(args)
        }
        AccessCmd::ReplaceSelectedText(s) => {
            let args = AccessTextArgs::now(widget, s, true);
            ACCESS_TEXT_EVENT.notify(args)
        }
        AccessCmd::Scroll(s) => {
            let args = AccessScrollArgs::now(widget, s);
            ACCESS_SCROLL_EVENT.notify(args)
        }
        AccessCmd::SelectText {
            start: (start_wgt, start_idx),
            caret: (caret_wgt, caret_idx),
        } => {
            let start_wgt = match find_wgt(window_id, WidgetId::from_raw(start_wgt.0)) {
                Some(w) => w,
                None => return,
            };
            let caret_wgt = match find_wgt(window_id, WidgetId::from_raw(caret_wgt.0)) {
                Some(w) => w,
                None => return,
            };
            let args = AccessSelectionArgs::now((start_wgt, start_idx), (caret_wgt, caret_idx));
            ACCESS_SELECTION_EVENT.notify(args)
        }
        AccessCmd::SetString(s) => {
            let args = AccessTextArgs::now(widget, s, false);
            ACCESS_TEXT_EVENT.notify(args)
        }
        AccessCmd::SetNumber(n) => {
            let args = AccessNumberArgs::now(widget, n);
            ACCESS_NUMBER_EVENT.notify(args)
        }
        a => {
            tracing::warn!("access command `{a:?}` not implemented");
        }
    }
}

event_args! {
    /// Arguments for the [`ACCESS_INITED_EVENT`].
    pub struct AccessInitedArgs {
        /// Target window.
        pub window_id: WindowId,

        ..

        /// Broadcast to all.
        fn is_in_target(&self, _id: WidgetId) -> bool {
            true
        }
    }

    /// Arguments for the [`ACCESS_DEINITED_EVENT`].
    pub struct AccessDeinitedArgs {
        /// Target window.
        pub window_id: WindowId,

        ..

        /// Broadcast to all.
        fn is_in_target(&self, _id: WidgetId) -> bool {
            true
        }
    }

    /// Arguments for the [`ACCESS_CLICK_EVENT`].
    pub struct AccessClickArgs {
        /// Target.
        pub target: WidgetPath,

        /// Is primary click (default action).
        ///
        /// If `false` is context click.
        pub is_primary: bool,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_FOCUS_EVENT`].
    pub struct AccessFocusArgs {
        /// Target.
        pub target: WidgetPath,

        /// If the widget must be focused.
        ///
        /// If `true` the widget is focused, if `false` and the widget is focused, does ESC.
        pub focus: bool,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_FOCUS_NAV_ORIGIN_EVENT`].
    pub struct AccessFocusNavOriginArgs {
        /// Target.
        pub target: WidgetPath,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_EXPANDER_EVENT`].
    pub struct AccessExpanderArgs {
        /// Target.
        pub target: WidgetPath,

        /// New expanded value.
        pub expanded: bool,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_INCREMENT_EVENT`].
    pub struct AccessIncrementArgs {
        /// Target.
        pub target: WidgetPath,

        /// Increment steps.
        ///
        /// Usually is -1 or 1.
        pub delta: i32,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_TOOLTIP_EVENT`].
    pub struct AccessToolTipArgs {
        /// Target.
        pub target: WidgetPath,

        /// New tooltip visibility.
        pub visible: bool,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_SCROLL_EVENT`].
    pub struct AccessScrollArgs {
        /// Target.
        pub target: WidgetPath,

        /// Scroll command.
        pub command: ScrollCmd,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_TEXT_EVENT`].
    pub struct AccessTextArgs {
        /// Target.
        pub target: WidgetPath,

        /// Replacement text.
        pub txt: Txt,

        /// If only the selected text is replaced.
        ///
        /// Note that if the selection is empty the text is just inserted at the caret position, or is appended if there
        /// is no caret.
        pub selection_only: bool,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_NUMBER_EVENT`].
    pub struct AccessNumberArgs {
        /// Target.
        pub target: WidgetPath,

        /// Replacement number.
        pub num: f64,

        ..

        /// If is in `target`.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
        }
    }

    /// Arguments for the [`ACCESS_SELECTION_EVENT`].
    pub struct AccessSelectionArgs {
        /// Selection start.
        ///
        /// Text widget and character index where the selection *starts*.
        pub start: (WidgetPath, usize),
        /// Selection end.
        ///
        /// This is where the caret is placed, it does not need to be greater than the start.
        pub caret: (WidgetPath, usize),

        ..

        /// If is in `start` or `end` paths.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.start.0.contains(id) || self.caret.0.contains(id)
        }
    }
}
impl AccessClickArgs {
    /// Is context click.
    pub fn is_context(&self) -> bool {
        !self.is_primary
    }
}

event! {
    /// Accessibility info is now required for the window.
    pub static ACCESS_INITED_EVENT: AccessInitedArgs;

    /// Accessibility info is no longer required for the window.
    pub static ACCESS_DEINITED_EVENT: AccessDeinitedArgs;

    /// Run the primary or context click action.
    pub static ACCESS_CLICK_EVENT: AccessClickArgs;

    /// Focus or escape focus on a widget.
    pub static ACCESS_FOCUS_EVENT: AccessFocusArgs;

    /// Sets the focus navigation origin.
    pub static ACCESS_FOCUS_NAV_ORIGIN_EVENT: AccessFocusNavOriginArgs;

    /// Expand or collapse the widget content.
    pub static ACCESS_EXPANDER_EVENT: AccessExpanderArgs;

    /// Increment or decrement the widget value by steps.
    pub static ACCESS_INCREMENT_EVENT: AccessIncrementArgs;

    /// Show or hide the widget's tooltip.
    pub static ACCESS_TOOLTIP_EVENT: AccessToolTipArgs;

    /// Run a scroll command.
    pub static ACCESS_SCROLL_EVENT: AccessScrollArgs;

    /// Replace the text content.
    pub static ACCESS_TEXT_EVENT: AccessTextArgs;

    /// Replace the number value.
    pub static ACCESS_NUMBER_EVENT: AccessNumberArgs;

    /// Select text.
    pub static ACCESS_SELECTION_EVENT: AccessSelectionArgs;
}

/// Accessibility service.
pub struct ACCESS;

impl ACCESS {
    /// Click the widget in the window.
    ///
    /// If `is_primary` is `true` a primary click is generated, if it is `false` a context click is generated.
    pub fn click(&self, widget: WidgetPath, is_primary: bool) {
        ACCESS_CLICK_EVENT.notify(AccessClickArgs::now(widget, is_primary));
    }

    /// Show tooltip for widget in the window, if it has any tooltip.
    ///
    /// The tooltip can auto-hide following the same rules as tooltips shown by hover.
    pub fn show_tooltip(&self, widget: WidgetPath) {
        ACCESS_TOOLTIP_EVENT.notify(AccessToolTipArgs::now(widget, true));
    }

    /// Hide tooltip for the widget in the window, if it has any tooltip showing.
    pub fn hide_tooltip(&self, widget: WidgetPath) {
        ACCESS_TOOLTIP_EVENT.notify(AccessToolTipArgs::now(widget, false));
    }
}
