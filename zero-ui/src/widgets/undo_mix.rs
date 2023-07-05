//! Undo scope mix.
//!

use crate::prelude::new_widget::*;

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
    }
}
