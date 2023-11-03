//! Accessibility events.
//!
//! Most of [access events] are handled directly by widgets or are merged into other events,
//! the event properties only covers some events that may be used directly.

use crate::prelude::new_property::*;

use super::event_property;
use crate::core::access::*;

pub(super) fn access_click(child: impl UiNode, _: bool) -> impl UiNode {
    match_node(child, |_, op| {
        if let UiNodeOp::Info { info } = op {
            if let Some(mut access) = info.access() {
                access.push_command(crate::core::widget_info::access::AccessCmdName::Click)
            }
        }
    })
}

event_property! {
    /// Access requested a click.
    ///
    /// Note that the normal click event is already triggered by this event.
    pub fn access_click {
        event: ACCESS_CLICK_EVENT,
        args: AccessClickArgs,
        with: access_click,
    }

    /// Access requested expand or collapse the widget content.
    pub fn access_expander {
        event: ACCESS_EXPANDER_EVENT,
        args: AccessExpanderArgs,
    }

    /// Access requested increment or decrement the widget value by steps.
    pub fn access_increment {
        event: ACCESS_INCREMENT_EVENT,
        args: AccessIncrementArgs,
    }

    /// Access requested show or hide the widget's tooltip.
    ///
    /// Note that the tooltip property already handles this event.
    pub fn access_tooltip {
        event: ACCESS_TOOLTIP_EVENT,
        args: AccessToolTipArgs,
    }

    /// Access requested a scroll command.
    ///
    /// Note that the scroll widget already handles this event.
    pub fn access_scroll {
        event: ACCESS_SCROLL_EVENT,
        args: AccessScrollArgs,
    }

    /// Access requested a text input/replace.
    ///
    /// Note that the text widget already handles this event.
    pub fn access_text {
        event: ACCESS_TEXT_EVENT,
        args: AccessTextArgs,
    }

    /// Access requested a number input.
    pub fn access_number {
        event: ACCESS_NUMBER_EVENT,
        args: AccessNumberArgs,
    }

    /// Access requested a text selection.
    ///
    /// Note that the text widget already handles this event.
    pub fn access_selection {
        event: ACCESS_SELECTION_EVENT,
        args: AccessSelectionArgs,
    }
}
