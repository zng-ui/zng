//! Undo scope mix and undo history widget.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::prelude::new_widget::*;

use crate::core::gesture::ClickArgs;
use crate::core::undo::{UndoInfo, UndoOp, REDO_CMD, UNDO, UNDO_CMD};
use crate::widgets::{
    layouts::{stack::StackDirection, Stack},
    view, Button, Text,
};

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
impl UndoHistory {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            wgt.set_child(view::presenter(UndoPanelArgs {}, UNDO_PANEL_FN_VAR));
        });
    }
}

context_var! {
    /// Widget function for a single undo or redo entry.
    pub static UNDO_ENTRY_FN_VAR: WidgetFn<UndoEntryArgs> = WidgetFn::new(default_undo_entry_fn);

    /// Widget function for an undo or redo stack.
    pub static UNDO_STACK_FN_VAR: WidgetFn<UndoStackArgs> = WidgetFn::new(default_undo_stack_fn);

    /// Widget function for the [`UndoHistory!`] child.
    ///
    /// [`UndoHistory!`]: struct@UndoHistory
    pub static UNDO_PANEL_FN_VAR: WidgetFn<UndoPanelArgs> = WidgetFn::new(default_undo_panel_fn);
}

/// Widget function that converts [`UndoEntryArgs`] to widgets.
///
/// Sets the [`UNDO_ENTRY_FN_VAR`].
#[property(CONTEXT+1, default(UNDO_ENTRY_FN_VAR), widget_impl(UndoHistory))]
pub fn undo_entry_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<UndoEntryArgs>>) -> impl UiNode {
    with_context_var(child, UNDO_ENTRY_FN_VAR, wgt_fn)
}

/// Widget function that converts [`UndoStackArgs`] to widgets.
///
/// Sets the [`UNDO_STACK_FN_VAR`].
#[property(CONTEXT+1, default(UNDO_STACK_FN_VAR), widget_impl(UndoHistory))]
pub fn undo_stack_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<UndoStackArgs>>) -> impl UiNode {
    with_context_var(child, UNDO_STACK_FN_VAR, wgt_fn)
}

/// Widget function that converts [`UndoPanelArgs`] to widgets.
///
/// Sets the [`UNDO_PANEL_FN_VAR`].
#[property(CONTEXT+1, default(UNDO_PANEL_FN_VAR), widget_impl(UndoHistory))]
pub fn undo_panel_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<UndoPanelArgs>>) -> impl UiNode {
    with_context_var(child, UNDO_PANEL_FN_VAR, wgt_fn)
}

/// Default [`UNDO_ENTRY_FN_VAR`].
///
/// Returns a `Button!` with the [`UndoRedoButtonStyle!`] and the entry displayed in a `Text!` child.
/// The button notifies [`UNDO_CMD`] or [`REDO_CMD`] with the entry timestamp, the command is scoped on the
/// undo parent of the caller not of the button.
///
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
pub fn default_undo_entry_fn(args: UndoEntryArgs) -> impl UiNode {
    let mut cmd = match args.op {
        UndoOp::Undo => UNDO_CMD,
        UndoOp::Redo => REDO_CMD,
    };
    if let Some(w) = UNDO.scope() {
        cmd = cmd.scoped(w);
    }
    let ts = args.timestamp;
    Button! {
        child = Text!(args.info.description());
        style_fn = UndoRedoButtonStyle!();
        on_click = hn!(|args: &ClickArgs| {
            args.propagation().stop();
            cmd.notify_param(ts);
        });
    }
}

/// Default [`UNDO_STACK_FN_VAR`].
///
/// Returns top-to-bottom `Stack!` of [`UNDO_ENTRY_FN_VAR`], latest first.
///
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
pub fn default_undo_stack_fn(args: UndoStackArgs) -> impl UiNode {
    let entry = UNDO_ENTRY_FN_VAR.get();
    let children = args
        .stack
        .into_iter()
        .rev()
        .map(|(ts, info)| {
            entry(UndoEntryArgs {
                timestamp: ts,
                info,
                op: args.op,
            })
        })
        .collect::<UiNodeVec>();

    Stack! {
        direction = StackDirection::top_to_bottom();
        children;
    }
}

/// Default [`UNDO_PANEL_FN_VAR`].
pub fn default_undo_panel_fn(_: UndoPanelArgs) -> impl UiNode {
    let stack = UNDO_STACK_FN_VAR.get();
    stack(UndoStackArgs {
        stack: UNDO.undo_stack(),
        op: UndoOp::Undo,
    })
}

/// Represents an action in the undo or redo stack.
#[derive(Clone)]
pub struct UndoEntryArgs {
    /// Moment the undo action was first registered.
    ///
    /// This does not change after redo and undo, it is always the register time.
    pub timestamp: Instant,
    /// Info about the action.
    pub info: Arc<dyn UndoInfo>,
    /// What stack this entry is at.
    pub op: UndoOp,
}
// this is just in case the args gets placed in a var
// false positives (ne when is eq) does not matter.
#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for UndoEntryArgs {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && Arc::ptr_eq(&self.info, &other.info) && self.op == other.op
    }
}
impl fmt::Debug for UndoEntryArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoEntryArgs")
            .field("timestamp", &self.timestamp)
            .field("info", &self.info.description())
            .field("op", &self.op)
            .finish()
    }
}

/// Represents an undo or redo stack.
#[derive(Clone)]
pub struct UndoStackArgs {
    /// Stack, latest at the end.
    pub stack: Vec<(Instant, Arc<dyn UndoInfo>)>,
    /// What stack this is.
    pub op: UndoOp,
}

// this is just in case the args gets placed in a var
// false positives (ne when is eq) does not matter.
#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for UndoStackArgs {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op
            && self.stack.len() == other.stack.len()
            && self
                .stack
                .iter()
                .zip(&other.stack)
                .all(|((t0, a0), (t1, a1))| t0 == t1 && Arc::ptr_eq(a0, a1))
    }
}

impl fmt::Debug for UndoStackArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoStackArgs")
            .field("stack.len()", &self.stack.len())
            .field("op", &self.op)
            .finish()
    }
}

/// Args to present the child of [`UndoHistory!`].
///
/// The args are empty in the current release, you can use [`UNDO`] to
/// get all the data needed.
///
/// [`UndoHistory!`]: struct@UndoHistory
#[derive(Debug, Clone, PartialEq)]
pub struct UndoPanelArgs {}

/// Menu style button for an entry in a undo/redo stack.
#[widget($crate::widgets::undo::UndoRedoButtonStyle)]
pub struct UndoRedoButtonStyle(crate::widgets::button::DefaultStyle);
impl UndoRedoButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            corner_radius = unset!;
            border = unset!;
            padding = (4, 6);
        }
    }
}
