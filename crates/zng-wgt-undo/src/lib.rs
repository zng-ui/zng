#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Undo properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::time::Duration;

use zng_ext_undo::*;
use zng_wgt::prelude::*;

/// Sets if the widget is an undo scope.
///
/// If `true` the widget will handle [`UNDO_CMD`] and [`REDO_CMD`] for all undo actions
/// that happen inside it.
///
/// [`UNDO_CMD`]: static@zng_ext_undo::UNDO_CMD
/// [`REDO_CMD`]: static@zng_ext_undo::REDO_CMD
#[property(CONTEXT - 10, default(false))]
pub fn undo_scope(child: impl IntoUiNode, is_scope: impl IntoVar<bool>) -> UiNode {
    let mut scope = WidgetUndoScope::new();
    let mut undo_cmd = CommandHandle::dummy();
    let mut redo_cmd = CommandHandle::dummy();
    let mut clear_cmd = CommandHandle::dummy();
    let is_scope = is_scope.into_var();
    match_node(child, move |c, mut op| {
        match &mut op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&is_scope);

                if !is_scope.get() {
                    return; // default handling without scope context.
                }

                scope.init();

                let id = WIDGET.id();
                undo_cmd = UNDO_CMD.scoped(id).subscribe(false);
                redo_cmd = REDO_CMD.scoped(id).subscribe(false);
                clear_cmd = CLEAR_HISTORY_CMD.scoped(id).subscribe(false);
            }
            UiNodeOp::Deinit => {
                if !is_scope.get() {
                    return;
                }

                UNDO.with_scope(&mut scope, || c.deinit());
                scope.deinit();
                undo_cmd = CommandHandle::dummy();
                redo_cmd = CommandHandle::dummy();
                return;
            }
            UiNodeOp::Info { info } => {
                if !is_scope.get() {
                    return;
                }
                scope.info(info);
            }
            UiNodeOp::Event { update } => {
                if !is_scope.get() {
                    return;
                }

                let id = WIDGET.id();
                if let Some(args) = UNDO_CMD.scoped(id).on_unhandled(update) {
                    args.propagation().stop();
                    UNDO.with_scope(&mut scope, || {
                        if let Some(&n) = args.param::<u32>() {
                            UNDO.undo_select(n);
                        } else if let Some(&i) = args.param::<Duration>() {
                            UNDO.undo_select(i);
                        } else if let Some(&t) = args.param::<DInstant>() {
                            UNDO.undo_select(t);
                        } else {
                            UNDO.undo();
                        }
                    });
                } else if let Some(args) = REDO_CMD.scoped(id).on_unhandled(update) {
                    args.propagation().stop();
                    UNDO.with_scope(&mut scope, || {
                        if let Some(&n) = args.param::<u32>() {
                            UNDO.redo_select(n);
                        } else if let Some(&i) = args.param::<Duration>() {
                            UNDO.redo_select(i);
                        } else if let Some(&t) = args.param::<DInstant>() {
                            UNDO.redo_select(t);
                        } else {
                            UNDO.redo();
                        }
                    });
                } else if let Some(args) = CLEAR_HISTORY_CMD.scoped(id).on_unhandled(update) {
                    args.propagation().stop();
                    UNDO.with_scope(&mut scope, || {
                        UNDO.clear();
                    });
                }
            }
            UiNodeOp::Update { .. } => {
                if let Some(is_scope) = is_scope.get_new() {
                    WIDGET.info();

                    if is_scope {
                        if !scope.is_inited() {
                            scope.init();

                            let id = WIDGET.id();
                            undo_cmd = UNDO_CMD.scoped(id).subscribe(false);
                            redo_cmd = REDO_CMD.scoped(id).subscribe(false);
                        }
                    } else if scope.is_inited() {
                        scope.deinit();
                        undo_cmd = CommandHandle::dummy();
                        redo_cmd = CommandHandle::dummy();
                    }
                }
                if !is_scope.get() {
                    return;
                }
            }
            _ => {
                if !is_scope.get() {
                    return;
                }
            }
        }

        UNDO.with_scope(&mut scope, || c.op(op));

        let can_undo = scope.can_undo();
        let can_redo = scope.can_redo();
        undo_cmd.set_enabled(can_undo);
        redo_cmd.set_enabled(can_redo);
        clear_cmd.set_enabled(can_undo || can_redo);
    })
}

/// Enable or disable undo inside the widget.
#[property(CONTEXT, default(true))]
pub fn undo_enabled(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |c, op| {
        if !enabled.get() {
            UNDO.with_disabled(|| c.op(op))
        }
    })
}

/// Sets the maximum length for undo/redo stacks in the widget and descendants.
///
/// This property sets the [`UNDO_LIMIT_VAR`].
///
/// [`UNDO_LIMIT_VAR`]: zng_ext_undo::UNDO_LIMIT_VAR
#[property(CONTEXT - 11, default(UNDO_LIMIT_VAR))]
pub fn undo_limit(child: impl IntoUiNode, max: impl IntoVar<u32>) -> UiNode {
    with_context_var(child, UNDO_LIMIT_VAR, max)
}

/// Sets the time interval that undo and redo cover each call for undo handlers in the widget and descendants.
///
/// When undo is requested inside the context all actions after the latest that are within `interval` of the
/// previous are undone.
///
/// This property sets the [`UNDO_INTERVAL_VAR`].
///
/// [`UNDO_INTERVAL_VAR`]: zng_ext_undo::UNDO_INTERVAL_VAR
#[property(CONTEXT - 11, default(UNDO_INTERVAL_VAR))]
pub fn undo_interval(child: impl IntoUiNode, interval: impl IntoVar<Duration>) -> UiNode {
    with_context_var(child, UNDO_INTERVAL_VAR, interval)
}

/// Undo scope widget mixin.
///
/// Widget is an undo/redo scope, it tracks changes and handles undo/redo commands.
///
/// You can force the widget to use a parent undo scope by setting [`undo_scope`] to `false`, this will cause the widget
/// to start registering undo/redo actions in the parent, note that the widget will continue behaving as if it
/// owns the scope, so it may clear it.
///
/// [`undo_scope`]: fn@undo_scope
#[widget_mixin]
pub struct UndoMix<P>(P);

impl<P: WidgetImpl> UndoMix<P> {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            crate::undo_scope = true;
        }
    }

    widget_impl! {
        /// If the widget can register undo actions.
        ///
        /// Is `true` by default in this widget, if set to `false` disables undo in the widget.
        pub undo_enabled(enabled: impl IntoVar<bool>);

        /// Sets the maximum number of undo/redo actions that are retained in the widget.
        pub undo_limit(limit: impl IntoVar<u32>);

        /// Sets the time interval that undo and redo cover each call for undo handlers in the widget and descendants.
        ///
        /// When undo is requested inside the context all actions after the latest that are within `interval` of the
        /// previous are undone.
        pub undo_interval(interval: impl IntoVar<Duration>);
    }
}
