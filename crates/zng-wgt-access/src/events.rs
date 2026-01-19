use zng_app::access::*;
use zng_view_api::access::AccessCmdName;
use zng_wgt::prelude::*;

event_property! {
    /// Access requested a click.
    ///
    /// Note that the normal click event is already triggered by this event.
    pub fn on_access_click<on_pre_access_click>(child: impl IntoUiNode, handler: Handler<AccessClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_CLICK_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::Click)
    }

    /// Access requested to expand or collapse the widget content.
    pub fn on_access_expander<on_pre_access_expander>(child: impl IntoUiNode, handler: Handler<AccessExpanderArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_EXPANDER_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::SetExpanded)
    }

    /// Access requested to increment or decrement the widget value by steps.
    pub fn on_access_increment<on_pre_access_increment>(child: impl IntoUiNode, handler: Handler<AccessIncrementArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_INCREMENT_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::Increment)
    }

    /// Access requested to show or hide the widget's tooltip.
    ///
    /// Note that the tooltip property already handles this event.
    pub fn on_access_tooltip<on_pre_access_tooltip>(child: impl IntoUiNode, handler: Handler<AccessToolTipArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_TOOLTIP_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::SetToolTipVis)
    }

    /// Access requested a scroll command.
    ///
    /// Note that the scroll widget already handles this event.
    pub fn on_access_scroll<on_pre_access_scroll>(child: impl IntoUiNode, handler: Handler<AccessScrollArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_SCROLL_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::Scroll)
    }

    /// Access requested a text input/replace.
    ///
    /// Note that the text widget already handles this event.
    pub fn on_access_text<on_pre_access_text>(child: impl IntoUiNode, handler: Handler<AccessTextArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_TEXT_EVENT).build::<PRE>(child, handler);
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op
                && let Some(mut access) = info.access()
            {
                access.push_command(AccessCmdName::SetString);
                access.push_command(AccessCmdName::ReplaceSelectedText);
            }
        })
    }

    /// Access requested a number input.
    pub fn on_access_number<on_pre_access_number>(child: impl IntoUiNode, handler: Handler<AccessNumberArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_NUMBER_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::SetNumber)
    }

    /// Access requested a text selection.
    ///
    /// Note that the text widget already handles this event.
    pub fn on_access_selection<on_pre_access_selection>(child: impl IntoUiNode, handler: Handler<AccessSelectionArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(ACCESS_SELECTION_EVENT).build::<PRE>(child, handler);
        access_capable(child, AccessCmdName::SelectText)
    }
}

fn access_capable(child: impl IntoUiNode, cmd: AccessCmdName) -> UiNode {
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op
            && let Some(mut access) = info.access()
        {
            access.push_command(cmd)
        }
    })
}
