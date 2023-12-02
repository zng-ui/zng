use crate::{
    context::WIDGET,
    var::{IntoVar, Var, VarValue},
    widget_instance::{match_node, UiNode, UiNodeOp},
};

pub use zero_ui_state_map::*;

/// Helper for declaring properties that set the widget state.
///
/// The state ID is set in [`WIDGET`] on init and is kept updated. On deinit it is set to the `default` value.
///
/// # Examples
///
/// ```
/// # fn main() -> () { }
/// use zero_ui_core::{property, context::*, var::IntoVar, widget_instance::UiNode};
///
/// pub static FOO_ID: StaticStateId<u32> = StateId::new_static();
///
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     with_widget_state(child, &FOO_ID, || 0, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &mut impl UiNode) -> u32 {
///     widget.with_context(WidgetUpdateMode::Ignore, || WIDGET.get_state(&FOO_ID)).flatten().unwrap_or_default()
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner() -> u32 {
///     WIDGET.get_state(&FOO_ID).unwrap_or_default()
/// }
/// ```
pub fn with_widget_state<U, I, T>(child: U, id: impl Into<StateId<T>>, default: I, value: impl IntoVar<T>) -> impl UiNode
where
    U: UiNode,
    I: Fn() -> T + Send + 'static,
    T: StateValue + VarValue,
{
    #[cfg(dyn_closure)]
    let default: Box<dyn Fn() -> T + Send> = Box::new(default);
    with_widget_state_impl(child.cfg_boxed(), id.into(), default, value.into_var()).cfg_boxed()
}
fn with_widget_state_impl<U, I, T>(child: U, id: impl Into<StateId<T>>, default: I, value: impl IntoVar<T>) -> impl UiNode
where
    U: UiNode,
    I: Fn() -> T + Send + 'static,
    T: StateValue + VarValue,
{
    let id = id.into();
    let value = value.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();
            WIDGET.sub_var(&value);
            WIDGET.set_state(id, value.get());
        }
        UiNodeOp::Deinit => {
            child.deinit();
            WIDGET.set_state(id, default());
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            if let Some(v) = value.get_new() {
                WIDGET.set_state(id, v);
            }
        }
        _ => {}
    })
}

/// Helper for declaring properties that set the widget state with a custom closure.
///
/// The `default` closure is used to init the state value, then the `modify` closure is used to modify the state using the variable value.
///
/// On deinit the `default` value is set on the state again.
///
/// See [`with_widget_state`] for more details.
pub fn with_widget_state_modify<U, S, V, I, M>(
    child: U,
    id: impl Into<StateId<S>>,
    value: impl IntoVar<V>,
    default: I,
    modify: M,
) -> impl UiNode
where
    U: UiNode,
    S: StateValue,
    V: VarValue,
    I: Fn() -> S + Send + 'static,
    M: FnMut(&mut S, &V) + Send + 'static,
{
    #[cfg(dyn_closure)]
    let default: Box<dyn Fn() -> S + Send> = Box::new(default);
    #[cfg(dyn_closure)]
    let modify: Box<dyn FnMut(&mut S, &V) + Send> = Box::new(modify);

    with_widget_state_modify_impl(child.cfg_boxed(), id.into(), value.into_var(), default, modify)
}
fn with_widget_state_modify_impl<U, S, V, I, M>(
    child: U,
    id: impl Into<StateId<S>>,
    value: impl IntoVar<V>,
    default: I,
    mut modify: M,
) -> impl UiNode
where
    U: UiNode,
    S: StateValue,
    V: VarValue,
    I: Fn() -> S + Send + 'static,
    M: FnMut(&mut S, &V) + Send + 'static,
{
    let id = id.into();
    let value = value.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            WIDGET.sub_var(&value);

            value.with(|v| {
                WIDGET.with_state_mut(|mut s| {
                    modify(s.entry(id).or_insert_with(&default), v);
                })
            })
        }
        UiNodeOp::Deinit => {
            child.deinit();

            WIDGET.set_state(id, default());
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            value.with_new(|v| {
                WIDGET.with_state_mut(|mut s| {
                    modify(s.req_mut(id), v);
                })
            });
        }
        _ => {}
    })
}
