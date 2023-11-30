//! UI helpers.

use std::{any::Any, sync::Arc, time::Duration};

use parking_lot::Mutex;
use zero_ui_var::{
    animation::{
        easing::{EasingStep, EasingTime},
        AnimationHandle, Transitionable,
    },
    *,
};

use crate::{
    context::{StateMapRef, WIDGET},
    event::{Event, EventArgs},
    units::TimeUnits,
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction, PropertyInputTypes, WhenBuildAction},
    widget_instance::{match_node, UiNode, UiNodeOp},
};

/// Expands a property assign to include an easing animation.
///
/// The attribute generates a [property build action] that applies [`Var::easing`] to the final variable inputs of the property.
///
/// # Arguments
///
/// The attribute takes one required argument and one optional that matches the [`Var::easing`]
/// parameters. The required first arg is the duration, the second arg is an easing function, if not present the [`easing::linear`] is used.
///
/// Some items are auto-imported in each argument scope, the [`TimeUnits`] are imported in the first argument, so you can use syntax
/// like `300.ms()` to declare the duration, all of the [`easing::*`] functions are imported in the second argument so you can use
/// the function names directly.
///
/// ## Unset
///
/// An alternative argument `unset` can be used instead to remove animations set by the inherited context or styles.
///
/// [`TimeUnits`]: crate::units::TimeUnits
/// [`easing::*`]: mod@crate::var::easing
/// [property build action]: crate::widget_builder::WidgetBuilder::push_property_build_action
///
/// ## When
///
/// The attribute can also be set in `when` assigns, in this case the easing will be applied when the condition is active, so
/// only the transition to the `true` value is animated using the conditional easing.
///
/// Note that you can't `unset` easing in when conditions, but you can set it to `0.ms()`, if all easing set for a property are `0`
/// no easing variable is generated, but in contexts that actually have animation the when value will be set *immediately*,
/// by a zero sized animation.
///
/// # Examples
///
/// The example demonstrates setting and removing easing animations.
///
/// ```
/// # use zero_ui_core::{*, var::*, color::*, widget_instance::*, units::SideOffsets};
/// # #[widget($crate::Foo)] pub struct Foo(widget_base::WidgetBase);
/// # #[property(FILL, default(colors::BLACK))]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #    child
/// # }
/// # #[property(LAYOUT, default(0))]
/// # pub fn margin(child: impl UiNode, color: impl IntoVar<SideOffsets>) -> impl UiNode {
/// #    child
/// # }
/// # fn main() {
/// Foo! {
///     #[easing(300.ms(), expo)] // set/override the easing.
///     background_color = colors::RED;
///
///     #[easing(unset)] // remove easing set by style or widget defaults.
///     margin = 0;
/// }
/// # ; }
/// ```
///
/// # Limitations
///
/// The attribute only works in properties that only have variable inputs of types that are [`Transitionable`], if the attribute
/// is set in a property that does not match this a cryptic type error occurs, with a mention of `easing_property_input_Transitionable`.
///
#[doc(inline)]
pub use zero_ui_proc_macros::easing;

/// Helper for declaring properties that sets a context var.
///
/// The method presents the `value` as the [`ContextVar<T>`] in the widget and widget descendants.
/// The context var [`is_new`] and [`read_only`] status are always equal to the `value` var status. Users
/// of the context var can also retrieve the `value` var using [`actual_var`].
///
/// The generated [`UiNode`] delegates each method to `child` inside a call to [`ContextVar::with_context`].
///
/// # Examples
///
/// A simple context property declaration:
///
/// ```
/// # fn main() -> () { }
/// # use zero_ui_core::{*, widget_instance::*, var::*};
/// context_var! {
///     pub static FOO_VAR: u32 = 0u32;
/// }
///
/// /// Sets the [`FooVar`] in the widgets and its content.
/// #[property(CONTEXT, default(FOO_VAR))]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     with_context_var(child, FOO_VAR, value)
/// }
/// ```
///
/// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FOO_VAR.get`, and if `value` is set to a
/// variable the `FOO_VAR` will also reflect its [`is_new`] and [`read_only`]. If the `value` var is not read-only inner nodes
/// can modify it using `FOO_VAR.set` or `FOO_VAR.modify`.
///
/// Also note that the property [`default`] is set to the same `FOO_VAR`, this causes the property to *pass-through* the outer context
/// value, as if it was not set.
///
/// **Tip:** You can use a [`merge_var!`] to merge a new value to the previous context value:
///
/// ```
/// # fn main() -> () { }
/// # use zero_ui_core::{*, widget_instance::*, var::*};
///
/// #[derive(Debug, Clone, Default, PartialEq)]
/// pub struct Config {
///     pub foo: bool,
///     pub bar: bool,
/// }
///
/// context_var! {
///     pub static CONFIG_VAR: Config = Config::default();
/// }
///
/// /// Sets the *foo* config.
/// #[property(CONTEXT, default(false))]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
///         let mut c = c.clone();
///         c.foo = v;
///         c
///     }))
/// }
///
/// /// Sets the *bar* config.
/// #[property(CONTEXT, default(false))]
/// pub fn bar(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
///         let mut c = c.clone();
///         c.bar = v;
///         c
///     }))
/// }
/// ```
///
/// When set in a widget, the [`merge_var!`] will read the context value of the parent properties, modify a clone of the value and
/// the result will be accessible to the inner properties, the widget user can then set with the composed value in steps and
/// the final consumer of the composed value only need to monitor to a single context variable.
///
/// [`is_new`]: AnyVar::is_new
/// [`read_only`]: Var::read_only
/// [`actual_var`]: Var::actual_var
/// [`default`]: crate::property#default
pub fn with_context_var<T: VarValue>(child: impl UiNode, context_var: ContextVar<T>, value: impl IntoVar<T>) -> impl UiNode {
    let value = value.into_var();
    let mut actual_value = None;
    let mut id = None;

    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                id = Some(ContextInitHandle::new());
                actual_value = Some(Arc::new(value.clone().actual_var().boxed()));
            }
            UiNodeOp::Deinit => {
                is_deinit = true;
            }
            _ => {}
        }

        context_var.with_context(id.clone().expect("node not inited"), &mut actual_value, || child.op(op));

        if is_deinit {
            id = None;
            actual_value = None;
        }
    })
}

/// Helper for declaring properties that sets a context var to a value generated on init.
///
/// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextVar<T>`]
/// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
///
/// Apart from the value initialization this behaves just like [`with_context_var`].
pub fn with_context_var_init<T: VarValue>(
    child: impl UiNode,
    var: ContextVar<T>,
    mut init_value: impl FnMut() -> BoxedVar<T> + Send + 'static,
) -> impl UiNode {
    let mut id = None;
    let mut value = None;
    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                id = Some(ContextInitHandle::new());
                value = Some(Arc::new(init_value().actual_var()));
            }
            UiNodeOp::Deinit => {
                is_deinit = true;
            }
            _ => {}
        }

        var.with_context(id.clone().expect("node not inited"), &mut value, || child.op(op));

        if is_deinit {
            id = None;
            value = None;
        }
    })
}

/// Wraps `child` in a node that provides a unique [`ContextInitHandle`], refreshed every (re)init.
///
/// Note that [`with_context_var`] and [`with_context_var_init`] already provide an unique ID.
pub fn with_new_context_init_id(child: impl UiNode) -> impl UiNode {
    let mut id = None;

    match_node(child, move |child, op| {
        let is_deinit = matches!(op, UiNodeOp::Deinit);
        id.get_or_insert_with(ContextInitHandle::new).with_context(|| child.op(op));

        if is_deinit {
            id = None;
        }
    })
}

type EasingFn = Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>;

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait easing_property: Send + Sync + Clone + Copy {
    fn easing_property_unset(self);
    fn easing_property(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>>;
    fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction;
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait easing_property_input_Transitionable: Any + Send {
    fn easing(self, duration: Duration, easing: EasingFn, when_conditions_data: &[Option<Arc<dyn Any + Send + Sync>>]) -> Self;
}
impl<T: VarValue + Transitionable> easing_property_input_Transitionable for BoxedVar<T> {
    fn easing(self, duration: Duration, easing: EasingFn, when_conditions_data: &[Option<Arc<dyn Any + Send + Sync>>]) -> Self {
        if let Some(when) = self.as_any().downcast_ref::<types::ContextualizedVar<T, types::ArcWhenVar<T>>>() {
            let conditions: Vec<_> = when_conditions_data
                .iter()
                .map(|d| d.as_ref().and_then(|d| d.downcast_ref::<(Duration, EasingFn)>().cloned()))
                .collect();

            if conditions.iter().any(|c| c.is_some()) {
                let when = when.clone();
                return types::ContextualizedVar::new(Arc::new(move || {
                    when.borrow_init().easing_when(conditions.clone(), (duration, easing.clone()))
                }))
                .boxed();
            }
        }
        Var::easing(&self, duration, move |t| easing(t)).boxed()
    }
}

macro_rules! impl_easing_property_inputs {
    ($T0:ident, $($T:ident,)*) => {
        impl_easing_property_inputs! {
            $($T,)*
        }

        impl<
            $T0: easing_property_input_Transitionable,
            $($T: easing_property_input_Transitionable),*
        > easing_property for PropertyInputTypes<($T0, $($T,)*)> {
            fn easing_property_unset(self) { }
            fn easing_property(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>> {
                if duration == Duration::ZERO {
                    vec![]
                } else {
                    vec![
                        Box::new(PropertyBuildAction::<$T0>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone(), &a.when_conditions_data)))),
                        $(Box::new(PropertyBuildAction::<$T>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone(), &a.when_conditions_data)))),)*
                    ]
                }
            }
            fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction {
                if duration == Duration::ZERO {
                    WhenBuildAction::new_no_default((duration, easing))
                } else {
                    WhenBuildAction::new(
                        (duration, easing),
                        || {
                            let easing = Arc::new($crate::var::animation::easing::linear) as EasingFn;
                            vec![
                                Box::new(PropertyBuildAction::<$T0>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, 0.ms(), easing.clone(), &a.when_conditions_data)))),
                                $(Box::new(PropertyBuildAction::<$T>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, 0.ms(), easing.clone(), &a.when_conditions_data)))),)*
                            ]
                        }
                    )
                }
            }
        }
    };
    () => { };
}
impl_easing_property_inputs! {
    I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15,
}

/// Easing when extension for ArcWhenVar.
pub trait VarEasingWhen<T: VarValue>: Var<T> {
    /// Create a variable similar to [`Var::easing`], but with different duration and easing functions for each condition.
    ///
    /// The `condition_easing` must contain one entry for each when condition, entries can be `None`, the easing used
    /// is the first entry that corresponds to a `true` condition, or falls-back to the `default_easing`.
    fn easing_when(
        &self,
        condition_easing: Vec<Option<(Duration, EasingFn)>>,
        default_easing: (Duration, EasingFn),
    ) -> types::ContextualizedVar<T, ReadOnlyArcVar<T>>
    where
        T: Transitionable;
}
impl<T: VarValue> VarEasingWhen<T> for super::types::ArcWhenVar<T> {
    fn easing_when(
        &self,
        condition_easing: Vec<Option<(Duration, EasingFn)>>,
        default_easing: (Duration, EasingFn),
    ) -> types::ContextualizedVar<T, ReadOnlyArcVar<T>>
    where
        T: Transitionable,
    {
        let source = self.clone();
        types::ContextualizedVar::new(Arc::new(move || {
            debug_assert_eq!(source.conditions().len(), condition_easing.len());

            let source_wk = source.downgrade();
            let easing_var = super::var(source.get());

            let condition_easing = condition_easing.clone();
            let default_easing = default_easing.clone();
            let mut _anim_handle = AnimationHandle::dummy();
            var_bind(&source, &easing_var, move |value, _, easing_var| {
                let source = source_wk.upgrade().unwrap();
                for ((c, _), easing) in source.conditions().iter().zip(&condition_easing) {
                    if let Some((duration, func)) = easing {
                        if c.get() {
                            let func = func.clone();
                            _anim_handle = easing_var.ease(value.clone(), *duration, move |t| func(t));
                            return;
                        }
                    }
                }

                let (duration, func) = &default_easing;
                let func = func.clone();
                _anim_handle = easing_var.ease(value.clone(), *duration, move |t| func(t));
            })
            .perm();
            easing_var.read_only()
        }))
    }
}

fn var_bind<I, O, V>(
    input: &impl Var<I>,
    output: &V,
    update_output: impl FnMut(&I, &VarHookArgs, <V::Downgrade as WeakVar<O>>::Upgrade) + Send + 'static,
) -> VarHandle
where
    I: VarValue,
    O: VarValue,
    V: Var<O>,
{
    var_bind_ok(input, output.downgrade(), update_output)
}

fn var_bind_ok<I, O, W>(
    input: &impl Var<I>,
    wk_output: W,
    update_output: impl FnMut(&I, &VarHookArgs, W::Upgrade) + Send + 'static,
) -> VarHandle
where
    I: VarValue,
    O: VarValue,
    W: WeakVar<O>,
{
    let update_output = Mutex::new(update_output);
    input.hook(Box::new(move |args| {
        if let Some(output) = wk_output.upgrade() {
            if output.capabilities().contains(VarCapabilities::MODIFY) {
                if let Some(value) = args.downcast_value::<I>() {
                    update_output.lock()(value, args, output);
                }
            }
            true
        } else {
            false
        }
    }))
}

/// Variable for state properties (`is_*`, `has_*`).
///
/// State variables are `bool` probes that are set by the property, they are created automatically
/// by the property default when used in `when` expressions, but can be created manually.
///
/// # Examples
///
/// Example of manual usage to show a state as text:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, var::*, text::*};
/// # #[property(CONTEXT)]
/// # pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// # #[widget($crate::Text)]
/// # pub struct Text(widget_base::WidgetBase);
/// # #[property(CHILD, widget_impl(Text))]
/// # pub fn txt(child: impl UiNode, txt: impl IntoVar<Txt>) -> impl UiNode { child }
/// # fn main() {
/// # let _scope = zero_ui_core::app::App::minimal();
/// let probe = state_var();
/// # let _ =
/// Text! {
///     txt = probe.map(|p| formatx!("is_pressed = {p:?}"));
///     is_pressed = probe;
/// }
/// # ; }
/// ```
pub fn state_var() -> ArcVar<bool> {
    var(false)
}

/// Variable for getter properties (`get_*`, `actual_*`).
///
/// Getter variables are inited with a default value that is overridden by the property on node init and updated
/// by the property when the internal state they track changes. They are created automatically by the property
/// default when used in `when` expressions, but can be created manually.
///
/// # Examples
///
/// Example of manual usage to map the state to a color:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, var::*, text::*, color::*};
/// # #[property(CONTEXT)]
/// # pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// # #[widget($crate::Row)] pub struct Row(widget_base::WidgetBase);
/// # #[property(FILL)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
/// # fn main() {
/// # let _scope = zero_ui_core::app::App::minimal();
/// let probe = getter_var::<usize>();
/// # let _ =
/// Row! {
///     background_color = probe.map(|&i| {
///         let g = (i % 255) as u8;
///         rgb(g, g, g)
///     });
///     get_index = probe;
/// }
/// # ; }
/// ```
pub fn getter_var<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}

fn validate_getter_var<T: VarValue>(_var: &impl Var<T>) {
    #[cfg(debug_assertions)]
    if _var.capabilities().is_always_read_only() {
        tracing::error!("`is_`, `has_` or `get_` property inited with read-only var in {:?}", WIDGET.id());
    }
}

/// Helper for declaring state properties that depend on a single event.
pub fn event_is_state<A: EventArgs>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event: Event<A>,
    mut on_event: impl FnMut(&A) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event);
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = event.on(update) {
                if let Some(s) = on_event(args) {
                    let _ = state.set(s);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on two other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state2<A0, A1>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1);
    let mut partial = (default0, default1);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on three other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state3<A0, A1, A2>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    mut on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2);
    let mut partial = (default0, default1, default2);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event2.on(update) {
                if let Some(state) = on_event2(args) {
                    if partial.2 != state {
                        partial.2 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1, partial.2) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on four other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state4<A0, A1, A2, A3>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    mut on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    event3: Event<A3>,
    default3: bool,
    mut on_event3: impl FnMut(&A3) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
    A3: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2, default3);
    let mut partial = (default0, default1, default2, default3);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2).sub_event(&event3);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event2.on(update) {
                if let Some(state) = on_event2(args) {
                    if partial.2 != state {
                        partial.2 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event3.on(update) {
                if let Some(state) = on_event3(args) {
                    if partial.3 != state {
                        partial.3 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1, partial.2, partial.3) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create composite properties
/// that merge other state properties.
pub fn bind_is_state(child: impl UiNode, source: impl IntoVar<bool>, state: impl IntoVar<bool>) -> impl UiNode {
    let source = source.into_var();
    let state = state.into_var();
    let mut _binding = VarHandle::dummy();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            let _ = state.set_from(&source);
            _binding = source.bind(&state);
        }
        UiNodeOp::Deinit => {
            _binding = VarHandle::dummy();
        }
        _ => {}
    })
}

/// Helper for declaring state properties that are controlled by values in the widget state map.
///
/// The `predicate` closure is called with the widget state on init and every update, if the returned value changes the `state`
/// updates. The `deinit` closure is called on deinit to get the *reset* value.
pub fn widget_state_is_state(
    child: impl UiNode,
    predicate: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    deinit: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    state: impl IntoVar<bool>,
) -> impl UiNode {
    let state = state.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();
            let s = WIDGET.with_state(&deinit);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        _ => {}
    })
}

/// Helper for declaring state getter properties that are controlled by values in the widget state map.
///
/// The `get_new` closure is called with the widget state and current `state` every init and update, if it returns some value
/// the `state` updates. The `get_deinit` closure is called on deinit to get the *reset* value.
pub fn widget_state_get_state<T: VarValue>(
    child: impl UiNode,
    get_new: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    get_deinit: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    state: impl IntoVar<T>,
) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();

            let new = state.with(|s| WIDGET.with_state(|w| get_deinit(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        _ => {}
    })
}
