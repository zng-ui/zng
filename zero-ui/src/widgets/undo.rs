//! Undo scope mix and undo history widget.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use zero_ui_core::undo::CommandUndoExt;

use crate::prelude::new_widget::*;

use crate::core::gesture::ClickArgs;
use crate::core::undo::{UndoInfo, UndoOp, REDO_CMD, UNDO_CMD};
use crate::widgets::button;
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
///
/// This widget shows a snapshot of the undo/redo stacks of the focused undo scope when the history widget is open.
/// Note that the stack is not live, this widget is designed to work as a menu popup content.
#[widget($crate::widgets::undo::UndoHistory {
    ($op:expr) => {
        op = $op;
    }
})]
pub struct UndoHistory(WidgetBase);
impl UndoHistory {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let op = wgt.capture_value::<UndoOp>(property_id!(Self::op)).unwrap_or(UndoOp::Undo);
            wgt.set_child(view::presenter(UndoPanelArgs { op }, UNDO_PANEL_FN_VAR));
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
/// Try [`extend_undo_button_style`] for making only visual changes.
///
/// Sets the [`UNDO_ENTRY_FN_VAR`].
///
/// [`extend_undo_button_style`]: fn@extend_undo_button_style
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

/// Identifies what stack history is shown by the widget.
#[property(CONTEXT, capture, default(UndoOp::Undo), widget_impl(UndoHistory))]
pub fn op(op: impl IntoValue<UndoOp>) {}

/// Default [`UNDO_ENTRY_FN_VAR`].
///
/// Returns a `Button!` with the [`UNDO_BUTTON_STYLE_VAR`] and the entry displayed in a `Text!` child.
/// The button notifies [`UNDO_CMD`] or [`REDO_CMD`] with the entry timestamp, the command is scoped on the
/// undo parent of the caller not of the button.
///
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
pub fn default_undo_entry_fn(args: UndoEntryArgs) -> impl UiNode {
    let ts = args.timestamp;
    let cmd = args.cmd;
    Button! {
        child = Text!(args.info.description());
        undo_entry = args;
        style_fn = UNDO_BUTTON_STYLE_VAR;
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
                cmd: args.cmd,
            })
        })
        .collect::<UiNodeVec>();

    Stack! {
        undo_stack = args.op;
        direction = StackDirection::top_to_bottom();
        children;
    }
}

/// Default [`UNDO_PANEL_FN_VAR`].
pub fn default_undo_panel_fn(args: UndoPanelArgs) -> impl UiNode {
    let stack = UNDO_STACK_FN_VAR.get();
    match args.op {
        UndoOp::Undo => {
            let cmd = UNDO_CMD.undo_scoped().get();
            stack(UndoStackArgs {
                stack: cmd.undo_stack(),
                op: UndoOp::Undo,
                cmd,
            })
        }
        UndoOp::Redo => {
            let cmd = REDO_CMD.undo_scoped().get();
            stack(UndoStackArgs {
                stack: cmd.redo_stack(),
                op: UndoOp::Redo,
                cmd,
            })
        }
    }
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

    /// The undo or redo command in the correct scope.
    pub cmd: Command,
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
    /// The undo or redo command, scoped.
    pub cmd: Command,
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
/// [`UndoHistory!`]: struct@UndoHistory
#[derive(Debug, Clone, PartialEq)]
pub struct UndoPanelArgs {
    /// What stack history must be shown.
    pub op: UndoOp,
}

/// Menu style button for an entry in a undo/redo stack.
#[widget($crate::widgets::undo::UndoRedoButtonStyle)]
pub struct UndoRedoButtonStyle(Style);
impl UndoRedoButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = 4;
            child_align = Align::START;

            #[easing(150.ms())]
            background_color = color_scheme_pair(button::BASE_COLORS_VAR);

            when *#is_cap_hovered_timestamp {
                #[easing(0.ms())]
                background_color = button::color_scheme_hovered(button::BASE_COLORS_VAR);
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = button::color_scheme_pressed(button::BASE_COLORS_VAR);
            }
        }
    }
}

context_var! {
    /// Variable set by the parent undo/redo stack widget, can be used to highlight items
    /// that will be included in the undo/redo operation.
    static HOVERED_TIMESTAMP_VAR: Option<Instant> = None;

    /// Variable set in each undo/redo entry widget.
    pub static UNDO_ENTRY_VAR: Option<UndoEntryArgs> = None;

    /// Variable set in each undo/redo stack widget.
    pub static UNDO_STACK_VAR: Option<UndoOp> = None;

    /// Style for the default undo/redo entry [`Button!`].
    ///
    /// Is [`UndoRedoButtonStyle!`] by default.
    ///
    /// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
    /// [`Button!`]: struct@Button
    pub static UNDO_BUTTON_STYLE_VAR: StyleFn = style_fn!(|_| UndoRedoButtonStyle!());
}

/// Sets the undo/redo entry button style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(UNDO_BUTTON_STYLE_VAR))]
pub fn replace_undo_button_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, UNDO_BUTTON_STYLE_VAR, style)
}

/// Extends the undo/redo entry button style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_undo_button_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, UNDO_BUTTON_STYLE_VAR, style)
}

/// Sets the undo/redo entry widget context.
///
/// In the widget style the [`UNDO_ENTRY_VAR`] can be used to access the [`UndoEntryArgs`].
#[property(CONTEXT-1)]
pub fn undo_entry(child: impl UiNode, entry: impl IntoValue<UndoEntryArgs>) -> impl UiNode {
    let entry = entry.into();

    // set the hovered timestamp
    let timestamp = entry.timestamp;
    let is_hovered = var(false);
    let child = is_cap_hovered(child, is_hovered.clone());
    let child = match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let actual = HOVERED_TIMESTAMP_VAR.actual_var();
            is_hovered
                .hook(Box::new(move |a| {
                    let is_hovered = *a.downcast_value::<bool>().unwrap();
                    let _ = actual.modify(move |a| {
                        if is_hovered {
                            a.set(Some(timestamp));
                        } else if a.as_ref() == &Some(timestamp) {
                            a.set(None);
                        }
                    });
                    true
                }))
                .perm();
        }
    });

    with_context_var(child, UNDO_ENTRY_VAR, Some(entry))
}

/// Setups the context in an undo/redo stack widget.
///
/// If this is not set in the stack widget the entry widgets may not work properly.
#[property(CONTEXT-1)]
pub fn undo_stack(child: impl UiNode, op: impl IntoValue<UndoOp>) -> impl UiNode {
    let child = with_context_var(child, HOVERED_TIMESTAMP_VAR, var(None));
    with_context_var(child, UNDO_STACK_VAR, Some(op.into()))
}

/// State is true when the widget is an [`undo_entry`] and it is hovered, has captured the mouse
/// or a sibling with higher timestamp is hovered/has cap.
///
/// [`undo_entry`]: fn@undo_entry
#[property(CONTEXT)]
pub fn is_cap_hovered_timestamp(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    // check the hovered timestamp
    bind_is_state(
        child,
        merge_var!(HOVERED_TIMESTAMP_VAR, UNDO_ENTRY_VAR, UNDO_STACK_VAR, |&ts, entry, &op| {
            match (ts, entry) {
                (Some(ts), Some(entry)) => match op {
                    Some(UndoOp::Undo) => entry.timestamp >= ts,
                    Some(UndoOp::Redo) => entry.timestamp <= ts,
                    None => false,
                },
                _ => false,
            }
        }),
        state,
    )
}
