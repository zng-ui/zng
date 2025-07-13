#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Undo history widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use colors::BASE_COLOR_VAR;
use zng_ext_input::gesture::ClickArgs;
use zng_ext_l10n::{L10nArgument, l10n};
use zng_ext_undo::*;
use zng_wgt::{base_color, margin, prelude::*};
use zng_wgt_button::Button;
use zng_wgt_container::{Container, child_align, padding};
use zng_wgt_fill::background_color;
use zng_wgt_input::{is_cap_hovered, is_pressed};
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_size_offset::max_height;
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::{Style, StyleFn, style_fn};
use zng_wgt_text::Text;

use std::fmt;
use std::sync::Arc;

/// Undo/redo stack view.
///
/// This widget shows a snapshot of the undo/redo stacks of the focused undo scope when the history widget is open.
/// Note that the stack is not live, this widget is designed to work as a menu popup content.
#[widget($crate::UndoHistory {
    ($op:expr) => {
        op = $op;
    }
})]
pub struct UndoHistory(WidgetBase);
impl UndoHistory {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let op = wgt.capture_value::<UndoOp>(property_id!(Self::op)).unwrap_or(UndoOp::Undo);
            wgt.set_child(UNDO_PANEL_FN_VAR.present_data(UndoPanelArgs { op }));
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

    /// If undo entries are grouped by the [`UNDO.undo_interval`].
    ///
    /// Enabled by default.
    ///
    /// [`UNDO.undo_interval`]: zng_ext_undo::UNDO::undo_interval
    pub static GROUP_BY_UNDO_INTERVAL_VAR: bool = true;
}

/// Widget function that converts [`UndoEntryArgs`] to widgets.
///
/// Try [`undo_button_style_fn`] for making only visual changes.
///
/// Sets the [`UNDO_ENTRY_FN_VAR`].
///
/// [`undo_button_style_fn`]: fn@undo_button_style_fn
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
/// [`UNDO.undo_interval`]: UNDO::undo_interval
#[property(CONTEXT+1, default(GROUP_BY_UNDO_INTERVAL_VAR), widget_impl(UndoHistory))]
pub fn group_by_undo_interval(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, GROUP_BY_UNDO_INTERVAL_VAR, enabled)
}

/// Identifies what stack history is shown by the widget.
#[property(CONTEXT, capture, default(UndoOp::Undo), widget_impl(UndoHistory))]
pub fn op(op: impl IntoValue<UndoOp>) {}

/// Default [`UNDO_ENTRY_FN_VAR`].
///
/// Returns a `Button!` with the [`UNDO_BUTTON_STYLE_FN_VAR`] and the entry displayed in a `Text!` child.
/// The button notifies [`UNDO_CMD`] or [`REDO_CMD`] with the entry timestamp, the command is scoped on the
/// undo parent of the caller not of the button.
///
/// [`UndoRedoButtonStyle!`]: struct@UndoRedoButtonStyle
/// [`UNDO_CMD`]: zng_ext_undo::UNDO_CMD
/// [`REDO_CMD`]: zng_ext_undo::REDO_CMD
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
        style_fn = UNDO_BUTTON_STYLE_FN_VAR;
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
    let children: UiVec;

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
    // l10n-# Number of undo/redo actions that are selected to run
    let count = l10n!("UndoHistory.count_actions", "{$n} actions", n = count);

    Container! {
        undo_stack = args.op;

        child = Scroll! {
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children;
            };
            child_align = Align::FILL_TOP;
            mode = ScrollMode::VERTICAL;
            max_height = 200.dip().min(80.pct());
        };

        child_bottom = Text! {
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
#[non_exhaustive]
pub struct UndoEntryArgs {
    /// Info about the action.
    ///
    /// Is at least one item, can be more if [`GROUP_BY_UNDO_INTERVAL_VAR`] is enabled.
    ///
    /// The latest undo action is the last entry in the list.
    pub info: Vec<(DInstant, Arc<dyn UndoInfo>)>,

    /// What stack this entry is at.
    pub op: UndoOp,

    /// The undo or redo command in the correct scope.
    pub cmd: Command,
}
impl UndoEntryArgs {
    /// New args.
    pub fn new(info: Vec<(DInstant, Arc<dyn UndoInfo>)>, op: UndoOp, cmd: Command) -> Self {
        Self { info, op, cmd }
    }

    /// Moment the undo action was first registered.
    ///
    /// This does not change after redo and undo, it is always the register time.
    ///
    /// This is the first timestamp in `info`.
    pub fn timestamp(&self) -> DInstant {
        self.info[0].0
    }
}
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
#[non_exhaustive]
pub struct UndoStackArgs {
    /// Stack, latest at the end.
    pub stack: UndoStackInfo,
    /// What stack this is.
    pub op: UndoOp,
    /// The undo or redo command, scoped.
    pub cmd: Command,
}
impl UndoStackArgs {
    /// New args.
    pub fn new(stack: UndoStackInfo, op: UndoOp, cmd: Command) -> Self {
        Self { stack, op, cmd }
    }
}

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
#[non_exhaustive]
pub struct UndoPanelArgs {
    /// What stack history must be shown.
    pub op: UndoOp,
}
impl UndoPanelArgs {
    /// New args.
    pub fn new(op: UndoOp) -> Self {
        Self { op }
    }
}

/// Menu style button for an entry in a undo/redo stack.
#[widget($crate::UndoRedoButtonStyle)]
pub struct UndoRedoButtonStyle(Style);
impl UndoRedoButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = 4;
            child_align = Align::START;

            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));
            background_color = BASE_COLOR_VAR.rgba();

            when *#is_cap_hovered_timestamp {
                background_color = BASE_COLOR_VAR.shade(1);
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = BASE_COLOR_VAR.shade(2);
            }
        }
    }
}

context_var! {
    /// Variable set by the parent undo/redo stack widget, can be used to highlight items
    /// that will be included in the undo/redo operation.
    static HOVERED_TIMESTAMP_VAR: Option<DInstant> = None;

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
    pub static UNDO_BUTTON_STYLE_FN_VAR: StyleFn = style_fn!(|_| UndoRedoButtonStyle!());
}

/// Extend or replace the undo/redo entry button style in a context.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn undo_button_style_fn(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    zng_wgt_style::with_style_fn(child, UNDO_BUTTON_STYLE_FN_VAR, style)
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
                .hook(move |a| {
                    let is_hovered = *a.value();
                    let _ = actual.modify(move |a| {
                        if is_hovered {
                            a.set(Some(timestamp));
                        } else if a.as_ref() == &Some(timestamp) {
                            a.set(None);
                        }
                    });
                    true
                })
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
    bind_state(
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
