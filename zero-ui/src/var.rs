//! Variables API.
//!
//! # Full API
//!
//! See [`zero_ui_var`] for the full var API.

pub use zero_ui_var::types::{
    AnyWhenVarBuilder, ArcCowVar, ArcWhenVar, ContextualizedVar, ReadOnlyVar, Response, VecChange, WeakArcVar, WeakContextInitHandle,
    WeakContextualizedVar, WeakReadOnlyVar, WeakWhenVar,
};
pub use zero_ui_var::{
    context_var, expr_var, getter_var, merge_var, response_done_var, response_var, state_var, var, var_default, var_from, when_var, AnyVar,
    AnyVarValue, AnyWeakVar, ArcEq, ArcVar, BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar, ContextInitHandle, ContextVar, IntoValue,
    IntoVar, LocalVar, MergeVarBuilder, ObservableVec, ReadOnlyArcVar, ReadOnlyContextVar, ResponderVar, ResponseVar, TraceValueArgs, Var,
    VarCapabilities, VarHandle, VarHandles, VarHookArgs, VarModify, VarPtr, VarUpdateId, VarValue, WeakVar, VARS,
};

pub use zero_ui_app::widget::{AnyVarSubscribe, VarLayout, VarSubscribe};

/// Var animation types and functions.
pub mod animation {
    pub use zero_ui_var::animation::{
        Animation, AnimationController, AnimationHandle, AnimationTimer, ChaseAnimation, ModifyInfo, NilAnimationObserver, Transition,
        TransitionKeyed, Transitionable, WeakAnimationHandle,
    };

    /// Common easing functions.
    pub mod easing {
        pub use zero_ui_var::animation::easing::{
            back, bounce, circ, cubic, cubic_bezier, ease_in, ease_in_out, ease_out, ease_out_in, elastic, expo, linear, none, quad, quart,
            quint, reverse, reverse_out, sine, step_ceil, step_floor, Bezier, EasingFn, EasingModifierFn, EasingStep, EasingTime,
        };
    }
}
