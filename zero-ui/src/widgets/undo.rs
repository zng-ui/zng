//! Undo scope mix and undo history widget.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::prelude::new_widget::*;

use crate::core::gesture::ClickArgs;
use crate::core::undo::{UndoInfo, REDO_CMD, UNDO, UNDO_CMD};
use crate::widgets::{Button, Text};

/// Undo scope widget mixin.
///
/// Widget is an undo/redo scope, it tracks changes and handles undo/redo commands.
///
/// You can force the widget to use a parent undo scope by setting [`undo_scope`] to `false`, this will cause the widget
/// to start registering undo/redo actions in the parent, note that the widget will continue behaving as if it
/// owns the scope, so it may clear it.
///
/// [`undo_scope`]: crate::properties::undo_scope
#[widget_mixin]
pub struct UndoMix<P>(P);

impl<P: WidgetImpl> UndoMix<P> {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            crate::properties::undo_scope = true;
        }
    }

    widget_impl! {
        /// If the widget can register undo actions.
        ///
        /// Is `true` by default in this widget, if set to `false` disables undo in the widget.
        pub crate::properties::undo_enabled(enabled: impl IntoVar<bool>);

        /// Sets the maximum number of undo/redo actions that are retained in the widget.
        pub crate::properties::undo_limit(limit: impl IntoVar<u32>);

        /// Sets the time interval that undo and redo cover each call for undo handlers in the widget and descendants.
        ///
        /// When undo is requested inside the context all actions after the latest that are within `interval` of the
        /// previous are undone.
        pub crate::properties::undo_interval(interval: impl IntoVar<Duration>);
    }
}

/// Undo/redo stack view.
#[widget($crate::widgets::undo::UndoHistory)]
pub struct UndoHistory(WidgetBase);

context_var! {
    /// Widget function for a single undo entry.
    pub static UNDO_ENTRY_FN_VAR: WidgetFn<UndoEntryArgs> = wgt_fn!(|args: UndoEntryArgs| default_undo_fn(args));
    /// Widget function for a single redo entry.
    pub static REDO_ENTRY_FN_VAR: WidgetFn<RedoEntryArgs> = wgt_fn!(|args: RedoEntryArgs| default_redo_fn(args));
}

/// Default [`UNDO_ENTRY_FN_VAR`].
///
/// Returns a `Button!` with the [`UndoRedoButtonStyle!`] and the entry displayed in a `Text!` child.
/// The button notifies [`UNDO_CMD`] with the entry timestamp, the command is scoped on the
/// undo parent of the caller not of the button.
/// 
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
pub fn default_undo_fn(args: UndoEntryArgs) -> impl UiNode {
    let mut cmd = UNDO_CMD;
    if let Some(w) = UNDO.scope() {
        cmd = cmd.scoped(w);
    }
    let ts = args.timestamp;
    Button! {
        child = Text!(args.info.description());
        // TODO style, is_sibling_below_hovered
        on_click = hn!(|args: &ClickArgs| {
            args.propagation().stop();
            cmd.notify_param(ts);
        });
    }
}

/// Default [`REDO_ENTRY_FN_VAR`].
///
/// Returns a `Button!` with the [`UndoRedoButtonStyle!`] and the entry displayed in a `Text!` child.
/// The button notifies [`REDO_CMD`] with the entry timestamp, the command is scoped on the
/// undo parent of the caller not of the button.
/// 
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
pub fn default_redo_fn(args: RedoEntryArgs) -> impl UiNode {
    let mut cmd = REDO_CMD;
    if let Some(w) = UNDO.scope() {
        cmd = cmd.scoped(w);
    }
    let ts = args.timestamp;
    Button! {
        child = Text!(args.info.description());
        // TODO style, is_sibling_below_hovered
        on_click = hn!(|args: &ClickArgs| {
            args.propagation().stop();
            cmd.notify_param(ts);
        });
    }
}

/// Represents an action in the undo stack.
pub struct UndoEntryArgs {
    /// Moment the first undo action was registered.
    pub timestamp: Instant,
    /// Info about the action.
    pub info: Arc<dyn UndoInfo>,
}

/// Represents an action in the redo stack.
pub struct RedoEntryArgs {
    /// Moment the first undo action was registered.
    pub timestamp: Instant,
    /// Info about the action.
    pub info: Arc<dyn UndoInfo>,
}

/// Menu entry in a 
#[widget($crate::widgets::undo::UndoRedoButtonStyle)]
pub struct UndoRedoButtonStyle(crate::widgets::button::DefaultStyle);
impl UndoRedoButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! { 
            self;
            corner_radius = 0;
            padding = (4, 6);
            border = unset!;
        }
    }
}
