//! Commands that control the scoped scrollable widget.
//!
//! The scrollable widget implements all of this commands scoped to its widget ID.
//!
//! # Duplicate Shortcuts
//!
//! Some commands like [`ScrollToTopCommand`] and [`ScrollToLeftmostCommand`] have duplicate shortcuts,
//! with the command operating on the horizontal axis having and extra alternate shortcut. Command
//! implementers must handle the vertical axis commands first and then handle the
//! horizontal axis commands, this way if the content only scrolls on the horizontal axis the primary
//! shortcuts still work, but if the content scrolls in both axis the primary shortcuts operate the
//! vertical scrolling and the alternate shortcuts operate the horizontal scrolling.
//!
//! [`ScrollToTopCommand`]: crate::widgets::scrollable::commands::ScrollToTopCommand
//! [`ScrollToLeftmostCommand`]: crate::widgets::scrollable::commands::ScrollToLeftmostCommand

use super::*;
use zero_ui::core::gesture::*;

command! {
    /// Represents the scrollable **scroll up** by one [`v_line_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Scroll Up"                                            |
    /// | [`info`]     | "Scroll the focused scrollable UP by one scroll unit." |
    /// | [`shortcut`] | `Up`                                                   |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`v_line_unit`]: fn@super::properties::v_line_unit
    pub ScrollUpCommand
        .init_name("Scroll Up")
        .init_info("Scroll the focused scrollable UP by one scroll unit.")
        .init_shortcut([shortcut!(Up)]);

    /// Represents the scrollable **scroll down** by one [`v_line_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                    |
    /// |--------------|----------------------------------------------------------|
    /// | [`name`]     | "Scroll Down"                                            |
    /// | [`info`]     | "Scroll the focused scrollable DOWN by one scroll unit." |
    /// | [`shortcut`] | `Down`                                                   |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`v_line_unit`]: fn@super::properties::v_line_unit
    pub ScrollDownCommand
        .init_name("Scroll Down")
        .init_info("Scroll the focused scrollable DOWN by one scroll unit.")
        .init_shortcut([shortcut!(Down)]);

    /// Represents the scrollable **scroll left** by one [`h_line_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                    |
    /// |--------------|----------------------------------------------------------|
    /// | [`name`]     | "Scroll Left"                                            |
    /// | [`info`]     | "Scroll the focused scrollable LEFT by one scroll unit." |
    /// | [`shortcut`] | `Left`                                                   |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub ScrollLeftCommand
        .init_name("Scroll Left")
        .init_info("Scroll the focused scrollable LEFT by one scroll unit.")
        .init_shortcut([shortcut!(Left)]);

    /// Represents the scrollable **scroll right** by one [`h_line_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                     |
    /// |--------------|-----------------------------------------------------------|
    /// | [`name`]     | "Scroll Right"                                            |
    /// | [`info`]     | "Scroll the focused scrollable RIGHT by one scroll unit." |
    /// | [`shortcut`] | `Down`                                                    |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub ScrollRightCommand
        .init_name("Scroll Right")
        .init_info("Scroll the focused scrollable RIGHT by one scroll unit.")
        .init_shortcut([shortcut!(Right)]);


    /// Represents the scrollable **page up** by one [`v_page_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Page Up"                                             |
    /// | [`info`]     | "Scroll the focused scrollable UP by one page unit."  |
    /// | [`shortcut`] | `PageUp`                                              |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`v_page_unit`]: fn@super::properties::v_page_unit
    pub PageUpCommand
        .init_name("Page Up")
        .init_info("Scroll the focused scrollable UP by one page unit.")
        .init_shortcut([shortcut!(PageUp)]);

    /// Represents the scrollable **page down** by one [`v_page_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                   |
    /// |--------------|---------------------------------------------------------|
    /// | [`name`]     | "Page Down"                                             |
    /// | [`info`]     | "Scroll the focused scrollable DOWN by one page unit."  |
    /// | [`shortcut`] | `PageDown`                                              |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`v_page_unit`]: fn@super::properties::v_page_unit
    pub PageDownCommand
        .init_name("Page Down")
        .init_info("Scroll the focused scrollable DOWN by one page unit.")
        .init_shortcut([shortcut!(PageDown)]);

    /// Represents the scrollable **page left** by one [`h_page_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Page Left"                                            |
    /// | [`info`]     | "Scroll the focused scrollable LEFT by one page unit." |
    /// | [`shortcut`] | `PageUp`, `SHIFT+PageUp`                               |
    ///
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub PageLeftCommand
        .init_name("Page Left")
        .init_info("Scroll the focused scrollable LEFT by one page unit.")
        .init_shortcut([shortcut!(PageUp), shortcut!(SHIFT+PageUp)]);

    /// Represents the scrollable **page right** by one [`h_page_unit`] action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                   |
    /// |--------------|---------------------------------------------------------|
    /// | [`name`]     | "Page Right"                                            |
    /// | [`info`]     | "Scroll the focused scrollable RIGHT by one page unit." |
    /// | [`shortcut`] | `PageDown`, `SHIFT+PageDown`                            |
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    /// [`h_page_unit`]: fn@super::properties::h_page_unit
    pub PageRightCommand
        .init_name("Page Right")
        .init_info("Scroll the focused scrollable RIGHT by one page unit.")
        .init_shortcut([shortcut!(PageDown), shortcut!(SHIFT+PageDown)]);

    /// Represents the scrollable **scroll to top** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Scroll to Top"                                        |
    /// | [`info`]     | "Scroll up to the content top."                        |
    /// | [`shortcut`] | `CTRL+Home`                                            |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub ScrollToTopCommand
        .init_name("Scroll to Top")
        .init_info("Scroll up to the content top.")
        .init_shortcut([shortcut!(CTRL+Home)]);

    /// Represents the scrollable **scroll to bottom** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Scroll to Bottom"                                     |
    /// | [`info`]     | "Scroll down to the content bottom."                   |
    /// | [`shortcut`] | `CTRL+End`                                             |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub ScrollToBottomCommand
        .init_name("Scroll to Bottom")
        .init_info("Scroll down to the content bottom.")
        .init_shortcut([shortcut!(CTRL+End)]);

    /// Represents the scrollable **scroll to leftmost** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Scroll to Leftmost"                                   |
    /// | [`info`]     | "Scroll left to the content left edge."                |
    /// | [`shortcut`] | `CTRL+Home`, <code>CTRL&#124;SHIFT+Home</code>         |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub ScrollToLeftmostCommand
        .init_name("Scroll to Leftmost")
        .init_info("Scroll left to the content left edge.")
        .init_shortcut([shortcut!(CTRL+Home), shortcut!(CTRL|SHIFT+Home)]);

    /// Represents the scrollable **scroll to rightmost** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                  |
    /// |--------------|--------------------------------------------------------|
    /// | [`name`]     | "Scroll to Rightmost"                                   |
    /// | [`info`]     | "Scroll right to the content right edge."              |
    /// | [`shortcut`] | `CTRL+End`, <code>CTRL&#124;SHIFT+End</code>           |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub ScrollToRightmostCommand
        .init_name("Scroll to Righmost")
        .init_info("Scroll right to the content right edge.")
        .init_shortcut([shortcut!(CTRL+End), shortcut!(CTRL|SHIFT+End)]);

    /// Represents the action of scrolling until a child widget is fully visible.
    ///
    /// # Metadata
    ///
    /// This command initializes with no extra metadata.
    ///
    /// # Parameter
    ///
    /// This command requires a parameter to work, it can be the [`WidgetId`] of a child widget or
    /// a [`ScrollToRequest`] instance.
    ///
    /// You can use the [`scroll_to`] function to invoke this command.
    pub ScrollToCommand;
}

macro_rules! impl_notify_alt {
    ($($Cmd:ident,)+) => {$(
        impl $Cmd {
            /// Notify the alternate mode of the command.
            pub fn notify_alt<Evs: WithEvents>(self, events: &mut Evs, alternate: bool) -> bool {
                self.notify(events, if alternate {
                    Some(ScrollRequest {
                        alternate,
                    }.to_parameter())
                } else {
                    None
                })
            }
        }
    )+}
}
impl_notify_alt! {
    ScrollLeftCommand,
    ScrollRightCommand,
    ScrollUpCommand,
    ScrollDownCommand,

    PageLeftCommand,
    PageRightCommand,
    PageUpCommand,
    PageDownCommand,
}

/// Parameters for the scroll and page commands.
#[derive(Debug, Clone)]
pub struct ScrollRequest {
    /// If the [alt factor] should be applied to the base scroll unit when scrolling.
    ///
    /// [alt factor]: super::properties::AltFactorVar
    pub alternate: bool,
}
impl ScrollRequest {
    /// Pack the request into a command parameter.
    pub fn to_parameter(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_parameter(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            p.downcast_ref::<bool>().map(|&alt| ScrollRequest { alternate: alt })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::parameter`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event.
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.parameter {
            if args.stop_propagation_requested() {
                None
            } else {
                Self::from_parameter(p)
            }
        } else {
            None
        }
    }
}
impl_from_and_into_var! {
    fn from(alternate: bool) -> ScrollRequest {
        ScrollRequest {
            alternate,
        }
    }
}

/// Parameters for the [`ScrollToCommand`].
#[derive(Debug, Clone)]
pub struct ScrollToRequest {
    /// Widget that will be scrolled into view.
    pub widget_id: WidgetId,

    /// How much the scroll position will change to showcase the target widget.
    pub mode: ScrollToMode,
}
impl ScrollToRequest {
    /// Pack the request into a command parameter.
    pub fn to_parameter(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_parameter(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            p.downcast_ref::<WidgetId>().map(|id| ScrollToRequest {
                widget_id: *id,
                mode: ScrollToMode::default(),
            })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::parameter`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event.
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.parameter {
            if args.stop_propagation_requested() {
                None
            } else {
                Self::from_parameter(p)
            }
        } else {
            None
        }
    }
}
impl_from_and_into_var! {
    fn from(widget_id: WidgetId) -> ScrollToRequest {
        ScrollToRequest {
            widget_id,
            mode: ScrollToMode::default()
        }
    }
}

/// Defines how much the [`ScrollToCommand`] will scroll to showcase the target widget.
#[derive(Debug, Clone)]
pub enum ScrollToMode {
    /// Scroll will change only just enough so that the widget inner rect is fully visible with the optional
    /// extra margin offsets.
    Minimal {
        /// Extra margin added so that the widget is touching the scrollable edge.
        margin: SideOffsets,
    },
    /// Scroll so that the point relative to the widget inner rectangle is at the same screen point on
    /// the scrollable viewport.
    Center {
        /// A point relative to the target widget inner size.
        widget_point: Point,
        /// A point relative to the scrollable viewport.
        scrollable_point: Point,
    },
}
impl ScrollToMode {
    /// New [`Minimal`] mode.
    ///
    /// [`Minimal`]: Self::Minimal
    pub fn minimal(margin: impl Into<SideOffsets>) -> Self {
        ScrollToMode::Minimal { margin: margin.into() }
    }

    /// New [`Center`] mode.
    ///
    /// [`Center`]: Self::Center
    pub fn center_points(widget_point: impl Into<Point>, scrollable_point: impl Into<Point>) -> Self {
        ScrollToMode::Center {
            widget_point: widget_point.into(),
            scrollable_point: scrollable_point.into(),
        }
    }

    /// New [`Center`] mode using the center points of widget and scrollable.
    ///
    /// [`Center`]: Self::Center
    pub fn center() -> Self {
        Self::center_points(Point::center(), Point::center())
    }
}
impl Default for ScrollToMode {
    /// Minimal with margin 10.
    fn default() -> Self {
        Self::minimal(10)
    }
}

/// Scroll the scrollable widget so that the child widget is fully visible.
///
/// This function is a helper for firing a [`ScrollToCommand`].
pub fn scroll_to<Evs: WithEvents>(events: &mut Evs, scrollable_id: WidgetId, child_id: WidgetId, mode: impl Into<ScrollToMode>) {
    ScrollToCommand.scoped(scrollable_id).notify(
        events,
        Some(
            ScrollToRequest {
                widget_id: child_id,
                mode: mode.into(),
            }
            .to_parameter(),
        ),
    );
}
