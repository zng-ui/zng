//! Accessibility/automation events.

use zero_ui_view_api::access::AccessCommand;

pub use zero_ui_view_api::access::ScrollCommand;

use crate::{
    event::{event, event_args, EventUpdate},
    text::Txt,
    widget_instance::WidgetId,
    window::WindowId,
};

pub(super) fn on_access_init(window_id: WindowId) -> EventUpdate {
    let args = AccessInitedArgs::now(window_id);
    ACCESS_INITED_EVENT.new_update(args)
}

pub(super) fn on_access_command(window_id: WindowId, widget_id: WidgetId, command: AccessCommand) -> Option<EventUpdate> {
    match command {
        AccessCommand::Click(primary) => {
            let args = AccessClickArgs::now(window_id, widget_id, primary);
            Some(ACCESS_CLICK_EVENT.new_update(args))
        }
        AccessCommand::Focus(focus) => {
            let args = AccessFocusArgs::now(window_id, widget_id, focus);
            Some(ACCESS_FOCUS_EVENT.new_update(args))
        }
        AccessCommand::SetNextTabStart => {
            // TODO
            None
        }
        AccessCommand::SetExpanded(expanded) => {
            let args = AccessExpanderArgs::now(window_id, widget_id, expanded);
            Some(ACCESS_EXPANDER_EVENT.new_update(args))
        }
        AccessCommand::Increment(inc) => {
            let args = AccessIncrementArgs::now(window_id, widget_id, inc);
            Some(ACCESS_INCREMENT_EVENT.new_update(args))
        }
        AccessCommand::SetToolTipVis(vis) => {
            let args = AccessToolTipArgs::now(window_id, widget_id, vis);
            Some(ACCESS_TOOLTIP_EVENT.new_update(args))
        }
        AccessCommand::ReplaceSelectedText(s) => {
            let args = AccessTextArgs::now(window_id, widget_id, s, true);
            Some(ACCESS_TEXT_EVENT.new_update(args))
        }
        AccessCommand::Scroll(s) => {
            let args = AccessScrollArgs::now(window_id, widget_id, s);
            Some(ACCESS_SCROLL_EVENT.new_update(args))
        }
        AccessCommand::SelectText {
            start: (start_wgt, start_idx),
            caret: (caret_wgt, caret_idx),
        } => {
            let start_wgt = WidgetId::from_raw(start_wgt.0);
            let caret_wgt = WidgetId::from_raw(caret_wgt.0);
            let args = AccessSelectionArgs::now(window_id, (start_wgt, start_idx), (caret_wgt, caret_idx));
            Some(ACCESS_SELECTION_EVENT.new_update(args))
        }
        AccessCommand::SetString(s) => {
            let args = AccessTextArgs::now(window_id, widget_id, s, false);
            Some(ACCESS_TEXT_EVENT.new_update(args))
        }
        AccessCommand::SetNumber(n) => {
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
    /// Arguments for the [`ACCESS_INIT_EVENT`].
    pub struct AccessInitedArgs {
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
        pub delta: i8,

        ..

        /// Target the widget.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widget(self.widget_id)
        }
    }

    /// Arguments for the [`ACCESS_TOOLTIP_EVENT`].
    pub struct AccessToolTipArgs  {
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
        pub command: ScrollCommand,

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
    /// Accessibility info was requested for the first time in a window.
    pub static ACCESS_INITED_EVENT: AccessInitedArgs;

    /// Run the primary or context click action.
    pub static ACCESS_CLICK_EVENT: AccessClickArgs;

    /// Focus or escape focus on a widget.
    pub static ACCESS_FOCUS_EVENT: AccessFocusArgs;

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

    /// Replace the number content and reset the selection.
    pub static ACCESS_NUMBER_EVENT: AccessNumberArgs;

    /// Select text.
    pub static ACCESS_SELECTION_EVENT: AccessSelectionArgs;
}
