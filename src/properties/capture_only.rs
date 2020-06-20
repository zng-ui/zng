//! Properties that are only used by widgets directly by capturing then in the `new` or `new_child` function.

use crate::core::{property, types::WidgetId, var::IntoVar, UiNode};

/// Widget id.
///
/// # Placeholder
///
/// This property is a placeholder that does not do anything directly, widgets can
/// capture this value for their own initialization.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`](implicit_mixin) that defines an `id`
/// property that maps to this property and sets a default value of `WidgetId::new_unique()`.
///
/// The default widget `new` function captures this `id` property and uses in the default
/// [`Widget`](crate::core::Widget) implementation.
#[property(context)]
pub fn widget_id<C: UiNode>(child: C, id: WidgetId) -> C {
    let _id = id;
    error_println!("id property cannot be set directly, must be captured in widget!'s new()");
    child
}

/// Stack in-between spacing.
///
/// # Placeholder
///
/// This property is a placeholder that does not do anything directly, widgets can
/// capture this value for their own initialization.
#[property(context)]
pub fn stack_spacing<C: UiNode>(child: C, spacing: impl IntoVar<f32>) -> C {
    let _spacing = spacing;
    child
}

#[allow(unused)]
macro_rules! capture_only_priority {
    () => {
        // * generates a doc(hidden) set and set_args.
        // * add an assert with a compile error if not captured.
        // * don't require the child argument?

        // different item syntax:
        /// Docs
        #[property(capture_only)]
        pub foo(spacing: impl IntoVar<f32>);
        // no.
        {} // rust-analyzer syntax color clear

        // same syntax, user decides what kind of error happens if foo::set() is called:
        #[property(capture_only)]
        pub fn foo<C: UiNode>(child: C, spacing: impl IntoVar<f32>) -> C {
            let _spacing = spacing;
            panic!("`foo` cannot be used directly");
            child
        }

        // almost syntax?
        #[property(capture_only)]
        pub foo(spacing: impl IntoVar<f32>) -> !;

        // different item
        #[property(capture_only)]
        pub struct foo<S: IntoVar<f32>> {
            spacing: S,// no impl here..
        }
    };
}
