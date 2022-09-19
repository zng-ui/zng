//! Commands that control the scoped scroll widget.
//!
//! The scroll widget implements all of this commands scoped to its widget ID.
//!
//! [`ScrollToTopCommand`]: crate::widgets::scroll::commands::ScrollToTopCommand
//! [`ScrollToLeftmostCommand`]: crate::widgets::scroll::commands::ScrollToLeftmostCommand

use super::*;
use zero_ui::core::gesture::*;

command! {
    /// Represents the **scroll up** by one [`v_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`v_line_unit`]: fn@super::properties::v_line_unit
    pub static SCROLL_UP_CMD = {
        name: "Scroll Up",
        info: "Scroll Up by one scroll unit.",
        shortcut: shortcut!(Up),
    };

    /// Represents the **scroll down** by one [`v_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`v_line_unit`]: fn@super::properties::v_line_unit
    pub static SCROLL_DOWN_CMD = {
        name: "Scroll Down",
        info: "Scroll Down by one scroll unit.",
        shortcut: shortcut!(Down),
    };

    /// Represents the **scroll left** by one [`h_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub static SCROLL_LEFT_CMD = {
        name: "Scroll Left",
        info: "Scroll Left by one scroll unit.",
        shortcut: shortcut!(Left),
    };

    /// Represents the **scroll right** by one [`h_line_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub static SCROLL_RIGHT_CMD = {
        name: "Scroll Right",
        info: "Scroll Right by one scroll unit.",
        shortcut: shortcut!(Right),
    };


    /// Represents the **page up** by one [`v_page_unit`] action.
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
    pub static PAGE_UP_CMD = {
        name: "Page Up",
        info: "Scroll Up by one page unit.",
        shortcut: shortcut!(PageUp),
    };

    /// Represents the **page down** by one [`v_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`v_page_unit`]: fn@super::properties::v_page_unit
    pub static PAGE_DOWN_CMD = {
        name: "Page Down",
        info: "Scroll down by one page unit.",
        shortcut: shortcut!(PageDown),
    };

    /// Represents the **page left** by one [`h_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`h_line_unit`]: fn@super::properties::h_line_unit
    pub static PAGE_LEFT_CMD = {
        name: "Page Left",
        info: "Scroll Left by one page unit.",
        shortcut: shortcut!(SHIFT+PageUp),
    };

    /// Represents the **page right** by one [`h_page_unit`] action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`bool`] that enables the alternate of the command
    /// or a [`ScrollRequest`] that does the same.
    ///
    /// [`h_page_unit`]: fn@super::properties::h_page_unit
    pub static PAGE_RIGHT_CMD = {
        name: "Page Right",
        info: "Scroll Right by one page unit.",
        shortcut: shortcut!(SHIFT+PageDown),
    };

    /// Represents the **scroll to top** action.
    pub static SCROLL_TO_TOP_CMD = {
        name: "Scroll to Top",
        info: "Scroll up to the content top.",
        shortcut: [shortcut!(Home), shortcut!(CTRL+Home)],
    };

    /// Represents the **scroll to bottom** action.
    pub static SCROLL_TO_BOTTOM_CMD = {
        name: "Scroll to Bottom",
        info: "Scroll down to the content bottom.",
        shortcut: [shortcut!(End), shortcut!(CTRL+End)],
    };

    /// Represents the **scroll to leftmost** action.
    pub static SCROLL_TO_LEFTMOST_CMD = {
        name: "Scroll to Leftmost",
        info: "Scroll left to the content left edge.",
        shortcut: [shortcut!(SHIFT+Home), shortcut!(CTRL|SHIFT+Home)],
    };

    /// Represents the **scroll to rightmost** action.
    pub static SCROLL_TO_RIGHTMOST_CMD = {
        name: "Scroll to Righmost",
        info: "Scroll right to the content right edge.",
        shortcut: [shortcut!(SHIFT+End), shortcut!(CTRL|SHIFT+End)],
    };

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
    pub static SCROLL_TO_CMD;
}

/// Parameters for the scroll and page commands.
#[derive(Debug, Clone)]
pub struct ScrollRequest {
    /// If the [alt factor] should be applied to the base scroll unit when scrolling.
    ///
    /// [alt factor]: super::properties::ALT_FACTOR_VAR
    pub alternate: bool,
}
impl ScrollRequest {
    /// Pack the request into a command parameter.
    pub fn to_param(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_param(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            p.downcast_ref::<bool>().map(|&alt| ScrollRequest { alternate: alt })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::param`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event.
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.param {
            if args.propagation().is_stopped() {
                None
            } else {
                Self::from_param(p)
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

/// Parameters for the [`SCROLL_TO_CMD`].
#[derive(Debug, Clone)]
pub struct ScrollToRequest {
    /// Widget that will be scrolled into view.
    pub widget_id: WidgetId,

    /// How much the scroll position will change to showcase the target widget.
    pub mode: ScrollToMode,
}
impl ScrollToRequest {
    /// Pack the request into a command parameter.
    pub fn to_param(self) -> CommandParam {
        CommandParam::new(self)
    }

    /// Extract a clone of the request from the command parameter if it is of a compatible type.
    pub fn from_param(p: &CommandParam) -> Option<Self> {
        if let Some(req) = p.downcast_ref::<Self>() {
            Some(req.clone())
        } else {
            p.downcast_ref::<WidgetId>().map(|id| ScrollToRequest {
                widget_id: *id,
                mode: ScrollToMode::default(),
            })
        }
    }

    /// Extract a clone of the request from [`CommandArgs::param`] if it is set to a compatible type and
    /// stop-propagation was not requested for the event and the command was enabled when it was send.
    pub fn from_args(args: &CommandArgs) -> Option<Self> {
        if let Some(p) = &args.param {
            if !args.enabled || args.propagation().is_stopped() {
                None
            } else {
                Self::from_param(p)
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

/// Defines how much the [`SCROLL_TO_CMD`] will scroll to showcase the target widget.
#[derive(Debug, Clone)]
pub enum ScrollToMode {
    /// Scroll will change only just enough so that the widget inner rect is fully visible with the optional
    /// extra margin offsets.
    Minimal {
        /// Extra margin added so that the widget is touching the scroll edge.
        margin: SideOffsets,
    },
    /// Scroll so that the point relative to the widget inner rectangle is at the same screen point on
    /// the scroll viewport.
    Center {
        /// A point relative to the target widget inner size.
        widget_point: Point,
        /// A point relative to the scroll viewport.
        scroll_point: Point,
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
    pub fn center_points(widget_point: impl Into<Point>, scroll_point: impl Into<Point>) -> Self {
        ScrollToMode::Center {
            widget_point: widget_point.into(),
            scroll_point: scroll_point.into(),
        }
    }

    /// New [`Center`] mode using the center points of widget and scroll.
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

/// Scroll the scroll widget so that the child widget is fully visible.
///
/// This function is a helper for firing a [`SCROLL_TO_CMD`].
pub fn scroll_to<Evs: WithEvents>(events: &mut Evs, scroll_id: WidgetId, child_id: WidgetId, mode: impl Into<ScrollToMode>) {
    SCROLL_TO_CMD.scoped(scroll_id).notify_param(
        events,
        ScrollToRequest {
            widget_id: child_id,
            mode: mode.into(),
        },
    );
}
