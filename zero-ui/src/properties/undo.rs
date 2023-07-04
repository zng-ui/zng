use std::time::Duration;

use crate::prelude::new_property::*;

use crate::core::undo::*;

/// Sets if the widget is an undo scope.
///
/// If `true` the widget will handle [`UNDO_CMD`] and [`REDO_CMD`] for all undo actions
/// that happen inside it.
#[property(WIDGET, default(false))]
pub fn undo_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    let mut scope = WidgetUndoScope::new();
    let mut undo_cmd = CommandHandle::dummy();
    let mut redo_cmd = CommandHandle::dummy();
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
                undo_cmd = UNDO_CMD.scoped(id).subscribe(true);
                redo_cmd = REDO_CMD.scoped(id).subscribe(true);
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
                            UNDO.undo_n(n);
                        } else if let Some(&t) = args.param::<Duration>() {
                            UNDO.undo_t(t);
                        } else {
                            UNDO.undo();
                        }
                    });
                } else if let Some(args) = REDO_CMD.scoped(id).on_unhandled(update) {
                    args.propagation().stop();
                    UNDO.with_scope(&mut scope, || {
                        if let Some(&n) = args.param::<u32>() {
                            UNDO.redo_n(n);
                        } else if let Some(&t) = args.param::<Duration>() {
                            UNDO.redo_t(t);
                        } else {
                            UNDO.redo();
                        }
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

        undo_cmd.set_enabled(scope.can_undo());
        redo_cmd.set_enabled(scope.can_redo());
    })
}

/// Enable or disable undo inside the widget.
#[property(CONTEXT, default(true))]
pub fn undo_enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |c, op| {
        if !enabled.get() {
            UNDO.with_disabled(|| c.op(op))
        }
    })
}
