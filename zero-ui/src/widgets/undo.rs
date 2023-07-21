//! Undo scope mix and undo history widget.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use zero_ui_core::undo::CommandUndoExt;

use crate::prelude::new_widget::*;

use crate::core::gesture::ClickArgs;
use crate::core::undo::{UndoInfo, UndoOp, UndoStackInfo, REDO_CMD, UNDO_CMD};
use crate::widgets::button;
use crate::widgets::{
    layouts::{stack::StackDirection, Stack},
    view, Button, Scroll, Text,
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

    /// If undo entries are grouped by the [`UNDO::undo_interval`].
    ///
    /// Enabled by default.
    pub static GROUP_BY_UNDO_INTERVAL_VAR: bool = true;
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

/// If undo entries are grouped by the [`UNDO.undo_interval`].
///
/// Enabled by default.
///
/// Sets the [`GROUP_BY_UNDO_INTERVAL_VAR`].
///
/// [`UNDO.undo_interval`]: crate::core::undo::UNDO::undo_interval
#[property(CONTEXT+1, default(GROUP_BY_UNDO_INTERVAL_VAR), widget_impl(UndoHistory))]
pub fn group_by_undo_interval(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, GROUP_BY_UNDO_INTERVAL_VAR, enabled)
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
    let ts = args.timestamp();
    let cmd = args.cmd;

    let label = if args.info.len() == 1 {
        args.info[0].1.description()
    } else {
        let mut txt = Txt::from_static("");
        let mut sep = "";
        let mut info_iter = args.info.iter();
        for (_, info) in &mut info_iter {
            use std::fmt::Write;

            if txt.chars().take(10).count() == 10 {
                let count = 1 + info_iter.count();
                let _ = write!(&mut txt, "{sep}{count}");
                break;
            }

            let _ = write!(&mut txt, "{sep}{}", info.description());
            sep = "₊";
        }
        txt.end_mut();
        txt
    };

    Button! {
        child = Text!(label);
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

    let timestamps;
    let children: UiNodeVec;

    if GROUP_BY_UNDO_INTERVAL_VAR.get() {
        let mut ts = vec![];

        children = args
            .stack
            .iter_groups()
            .rev()
            .map(|g| {
                let e = UndoEntryArgs {
                    info: g.to_vec(),
                    op: args.op,
                    cmd: args.cmd,
                };
                ts.push(e.timestamp());
                entry(e)
            })
            .collect();

        timestamps = ts;
    } else {
        timestamps = args.stack.stack.iter().rev().map(|(i, _)| *i).collect();
        children = args
            .stack
            .stack
            .into_iter()
            .rev()
            .map(|info| {
                entry(UndoEntryArgs {
                    info: vec![info],
                    op: args.op,
                    cmd: args.cmd,
                })
            })
            .collect();
    };

    let op = args.op;
    let count = HOVERED_TIMESTAMP_VAR.map(move |t| {
        let c = match t {
            Some(t) => match op {
                UndoOp::Undo => timestamps.iter().take_while(|ts| *ts >= t).count(),
                UndoOp::Redo => timestamps.iter().take_while(|ts| *ts <= t).count(),
            },
            None => 0,
        };
        L10nArgument::from(c)
    });
    let count = l10n!("UndoHistory.count_actions", "{$n} actions", n = count);

    crate::widgets::Container! {
        undo_stack = args.op;

        child = Scroll! {
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children;
            };
            child_align = Align::FILL_TOP;
            mode = crate::widgets::scroll::ScrollMode::VERTICAL;
            max_height = 200.dip().min(80.pct());
        };

        child_insert_below = Text! {
            margin = 2;
            txt = count;
            txt_align = Align::CENTER;
        }, 0;
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
    /// Info about the action.
    ///
    /// Is at least one item, can be more if [`GROUP_BY_UNDO_INTERVAL_VAR`] is enabled.
    ///
    /// The latest undo action is the last entry in the list.
    pub info: Vec<(Instant, Arc<dyn UndoInfo>)>,

    /// What stack this entry is at.
    pub op: UndoOp,

    /// The undo or redo command in the correct scope.
    pub cmd: Command,
}
impl UndoEntryArgs {
    /// Moment the undo action was first registered.
    ///
    /// This does not change after redo and undo, it is always the register time.
    ///
    /// This is the first timestamp in `info`.
    pub fn timestamp(&self) -> Instant {
        self.info[0].0
    }
}
// this is just in case the args gets placed in a var
// false positives (ne when is eq) does not matter.
#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for UndoEntryArgs {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op
            && self.info.len() == other.info.len()
            && self
                .info
                .iter()
                .zip(&other.info)
                .all(|(a, b)| a.0 == b.0 && Arc::ptr_eq(&a.1, &b.1))
    }
}
impl fmt::Debug for UndoEntryArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoEntryArgs")
            .field("info[0]", &self.info[0].1.description())
            .field("op", &self.op)
            .finish()
    }
}

/// Represents an undo or redo stack.
#[derive(Clone)]
pub struct UndoStackArgs {
    /// Stack, latest at the end.
    pub stack: UndoStackInfo,
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
            && self.stack.stack.len() == other.stack.stack.len()
            && self
                .stack
                .stack
                .iter()
                .zip(&other.stack.stack)
                .all(|((t0, a0), (t1, a1))| t0 == t1 && Arc::ptr_eq(a0, a1))
    }
}

impl fmt::Debug for UndoStackArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoStackArgs")
            .field("stack.len()", &self.stack.stack.len())
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

            background_color = color_scheme_pair(button::BASE_COLORS_VAR);

            when *#is_cap_hovered_timestamp {
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
    let timestamp = entry.timestamp();
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
                    Some(UndoOp::Undo) => entry.timestamp() >= ts,
                    Some(UndoOp::Redo) => entry.timestamp() <= ts,
                    None => false,
                },
                _ => false,
            }
        }),
        state,
    )
}
