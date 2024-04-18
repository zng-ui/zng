//! Accessibility/automation events.

use zng_txt::Txt;
use zng_view_api::access::AccessCmd;

pub use zng_view_api::access::ScrollCmd;

use crate::{
    event::{event, event_args},
    update::EventUpdate,
    widget::WidgetId,
    window::WindowId,
};

pub(super) fn on_access_init(window_id: WindowId) -> EventUpdate {
    let args = AccessInitedArgs::now(window_id);
    ACCESS_INITED_EVENT.new_update(args)
}

pub(super) fn on_access_deinit(window_id: WindowId) -> EventUpdate {
    let args = AccessDeinitedArgs::now(window_id);
    ACCESS_DEINITED_EVENT.new_update(args)
}

pub(super) fn on_access_command(window_id: WindowId, widget_id: WidgetId, command: AccessCmd) -> Option<EventUpdate> {
    match command {
        AccessCmd::Click(primary) => {
            let args = AccessClickArgs::now(window_id, widget_id, primary);
            Some(ACCESS_CLICK_EVENT.new_update(args))
        }
        AccessCmd::Focus(focus) => {
            let args = AccessFocusArgs::now(window_id, widget_id, focus);
            Some(ACCESS_FOCUS_EVENT.new_update(args))
        }
        AccessCmd::FocusNavOrigin => {
            let args = AccessFocusNavOriginArgs::now(window_id, widget_id);
            Some(ACCESS_FOCUS_NAV_ORIGIN_EVENT.new_update(args))
        }
        AccessCmd::SetExpanded(expanded) => {
            let args = AccessExpanderArgs::now(window_id, widget_id, expanded);
            Some(ACCESS_EXPANDER_EVENT.new_update(args))
        }
        AccessCmd::Increment(inc) => {
            let args = AccessIncrementArgs::now(window_id, widget_id, inc);
            Some(ACCESS_INCREMENT_EVENT.new_update(args))
        }
        AccessCmd::SetToolTipVis(vis) => {
            let args = AccessToolTipArgs::now(window_id, widget_id, vis);
            Some(ACCESS_TOOLTIP_EVENT.new_update(args))
        }
        AccessCmd::ReplaceSelectedText(s) => {
            let args = AccessTextArgs::now(window_id, widget_id, s, true);
            Some(ACCESS_TEXT_EVENT.new_update(args))
        }
        AccessCmd::Scroll(s) => {
            let args = AccessScrollArgs::now(window_id, widget_id, s);
            Some(ACCESS_SCROLL_EVENT.new_update(args))
        }
        AccessCmd::SelectText {
            start: (start_wgt, start_idx),
            caret: (caret_wgt, caret_idx),
        } => {
            let start_wgt = WidgetId::from_raw(start_wgt.0);
            let caret_wgt = WidgetId::from_raw(caret_wgt.0);
            let args = AccessSelectionArgs::now(window_id, (start_wgt, start_idx), (caret_wgt, caret_idx));
            Some(ACCESS_SELECTION_EVENT.new_update(args))
        }
        AccessCmd::SetString(s) => {
            let args = AccessTextArgs::now(window_id, widget_id, s, false);
            Some(ACCESS_TEXT_EVENT.new_update(args))
        }
        AccessCmd::SetNumber(n) => {
            let args = AccessNumberArgs::now(window_id, widget_id, n);
            Some(ACCESS_NUMBER_EVENT.new_update(args))
        }
        a => {
            tracing::warn!("access command `{a:?}` not implemented");
            None
        }
    }
}

event_args! {
    /// Arguments for the [`ACCESS_INITED_EVENT`].
    pub struct AccessInitedArgs {
        /// Target window.
        pub window_id: WindowId,

        ..

        /// Event is broadcast.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

        /// Arguments for the [`ACCESS_DEINITED_EVENT`].
        pub struct AccessDeinitedArgs {
            /// Target window.
            pub window_id: WindowId,

            ..

            /// Event is broadcast.
            fn delivery_list(&self, list: &mut UpdateDeliveryList) {
                list.search_all()
            }
        }

    /// Arguments for the [`ACCESS_CLICK_EVENT`].
    pub struct AccessClickArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// Is primary click (default action).
        ///
        /// If `false` is context click.
        pub is_primary: bool,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id);
        }
    }

    /// Arguments for the [`ACCESS_FOCUS_EVENT`].
    pub struct AccessFocusArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// If the widget must be focused.
        ///
        /// If `true` the widget is focused, if `false` and the widget is focused, does ESC.
        pub focus: bool,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_FOCUS_NAV_ORIGIN_EVENT`].
    pub struct AccessFocusNavOriginArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_EXPANDER_EVENT`].
    pub struct AccessExpanderArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// New expanded value.
        pub expanded: bool,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_INCREMENT_EVENT`].
    pub struct AccessIncrementArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// Increment steps.
        ///
        /// Usually is -1 or 1.
        pub delta: i32,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_TOOLTIP_EVENT`].
    pub struct AccessToolTipArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// New tooltip visibility.
        pub visible: bool,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_SCROLL_EVENT`].
    pub struct AccessScrollArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// Scroll command.
        pub command: ScrollCmd,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_TEXT_EVENT`].
    pub struct AccessTextArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// Replacement text.
        pub txt: Txt,

        /// If only the selected text is replaced.
        ///
        /// Note that if the selection is empty the text is just inserted at the caret position, or is appended if there
        /// is no caret.
        pub selection_only: bool,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_NUMBER_EVENT`].
    pub struct AccessNumberArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Target widget.
        pub widget_id: WidgetId,

        /// Replacement number.
        pub num: f64,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_SELECTION_EVENT`].
    pub struct AccessSelectionArgs {
        /// Target window.
        pub window_id: WindowId,

        /// Selection start.
        ///
        /// Text widget and character index where the selection *starts*.
        pub start: (WidgetId, usize),
        /// Selection end.
        ///
        /// This is where the caret is placed, it does not need to be greater than the start.
        pub caret: (WidgetId, usize),

        ..

        /// Target both widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.start.0);
            list.search_widget(self.caret.0);
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
    pub fn click(&self, window_id: impl Into<WindowId>, widget_id: impl Into<WidgetId>, is_primary: bool) {
        let win = window_id.into();
        let wgt = widget_id.into();
        ACCESS_CLICK_EVENT.notify(AccessClickArgs::now(win, wgt, is_primary));
    }

    /// Show tooltip for widget in the window, if it has any tooltip.
    ///
    /// The tooltip can auto-hide following the same rules as tooltips shown by hover.
    pub fn show_tooltip(&self, window_id: impl Into<WindowId>, widget_id: impl Into<WidgetId>) {
        let win = window_id.into();
        let wgt = widget_id.into();
        ACCESS_TOOLTIP_EVENT.notify(AccessToolTipArgs::now(win, wgt, true));
    }

    /// Hide tooltip for the widget in the window, if it has any tooltip showing.
    pub fn hide_tooltip(&self, window_id: impl Into<WindowId>, widget_id: impl Into<WidgetId>) {
        let win = window_id.into();
        let wgt = widget_id.into();
        ACCESS_TOOLTIP_EVENT.notify(AccessToolTipArgs::now(win, wgt, false));
    }
}
