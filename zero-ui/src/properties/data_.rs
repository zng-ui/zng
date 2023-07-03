use std::sync::Arc;

use crate::{core::var::types::ContextualizedVar, prelude::new_property::*};

/// Data context.
///
/// Sets the [`DATA`] context for this widget and descendants, replacing the parent's data. Note that only
/// one data context can be set at a time, the `data` will override the parent's data even if the type `T`
/// does not match.
#[property(CONTEXT - 1)]
pub fn data<T: VarValue>(child: impl UiNode, data: impl IntoVar<T>) -> impl UiNode {
    with_context_local(child, &DATA_CTX, data.into_var().boxed_any())
}

/// Data context.
///
/// Arbitrary data can be set on a context using the [`data`] property and retrieved using [`DATA.get`].
///
/// [`data`]: fn@data
pub struct DATA;
impl DATA {
    /// Require context data of type `T`.
    ///
    /// # Panics
    ///
    /// Panics if the context data is not set to a variable of type `T` on the first usage of the returned variable.
    pub fn req<T: VarValue>(&self) -> ContextualizedVar<T, BoxedVar<T>> {
        self.get(|| panic!("expected DATA of type `{}`", std::any::type_name::<T>()))
    }

    /// Get context data of type `T` if the context data is set with the same type, or gets the `fallback` value.
    pub fn get<T: VarValue>(&self, fallback: impl Fn() -> T + Send + Sync + 'static) -> ContextualizedVar<T, BoxedVar<T>> {
        ContextualizedVar::new(Arc::new(move || {
            DATA_CTX
                .get()
                .clone_any()
                .double_boxed_any()
                .downcast::<BoxedVar<T>>()
                .map(|b| *b)
                .unwrap_or_else(|_| LocalVar(fallback()).boxed())
        }))
    }

    /// Gets the current context data.
    ///
    /// Note that this is does not return a contextualizing var like [`get`], it gets the data var in the calling context.
    ///
    /// [`get`]: Self::get
    pub fn get_any(&self) -> BoxedAnyVar {
        DATA_CTX.get().clone_any()
    }
}

context_local! {
    static DATA_CTX: BoxedAnyVar = LocalVar(()).boxed_any();
}
