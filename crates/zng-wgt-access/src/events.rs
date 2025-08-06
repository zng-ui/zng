use zng_app::access::*;
use zng_view_api::access::AccessCmdName;
use zng_wgt::prelude::*;

event_property! {
    /// Access requested a click.
    ///
    /// Note that the normal click event is already triggered by this event.
    pub fn access_click {
        event: ACCESS_CLICK_EVENT,
        args: AccessClickArgs,
        with: access_click,
    }

    /// Access requested to expand or collapse the widget content.
    pub fn access_expander {
        event: ACCESS_EXPANDER_EVENT,
        args: AccessExpanderArgs,
        with: access_expander,
    }

    /// Access requested to increment or decrement the widget value by steps.
    pub fn access_increment {
        event: ACCESS_INCREMENT_EVENT,
        args: AccessIncrementArgs,
        with: access_increment,
    }

    /// Access requested to show or hide the widget's tooltip.
    ///
    /// Note that the tooltip property already handles this event.
    pub fn access_tooltip {
        event: ACCESS_TOOLTIP_EVENT,
        args: AccessToolTipArgs,
        with: access_tooltip,
    }

    /// Access requested a scroll command.
    ///
    /// Note that the scroll widget already handles this event.
    pub fn access_scroll {
        event: ACCESS_SCROLL_EVENT,
        args: AccessScrollArgs,
        with: access_scroll,
    }

    /// Access requested a text input/replace.
    ///
    /// Note that the text widget already handles this event.
    pub fn access_text {
        event: ACCESS_TEXT_EVENT,
        args: AccessTextArgs,
        with: access_text,
    }

    /// Access requested a number input.
    pub fn access_number {
        event: ACCESS_NUMBER_EVENT,
        args: AccessNumberArgs,
        with: access_number,
    }

    /// Access requested a text selection.
    ///
    /// Note that the text widget already handles this event.
    pub fn access_selection {
        event: ACCESS_SELECTION_EVENT,
        args: AccessSelectionArgs,
        with: access_selection,
    }
}

fn access_click(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::Click)
}

fn access_expander(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::SetExpanded)
}

fn access_increment(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::Increment)
}

fn access_tooltip(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::SetToolTipVis)
}

fn access_scroll(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::Scroll)
}

fn access_text(child: impl IntoUiNode, _: bool) -> UiNode {
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            if let Some(mut access) = info.access() {
                access.push_command(AccessCmdName::SetString);
                access.push_command(AccessCmdName::ReplaceSelectedText);
            }
        }
    })
}

fn access_number(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::SetNumber)
}

fn access_selection(child: impl IntoUiNode, _: bool) -> UiNode {
    access_capable(child, AccessCmdName::SelectText)
}

fn access_capable(child: impl IntoUiNode, cmd: AccessCmdName) -> UiNode {
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            if let Some(mut access) = info.access() {
                access.push_command(cmd)
            }
        }
    })
}
