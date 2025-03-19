//! Helper nodes.
//!
//! This module defines some foundational nodes that can be used for declaring properties and widgets.

use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use zng_app::{
    event::{Command, CommandArgs, CommandHandle, CommandScope, Event, EventArgs},
    handler::WidgetHandler,
    render::{FrameBuilder, FrameValueKey},
    update::WidgetUpdates,
    widget::{
        VarLayout, WIDGET, WidgetUpdateMode,
        border::{BORDER, BORDER_ALIGN_VAR, BORDER_OVER_VAR},
        info::Interactivity,
        node::*,
    },
    window::WINDOW,
};
use zng_app_context::{ContextLocal, LocalContext};
use zng_layout::{
    context::LAYOUT,
    unit::{PxConstraints2d, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize, PxVector, SideOffsets},
};
use zng_state_map::{StateId, StateMapRef, StateValue};
use zng_var::{types::VecChange, *};

#[doc(hidden)]
pub use pastey::paste;

#[doc(hidden)]
pub use zng_app;

/// Helper for declaring properties that sets a context var.
///
/// The generated [`UiNode`] delegates each method to `child` inside a call to [`ContextVar::with_context`].
///
/// # Examples
///
/// A simple context property declaration:
///
/// ```
/// # fn main() -> () { }
/// # use zng_app::{*, widget::{node::*, *}};
/// # use zng_var::*;
/// # use zng_wgt::node::*;
/// #
/// context_var! {
///     pub static FOO_VAR: u32 = 0u32;
/// }
///
/// /// Sets the [`FOO_VAR`] in the widgets and its content.
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
/// # use zng_app::{*, widget::{node::*, *}};
/// # use zng_var::*;
/// # use zng_wgt::node::*;
/// #
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
/// [`is_new`]: zng_var::AnyVar::is_new
/// [`read_only`]: zng_var::Var::read_only
/// [`actual_var`]: zng_var::Var::actual_var
/// [`default`]: zng_app::widget::property#default
/// [`merge_var!`]: zng_var::merge_var
/// [`UiNode`]: zng_app::widget::node::UiNode
/// [`ContextVar::with_context`]: zng_var::ContextVar::with_context
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
///
/// [`ContextVar<T>`]: zng_var::ContextVar
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

///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event` and `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zng_app::event::*;
/// # use zng_wgt::node::*;
/// # #[derive(Clone, Debug, PartialEq)] pub enum KeyState { Pressed }
/// # event_args! { pub struct KeyInputArgs { pub state: KeyState, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # event! { pub static KEY_INPUT_EVENT: KeyInputArgs; }
/// event_property! {
///     /// on_key_input docs.
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         // default filter is |args| true,
///     }
///
///     pub(crate) fn key_down {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |args| args.state == KeyState::Pressed,
///     }
/// }
/// ```
///
/// # Filter
///
/// App events are delivered to all widgets that are both in the [`UpdateDeliveryList`] and event subscribers list,
/// event properties can specialize further by defining a filter predicate.
///
/// The `filter:` predicate is called if [`propagation`] is not stopped. It must return `true` if the event arguments
/// are relevant in the context of the widget and event property. If it returns `true` the `handler` closure is called.
/// See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default always allows, so all app events targeting the widget and not already handled
/// are allowed by default. Note that events that represent an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`]
/// widgets, event properties should probably distinguish if they fire on normal interactions versus on *disabled* interactions.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
///
/// # Commands
///
/// You can use [`command_property`] to declare command event properties.
///
/// # Implement For
///
/// You can implement the new properties for a widget or mix-in using the `widget_impl:` directive:
///
/// ```
/// # fn main() { }
/// # use zng_app::{event::*, widget::{node::UiNode, widget_mixin}};
/// # use zng_wgt::node::*;
/// # event_args! { pub struct KeyInputArgs { .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
/// # event! { pub static KEY_INPUT_EVENT: KeyInputArgs; }
/// # fn some_node(child: impl UiNode) -> impl UiNode { child }
/// /// Keyboard events.
/// #[widget_mixin]
/// pub struct KeyboardMix<P>(P);
///
/// event_property! {
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         widget_impl: KeyboardMix<P>,
///     }
/// }
/// ```
///
/// # With Extra Nodes
///
/// You can wrap the event handler node with extra nodes by setting the optional `with:` closure:
///
/// ```
/// # fn main() { }
/// # use zng_app::{event::*, widget::node::UiNode};
/// # use zng_wgt::node::*;
/// # event_args! { pub struct KeyInputArgs { .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
/// # event! { pub static KEY_INPUT_EVENT: KeyInputArgs; }
/// # fn some_node(child: impl UiNode) -> impl UiNode { child }
/// event_property! {
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         with: |child, _preview| some_node(child),
///     }
/// }
/// ```
///
/// The closure receives two arguments, the handler `UiNode` and a `bool` that is `true` if the closure is called in the *on_pre_*
/// property or `false` when called in the *on_* property.
///
/// [`on_pre_event`]: crate::node::on_pre_event
/// [`on_event`]: crate::node::on_event
/// [`propagation`]: zng_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zng_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
/// [`UpdateDeliveryList`]: zng_app::update::UpdateDeliveryList
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path
            $(, filter: $filter:expr)?
            $(, widget_impl: $Wgt:ty)?
            $(, with: $with:expr)?
            $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            done {
                sig { $(#[$on_event_attrs])* $vis fn $event { event: $EVENT, args: $Args, } }
            }

            $(filter: $filter,)?
            $(widget_impl: $Wgt,)?
            $(with: $with,)?
        }
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    // match filter:
    (
        done {
            $($done:tt)+
        }
        filter: $filter:expr,
        $($rest:tt)*
    ) => {
        $crate::__event_property! {
            done {
                $($done)+
                filter { $filter }
            }
            $($rest)*
        }
    };
    // match widget_impl:
    (
        done {
            $($done:tt)+
        }
        widget_impl: $Wgt:ty,
        $($rest:tt)*
    ) => {
        $crate::__event_property! {
            done {
                $($done)+
                widget_impl { , widget_impl($Wgt) }
            }
            $($rest)*
        }
    };
    // match with:
    (
        done {
            $($done:tt)+
        }
        with: $with:expr,
    ) => {
        $crate::__event_property! {
            done {
                $($done)+
                with { $with }
            }
        }
    };
    // match done sig
    (
        done {
            sig { $($sig:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { |_args| true }
                widget_impl { }
                with { }
            }
        }
    };
    // match done sig+filter
    (
        done {
            sig { $($sig:tt)+ }
            filter { $($filter:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { $($filter)+ }
                widget_impl { }
                with { }
            }
        }
    };
    // match done sig+widget_impl
    (
        done {
            sig { $($sig:tt)+ }
            widget_impl { $($widget_impl:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { |_args| true }
                widget_impl { $($widget_impl)+ }
                with { }
            }
        }
    };
    // match done sig+with
    (
        done {
            sig { $($sig:tt)+ }
            with { $($with:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { |_args| true }
                widget_impl { }
                with { $($with)+ }
            }
        }
    };
    // match done sig+filter+widget_impl
    (
        done {
            sig { $($sig:tt)+ }
            filter { $($filter:tt)+ }
            widget_impl { $($widget_impl:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { $($filter)+ }
                widget_impl { $($widget_impl)+ }
                with { }
            }
        }
    };
    // match done sig+filter+with
    (
        done {
            sig { $($sig:tt)+ }
            filter { $($filter:tt)+ }
            with { $($with:tt)+ }
        }
    ) => {
        $crate::__event_property! {
            done {
                sig { $($sig)+ }
                filter { $($filter)+ }
                widget_impl { }
                with { $($with)+ }
            }
        }
    };
    // match done sig+filter+widget_impl+with
    (
        done {
            sig { $(#[$on_event_attrs:meta])* $vis:vis fn $event:ident { event: $EVENT:path, args: $Args:path, } }
            filter { $filter:expr }
            widget_impl { $($widget_impl:tt)* }
            with { $($with:expr)? }
        }
    ) => {
        $crate::node::paste! {
            $(#[$on_event_attrs])*
            ///
            /// # Preview
            ///
            #[doc = "You can preview this event using [`on_pre_"$event "`](fn.on_pre_"$event ".html)."]
            /// Otherwise the handler is only called after the widget content has a chance to stop propagation.
            ///
            /// # Async
            ///
            /// You can use async event handlers with this property.
            #[$crate::node::zng_app::widget::property(
                EVENT,
                default( $crate::node::zng_app::handler::hn!(|_|{}) )
                $($widget_impl)*
            )]
            $vis fn [<on_ $event>](
                child: impl $crate::node::zng_app::widget::node::UiNode,
                handler: impl $crate::node::zng_app::handler::WidgetHandler<$Args>,
            ) -> impl $crate::node::zng_app::widget::node::UiNode {
                $crate::__event_property!(=> with($crate::node::on_event(child, $EVENT, $filter, handler), false, $($with)?))
            }

            #[doc = "Preview [`on_"$event "`](fn.on_"$event ".html) event."]
            ///
            /// # Preview
            ///
            /// Preview event properties call the handler before the main event property and before the widget content, if you stop
            /// the propagation of a preview event the main event handler is not called.
            ///
            /// # Async
            ///
            /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
            /// subsequent code runs in widget updates.
            #[$crate::node::zng_app::widget::property(
                EVENT,
                default( $crate::node::zng_app::handler::hn!(|_|{}) )
                $($widget_impl)*
            )]
            $vis fn [<on_pre_ $event>](
                child: impl $crate::node::zng_app::widget::node::UiNode,
                handler: impl $crate::node::zng_app::handler::WidgetHandler<$Args>,
            ) -> impl $crate::node::zng_app::widget::node::UiNode {
                $crate::__event_property!(=> with($crate::node::on_pre_event(child, $EVENT, $filter, handler), true, $($with)?))
            }
        }
    };

    (=> with($child:expr, $preview:expr,)) => { $child };
    (=> with($child:expr, $preview:expr, $with:expr)) => { ($with)($child, $preview) };
}

/// Helper for declaring event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// # Commands
///
/// You can use [`on_command`] to declare command event properties.
///
/// [`propagation`]: zng_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zng_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
pub fn on_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(feature = "dyn_closure")]
    let filter: Box<dyn FnMut(&A) -> bool + Send> = Box::new(filter);
    on_event_impl(child.cfg_boxed(), event, filter, handler.cfg_boxed()).cfg_boxed()
}
fn on_event_impl<C, A, F, H>(child: C, event: Event<A>, mut filter: F, mut handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&event);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = event.on(update) {
                if !args.propagation().is_stopped() && filter(args) {
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    let t = std::time::Instant::now();
                    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
                    let t = web_time::Instant::now();

                    handler.event(args);

                    #[cfg(debug_assertions)]
                    {
                        let t = t.elapsed();
                        if t > std::time::Duration::from_millis(300) {
                            tracing::warn!(
                                "event handler for `{}` in {:?} blocked for {t:?}, consider using `async_hn!`",
                                event.as_any().name(),
                                WIDGET.id()
                            );
                        }
                    }
                }
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            handler.update();
        }
        _ => {}
    })
}

/// Helper for declaring preview event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent event updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// # Commands
///
/// You can use [`on_pre_command`] to declare command event properties.
///
/// [`propagation`]: zng_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zng_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
pub fn on_pre_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(feature = "dyn_closure")]
    let filter: Box<dyn FnMut(&A) -> bool + Send> = Box::new(filter);
    on_pre_event_impl(child.cfg_boxed(), event, filter, handler.cfg_boxed()).cfg_boxed()
}
fn on_pre_event_impl<C, A, F, H>(child: C, event: Event<A>, mut filter: F, mut handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&event);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = event.on(update) {
                if !args.propagation().is_stopped() && filter(args) {
                    #[cfg(debug_assertions)]
                    let t = std::time::Instant::now();

                    handler.event(args);

                    #[cfg(debug_assertions)]
                    {
                        let t = t.elapsed();
                        if t > std::time::Duration::from_millis(300) {
                            tracing::warn!(
                                "preview event handler for `{}` in {:?} blocked for {t:?}, consider using `async_hn!`",
                                event.as_any().name(),
                                WIDGET.id()
                            );
                        }
                    }
                }
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();
        }
        _ => {}
    })
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_property {
    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd { $cmd_init:expr }
            enabled { $enabled_var:expr }
            widget_impl { $($widget_impl:tt)* }
        }
    ) => { $crate::node::paste! {
        $(#[$on_cmd_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this command event using [`on_pre_"$command "`](fn.on_pre_"$command ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance to stop propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::node::zng_app::widget::property(EVENT, default( $crate::node::zng_app::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $command>](
            child: impl $crate::node::zng_app::widget::node::UiNode,
            handler: impl $crate::node::zng_app::handler::WidgetHandler<$crate::node::zng_app::event::CommandArgs>,
        ) -> impl $crate::node::zng_app::widget::node::UiNode {
            $crate::node::on_command(child, || $cmd_init, || $enabled_var, handler)
        }

        #[doc = "Preview [`on_"$command "`](fn.on_"$command ".html) command."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::node::zng_app::widget::property(EVENT, default( $crate::node::zng_app::handler::hn!(|_|{}) ) $($widget_impl)*)]
        $vis fn [<on_pre_ $command>](
            child: impl $crate::node::zng_app::widget::node::UiNode,
            handler: impl $crate::node::zng_app::handler::WidgetHandler<$crate::node::zng_app::event::CommandArgs>,
        ) -> impl $crate::node::zng_app::widget::node::UiNode {
            $crate::node::on_pre_command(child, || $cmd_init, || $enabled_var, handler)
        }
    } };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd { $cmd_init:expr }
            enabled { $enabled_var:expr }
            widget_impl_ty { $Wgt:ty }
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd { $cmd_init }
                enabled { $enabled_var }
                widget_impl { , widget_impl($Wgt) }
            }
        }
    };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd { $cmd_init:expr }
            widget_impl_ty { $Wgt:ty }
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd { $cmd_init }
                enabled { $crate::node::zng_app::var::LocalVar(true) }
                widget_impl { , widget_impl($Wgt) }
            }
        }
    };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd { $cmd_init:expr }
            enabled { $enabled_var:expr }
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd { $cmd_init }
                enabled { $enabled_var }
                widget_impl { }
            }
        }
    };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd { $cmd_init:expr }
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd { $cmd_init }
                enabled { $crate::node::zng_app::var::LocalVar(true) }
                widget_impl { }
            }
        }
    };
}

///<span data-del-macro-root></span> Declare one or more command event properties.
///
/// Each declaration expands to two properties `on_$command` and `on_pre_$command`.
/// The preview properties call [`on_pre_command`], the main event properties call [`on_command`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zng_app::{event::*, widget::*};
/// # use zng_app::var::*;
/// # use zng_wgt::node::*;
/// # command! {
/// #   pub static PASTE_CMD;
/// # }
/// command_property! {
///     /// Paste command property docs.
///     pub fn paste {
///         cmd: PASTE_CMD.scoped(WIDGET.id()),
///         // enabled: LocalVar(true), // default enabled
///     }
/// }
/// ```
///
/// # Command
///
/// The `cmd:` expression evaluates on init to generate the command, this allows for the
/// creation of widget scoped commands. The new command property event handler will receive events
/// for the command and scope that target the widget where the property is set.
///
/// If the command is scoped on the root widget and the command property is set on the same root widget a second handle
/// is taken for the window scope too, so callers can target the *window* using the window ID or the root widget ID.
///
/// # Enabled
///
/// The `enabled:` expression evaluates on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// If not provided the command is always enabled.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
///
/// # Implement For
///
/// You can implement the new properties for a widget or mix-in using the `widget_impl:` directive:
///
/// ```
/// # fn main() { }
/// # use zng_wgt::node::*;
/// # use zng_app::{event::*, widget::*};
/// # use zng_app::var::*;
/// # use zng_wgt::node::*;
/// # command! {
/// #   pub static PASTE_CMD;
/// # }
/// /// Clipboard handler.
/// #[widget_mixin]
/// pub struct ClipboardMix<P>(P);
///
/// command_property! {
///     /// Paste command property docs.
///     pub fn paste {
///         cmd: PASTE_CMD.scoped(WIDGET.id()),
///         widget_impl: ClipboardMix<P>,
///     }
/// }
/// ```
///
/// [`Command::subscribe`]: zng_app::event::Command::subscribe
#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr
            $(, enabled: $enabled_var:expr)?
            $(, widget_impl: $Wgt:ty)?
            $(,)?
        }
    )+) => {$(
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd { $cmd_init }
                $( enabled { $enabled_var } )?
                $( widget_impl_ty { $Wgt } )?
            }
        }
    )+};
}

/// Helper for declaring command event properties.
///
/// This function is used by the [`command_property!`] macro.
///
/// # Command
///
/// The `command_builder` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event `handler` will receive events for the command
/// and scope that target the widget where it is set.
///
/// If the command is scoped on the root widget and `on_command` is set on the same root widget a second handle
/// is taken for the window scope too, so callers can target the *window* using the window ID or the root widget ID.
///
/// # Enabled
///
/// The `enabled_builder` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// Note that the command handler can be enabled even when the widget is disabled, the widget
/// will receive the event while disabled in this case, you can use this to show feedback explaining
/// why the command cannot run.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_command`] equivalent at the same context level. If the command
/// event targets more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///  
/// [`propagation`]: zng_app::event::AnyEventArgs::propagation
/// [`Command::subscribe`]: zng_app::event::Command::subscribe
pub fn on_command<U, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    #[cfg(feature = "dyn_closure")]
    let command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(command_builder);
    #[cfg(feature = "dyn_closure")]
    let enabled_builder: Box<dyn FnMut() -> E + Send> = Box::new(enabled_builder);

    on_command_impl(child.boxed(), command_builder, enabled_builder, handler.cfg_boxed()).cfg_boxed()
}
fn on_command_impl<U, CB, E, EB, H>(child: U, mut command_builder: CB, mut enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    let mut enabled = None;
    let mut handle = CommandHandle::dummy();
    let mut win_handle = CommandHandle::dummy();
    let mut command = NIL_CMD;

    let mut handler = handler.cfg_boxed();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            command = command_builder();

            let id = WIDGET.id();
            handle = command.subscribe_wgt(is_enabled, id);
            if CommandScope::Widget(id) == command.scope() && WIDGET.parent_id().is_none() {
                // root scope, also include the window.
                win_handle = command.scoped(WINDOW.id()).subscribe_wgt(is_enabled, id);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = CommandHandle::dummy();
            win_handle = CommandHandle::dummy();
            command = NIL_CMD;
        }

        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = command.on_unhandled(update) {
                handler.event(args);
            } else if !win_handle.is_dummy() {
                if let Some(args) = command.scoped(WINDOW.id()).on_unhandled(update) {
                    handler.event(args);
                }
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("node not inited").get_new() {
                handle.set_enabled(enabled);
                win_handle.set_enabled(enabled);
            }
        }

        _ => {}
    })
}

zng_app::event::command! {
    static NIL_CMD;
}

/// Helper for declaring command preview handlers.
///
/// Other then the route this helper behaves exactly like [`on_command`].
///
/// # Route
///
/// The event `handler` is called before the [`on_command`] equivalent at the same context level. If the command event
/// targets more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
pub fn on_pre_command<U, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    #[cfg(feature = "dyn_closure")]
    let command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(command_builder);
    #[cfg(feature = "dyn_closure")]
    let enabled_builder: Box<dyn FnMut() -> E + Send> = Box::new(enabled_builder);

    on_pre_command_impl(child.cfg_boxed(), command_builder, enabled_builder, handler.cfg_boxed()).cfg_boxed()
}
fn on_pre_command_impl<U, CB, E, EB, H>(child: U, mut command_builder: CB, mut enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    let mut handler = handler.cfg_boxed();

    let mut enabled = None;
    let mut handle = CommandHandle::dummy();
    let mut win_handle = CommandHandle::dummy();
    let mut command = NIL_CMD;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            command = command_builder();

            let id = WIDGET.id();
            handle = command.subscribe_wgt(is_enabled, id);
            if CommandScope::Widget(id) == command.scope() && WIDGET.parent_id().is_none() {
                // root scope, also include the window.
                win_handle = command.scoped(WINDOW.id()).subscribe_wgt(is_enabled, id);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = CommandHandle::dummy();
            win_handle = CommandHandle::dummy();
            command = NIL_CMD;
        }

        UiNodeOp::Event { update } => {
            if let Some(args) = command.on_unhandled(update) {
                handler.event(args);
            } else if !win_handle.is_dummy() {
                if let Some(args) = command.scoped(WINDOW.id()).on_unhandled(update) {
                    handler.event(args);
                }
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("on_pre_command not initialized").get_new() {
                handle.set_enabled(enabled);
                win_handle.set_enabled(enabled);
            }
        }

        _ => {}
    })
}

/// Logs an error if the `_var` is always read-only.
pub fn validate_getter_var<T: VarValue>(_var: &impl Var<T>) {
    #[cfg(debug_assertions)]
    if _var.capabilities().is_always_read_only() {
        tracing::error!(
            "`is_`, `has_` or `get_` property inited with read-only var in `{}`",
            WIDGET.trace_id()
        );
    }
}

/// Helper for declaring state properties that depend on a single event.
///
/// When the `event` is received `on_event` is called, if it provides a new state the `state` variable is set.
pub fn event_state<A: EventArgs, S: VarValue>(
    child: impl UiNode,
    state: impl IntoVar<S>,
    default: S,
    event: Event<A>,
    mut on_event: impl FnMut(&A) -> Option<S> + Send + 'static,
) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event);
            let _ = state.set(default.clone());
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default.clone());
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
///
/// When the `event#` is received `on_event#` is called, if it provides a new value `merge` is called, if merge
/// provides a new value the `state` variable is set.
#[expect(clippy::too_many_arguments)]
pub fn event_state2<A0, A1, S0, S1, S>(
    child: impl UiNode,
    state: impl IntoVar<S>,
    default: S,
    event0: Event<A0>,
    default0: S0,
    mut on_event0: impl FnMut(&A0) -> Option<S0> + Send + 'static,
    event1: Event<A1>,
    default1: S1,
    mut on_event1: impl FnMut(&A1) -> Option<S1> + Send + 'static,
    mut merge: impl FnMut(S0, S1) -> Option<S> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    S0: VarValue,
    S1: VarValue,
    S: VarValue,
{
    let state = state.into_var();
    let partial_default = (default0, default1);
    let mut partial = partial_default.clone();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1);

            partial = partial_default.clone();
            let _ = state.set(default.clone());
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default.clone());
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
                if let Some(value) = merge(partial.0.clone(), partial.1.clone()) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on three other event states.
///
/// When the `event#` is received `on_event#` is called, if it provides a new value `merge` is called, if merge
/// provides a new value the `state` variable is set.
#[expect(clippy::too_many_arguments)]
pub fn event_state3<A0, A1, A2, S0, S1, S2, S>(
    child: impl UiNode,
    state: impl IntoVar<S>,
    default: S,
    event0: Event<A0>,
    default0: S0,
    mut on_event0: impl FnMut(&A0) -> Option<S0> + Send + 'static,
    event1: Event<A1>,
    default1: S1,
    mut on_event1: impl FnMut(&A1) -> Option<S1> + Send + 'static,
    event2: Event<A2>,
    default2: S2,
    mut on_event2: impl FnMut(&A2) -> Option<S2> + Send + 'static,
    mut merge: impl FnMut(S0, S1, S2) -> Option<S> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
    S0: VarValue,
    S1: VarValue,
    S2: VarValue,
    S: VarValue,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2);
    let mut partial = partial_default.clone();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2);

            partial = partial_default.clone();
            let _ = state.set(default.clone());
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default.clone());
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
                if let Some(value) = merge(partial.0.clone(), partial.1.clone(), partial.2.clone()) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on four other event states.
///
/// When the `event#` is received `on_event#` is called, if it provides a new value `merge` is called, if merge
/// provides a new value the `state` variable is set.
#[expect(clippy::too_many_arguments)]
pub fn event_state4<A0, A1, A2, A3, S0, S1, S2, S3, S>(
    child: impl UiNode,
    state: impl IntoVar<S>,
    default: S,
    event0: Event<A0>,
    default0: S0,
    mut on_event0: impl FnMut(&A0) -> Option<S0> + Send + 'static,
    event1: Event<A1>,
    default1: S1,
    mut on_event1: impl FnMut(&A1) -> Option<S1> + Send + 'static,
    event2: Event<A2>,
    default2: S2,
    mut on_event2: impl FnMut(&A2) -> Option<S2> + Send + 'static,
    event3: Event<A3>,
    default3: S3,
    mut on_event3: impl FnMut(&A3) -> Option<S3> + Send + 'static,
    mut merge: impl FnMut(S0, S1, S2, S3) -> Option<S> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
    A3: EventArgs,
    S0: VarValue,
    S1: VarValue,
    S2: VarValue,
    S3: VarValue,
    S: VarValue,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2, default3);
    let mut partial = partial_default.clone();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2).sub_event(&event3);

            partial = partial_default.clone();
            let _ = state.set(default.clone());
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default.clone());
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
                if let Some(value) = merge(partial.0.clone(), partial.1.clone(), partial.2.clone(), partial.3.clone()) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create state properties
/// that map from a context variable or to create composite properties that merge other state properties.
pub fn bind_state<T: VarValue>(child: impl UiNode, source: impl IntoVar<T>, state: impl IntoVar<T>) -> impl UiNode {
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

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` closure is called to provide a variable, the variable is set to `source` and bound to it,
/// you can use this to create state properties that map from a context variable or to create composite properties
/// that merge other state properties.
pub fn bind_state_init<T, V>(child: impl UiNode, source: impl Fn() -> V + Send + 'static, state: impl IntoVar<T>) -> impl UiNode
where
    T: VarValue,
    V: Var<T>,
{
    let state = state.into_var();
    let mut _source_var = None;
    let mut _binding = VarHandle::dummy();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            let source = source();
            let _ = state.set_from(&source);
            _binding = source.bind(&state);
            _source_var = Some(source);
        }
        UiNodeOp::Deinit => {
            _binding = VarHandle::dummy();
            _source_var = None;
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

/// Transforms and clips the `content` node according with the default widget border align behavior.
///
/// Properties that *fill* the widget can wrap their fill content in this node to automatically implement
/// the expected interaction with the widget borders, the content will be positioned, sized and clipped according to the
/// widget borders, corner radius and border align.
pub fn fill_node(content: impl UiNode) -> impl UiNode {
    let mut clip_bounds = PxSize::zero();
    let mut clip_corners = PxCornerRadius::zero();

    let mut offset = PxVector::zero();
    let offset_key = FrameValueKey::new_unique();
    let mut define_frame = false;

    match_node(content, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&BORDER_ALIGN_VAR);
            define_frame = false;
            offset = PxVector::zero();
        }
        UiNodeOp::Measure { desired_size, .. } => {
            let offsets = BORDER.inner_offsets();
            let align = BORDER_ALIGN_VAR.get();

            let our_offsets = offsets * align;
            let size_offset = offsets - our_offsets;

            let size_increase = PxSize::new(size_offset.horizontal(), size_offset.vertical());

            *desired_size = LAYOUT.constraints().fill_size() + size_increase;
        }
        UiNodeOp::Layout { wl, final_size } => {
            // We are inside the *inner* bounds AND inside border_nodes:
            //
            // .. ( layout ( new_border/inner ( border_nodes ( FILL_NODES ( new_child_context ( new_child_layout ( ..

            let (bounds, corners) = BORDER.fill_bounds();

            let mut new_offset = bounds.origin.to_vector();

            if clip_bounds != bounds.size || clip_corners != corners {
                clip_bounds = bounds.size;
                clip_corners = corners;
                WIDGET.render();
            }

            let (_, branch_offset) = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(bounds.size), || {
                wl.with_branch_child(|wl| child.layout(wl))
            });
            new_offset += branch_offset;

            if offset != new_offset {
                offset = new_offset;

                if define_frame {
                    WIDGET.render_update();
                } else {
                    define_frame = true;
                    WIDGET.render();
                }
            }

            *final_size = bounds.size;
        }
        UiNodeOp::Render { frame } => {
            let mut render = |frame: &mut FrameBuilder| {
                let bounds = PxRect::from_size(clip_bounds);
                frame.push_clips(
                    |c| {
                        if clip_corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, clip_corners, false, false);
                        } else {
                            c.push_clip_rect(bounds, false, false);
                        }

                        if let Some(inline) = WIDGET.bounds().inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, false);
                            }
                        }
                    },
                    |f| child.render(f),
                );
            };

            if define_frame {
                frame.push_reference_frame(offset_key.into(), offset_key.bind(offset.into(), false), true, false, |frame| {
                    render(frame);
                });
            } else {
                render(frame);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            if define_frame {
                update.with_transform(offset_key.update(offset.into(), false), false, |update| {
                    child.render_update(update);
                });
            } else {
                child.render_update(update);
            }
        }
        _ => {}
    })
}

/// Creates a border node that delegates rendering to a `border_visual` and manages the `border_offsets` coordinating
/// with the other borders of the widget.
///
/// This node disables inline layout for the widget.
pub fn border_node(child: impl UiNode, border_offsets: impl IntoVar<SideOffsets>, border_visual: impl UiNode) -> impl UiNode {
    let offsets = border_offsets.into_var();
    let mut render_offsets = PxSideOffsets::zero();
    let mut border_rect = PxRect::zero();

    match_node_list(ui_vec![child, border_visual], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offsets).sub_var_render(&BORDER_OVER_VAR);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let offsets = offsets.layout();
            *desired_size = BORDER.measure_border(offsets, || {
                LAYOUT.with_sub_size(PxSize::new(offsets.horizontal(), offsets.vertical()), || {
                    children.with_node(0, |n| wm.measure_block(n))
                })
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            // We are inside the *inner* bounds or inside a parent border_node:
            //
            // .. ( layout ( new_border/inner ( BORDER_NODES ( fill_nodes ( new_child_context ( new_child_layout ( ..
            //
            // `wl` is targeting the child transform, child nodes are naturally inside borders, so we
            // need to add to the offset and take the size, fill_nodes optionally cancel this transform.

            let offsets = offsets.layout();
            if render_offsets != offsets {
                render_offsets = offsets;
                WIDGET.render();
            }

            let parent_offsets = BORDER.inner_offsets();
            let origin = PxPoint::new(parent_offsets.left, parent_offsets.top);
            if border_rect.origin != origin {
                border_rect.origin = origin;
                WIDGET.render();
            }

            // layout child and border visual
            BORDER.layout_border(offsets, || {
                wl.translate(PxVector::new(offsets.left, offsets.top));

                let taken_size = PxSize::new(offsets.horizontal(), offsets.vertical());
                border_rect.size = LAYOUT.with_sub_size(taken_size, || children.with_node(0, |n| n.layout(wl)));

                // layout border visual
                LAYOUT.with_constraints(PxConstraints2d::new_exact_size(border_rect.size), || {
                    BORDER.with_border_layout(border_rect, offsets, || {
                        children.with_node(1, |n| n.layout(wl));
                    });
                });
            });

            *final_size = border_rect.size;
        }
        UiNodeOp::Render { frame } => {
            if BORDER_OVER_VAR.get() {
                children.with_node(0, |c| c.render(frame));
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.with_node(1, |c| c.render(frame));
                });
            } else {
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.with_node(1, |c| c.render(frame));
                });
                children.with_node(0, |c| c.render(frame));
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            children.with_node(0, |c| c.render_update(update));
            BORDER.with_border_layout(border_rect, render_offsets, || {
                children.with_node(1, |c| c.render_update(update));
            })
        }
        _ => {}
    })
}

/// Helper for declaring nodes that sets a context local value.
///
/// See [`context_local!`] for more details about contextual values.
///
/// [`context_local!`]: crate::prelude::context_local
pub fn with_context_local<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    value: impl Into<T>,
) -> impl UiNode {
    let mut value = Some(Arc::new(value.into()));

    match_node(child, move |child, op| {
        context.with_context(&mut value, || child.op(op));
    })
}

/// Helper for declaring nodes that sets a context local value generated on init.
///
/// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextLocal<T>`]
/// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
///
/// Apart from the value initialization this behaves just like [`with_context_local`].
///
/// [`ContextLocal<T>`]: zng_app_context::ContextLocal
pub fn with_context_local_init<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    init_value: impl FnMut() -> T + Send + 'static,
) -> impl UiNode {
    #[cfg(feature = "dyn_closure")]
    let init_value: Box<dyn FnMut() -> T + Send> = Box::new(init_value);
    with_context_local_init_impl(child.cfg_boxed(), context, init_value).cfg_boxed()
}
fn with_context_local_init_impl<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    mut init_value: impl FnMut() -> T + Send + 'static,
) -> impl UiNode {
    let mut value = None;

    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                value = Some(Arc::new(init_value()));
            }
            UiNodeOp::Deinit => {
                is_deinit = true;
            }
            _ => {}
        }

        context.with_context(&mut value, || child.op(op));

        if is_deinit {
            value = None;
        }
    })
}

/// Helper for declaring widgets that are recontextualized to take in some of the context
/// of an *original* parent.
///
/// See [`LocalContext::with_context_blend`] for more details about `over`. The returned
/// node will delegate all node operations to inside the blend. The [`UiNode::with_context`]
/// will delegate to the `child` widget context, but the `ctx` is not blended for this method, only
/// for [`UiNodeOp`] methods.
///
/// # Warning
///
/// Properties, context vars and context locals are implemented with the assumption that all consumers have
/// released the context on return, that is even if the context was shared with worker threads all work was block-waited.
/// This node breaks this assumption, specially with `over: true` you may cause unexpected behavior if you don't consider
/// carefully what context is being captured and what context is being replaced.
///
/// As a general rule, only capture during init or update in [`NestGroup::CHILD`], only wrap full widgets and only place the wrapped
/// widget in a parent's [`NestGroup::CHILD`] for a parent that has no special expectations about the child.
///
/// As an example of things that can go wrong, if you capture during layout, the `LAYOUT` context is captured
/// and replaces `over` the actual layout context during all subsequent layouts in the actual parent.
///
/// # Panics
///
/// Panics during init if `ctx` is not from the same app as the init context.
///
/// [`NestGroup::CHILD`]: zng_app::widget::builder::NestGroup::CHILD
/// [`UiNodeOp`]: zng_app::widget::node::UiNodeOp
/// [`UiNode::with_context`]: zng_app::widget::node::UiNode::with_context
/// [`LocalContext::with_context_blend`]: zng_app_context::LocalContext::with_context_blend
pub fn with_context_blend(mut ctx: LocalContext, over: bool, child: impl UiNode) -> impl UiNode {
    match_widget(child, move |c, op| {
        if let UiNodeOp::Init = op {
            let init_app = LocalContext::current_app();
            ctx.with_context_blend(over, || {
                let ctx_app = LocalContext::current_app();
                assert_eq!(init_app, ctx_app);
                c.op(op)
            });
        } else {
            ctx.with_context_blend(over, || c.op(op));
        }
    })
}

/// Helper for declaring properties that set the widget state.
///
/// The state ID is set in [`WIDGET`] on init and is kept updated. On deinit it is set to the `default` value.
///
/// # Examples
///
/// ```
/// # fn main() -> () { }
/// # use zng_app::{widget::{property, node::UiNode, WIDGET, WidgetUpdateMode}};
/// # use zng_var::IntoVar;
/// # use zng_wgt::node::with_widget_state;
/// # use zng_state_map::{StateId, static_id};
/// #
/// static_id! {
///     pub static ref FOO_ID: StateId<u32>;
/// }
///
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     with_widget_state(child, *FOO_ID, || 0, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &mut impl UiNode) -> u32 {
///     widget.with_context(WidgetUpdateMode::Ignore, || WIDGET.get_state(*FOO_ID)).flatten().unwrap_or_default()
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner() -> u32 {
///     WIDGET.get_state(*FOO_ID).unwrap_or_default()
/// }
/// ```
///
/// [`WIDGET`]: zng_app::widget::WIDGET
pub fn with_widget_state<U, I, T>(child: U, id: impl Into<StateId<T>>, default: I, value: impl IntoVar<T>) -> impl UiNode
where
    U: UiNode,
    I: Fn() -> T + Send + 'static,
    T: StateValue + VarValue,
{
    #[cfg(feature = "dyn_closure")]
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
    #[cfg(feature = "dyn_closure")]
    let default: Box<dyn Fn() -> S + Send> = Box::new(default);
    #[cfg(feature = "dyn_closure")]
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

/// Create a node that controls interaction for all widgets inside `node`.
///
/// When the `interactive` var is `false` all descendant widgets are [`BLOCKED`].
///
/// Unlike the [`interactive`] property this does not apply to the contextual widget, only `child` and descendants.
///
/// The node works for either if the `child` is a widget or if it only contains widgets, the performance
/// is slightly better if the `child` is a widget.
///
/// [`interactive`]: fn@crate::interactive
/// [`BLOCKED`]: Interactivity::BLOCKED
pub fn interactive_node(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    let interactive = interactive.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&interactive);
        }
        UiNodeOp::Info { info } => {
            if interactive.get() {
                child.info(info);
            } else if let Some(id) = child.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                // child is a widget.
                info.push_interactivity_filter(move |args| {
                    if args.info.id() == id {
                        Interactivity::BLOCKED
                    } else {
                        Interactivity::ENABLED
                    }
                });
                child.info(info);
            } else {
                let block_range = info.with_children_range(|info| child.info(info));
                if !block_range.is_empty() {
                    // has child widgets.

                    let id = WIDGET.id();
                    info.push_interactivity_filter(move |args| {
                        if let Some(parent) = args.info.parent() {
                            if parent.id() == id {
                                // check child range
                                for (i, item) in parent.children().enumerate() {
                                    if item == args.info {
                                        return if !block_range.contains(&i) {
                                            Interactivity::ENABLED
                                        } else {
                                            Interactivity::BLOCKED
                                        };
                                    } else if i >= block_range.end {
                                        break;
                                    }
                                }
                            }
                        }
                        Interactivity::ENABLED
                    });
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the index of the widget in the parent panel.
///
/// See [`with_index_len_node`] for more details.
pub fn with_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            // parent PanelList requests updates for this widget every time there is an update.
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(mut c) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let p = c.position(|w| w.id() == id);
                    update(p);
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the reverse index of the widget in the parent panel.
///
/// See [`with_index_len_node`] for more details.
pub fn with_rev_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(c) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let p = c.rev().position(|w| w.id() == id);
                    update(p);
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the index of the widget in the parent panel and the number of children.
///  
/// Panels must use [`PanelList::track_info_range`] to collect the `panel_list_id`, then implement getter properties
/// using the methods in this module. See the `stack!` getter properties for examples.
///
/// [`PanelList::track_info_range`]: zng_app::widget::node::PanelList::track_info_range
pub fn with_index_len_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<(usize, usize)>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(mut iter) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let mut p = 0;
                    let mut count = 0;
                    for c in &mut iter {
                        if c.id() == id {
                            p = count;
                            count += 1 + iter.count();
                            break;
                        } else {
                            count += 1;
                        }
                    }
                    update(Some((p, count)));
                }
            }
        }
        _ => {}
    })
}

/// Node that presents `data` using `wgt_fn`.
///
/// The node's child is always the result of `wgt_fn` called for the `data` value, it is reinited every time
/// either variable changes.
///
/// See also [`presenter_opt`] for a presenter that is nil with the data is `None`.
pub fn presenter<D: VarValue>(data: impl IntoVar<D>, wgt_fn: impl IntoVar<WidgetFn<D>>) -> impl UiNode {
    let data = data.into_var();
    let wgt_fn = wgt_fn.into_var();

    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&wgt_fn);
            *c.child() = wgt_fn.get()(data.get());
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || wgt_fn.is_new() {
                c.child().deinit();
                *c.child() = wgt_fn.get()(data.get());
                c.child().init();
                c.delegated();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

/// Node that presents `data` using `wgt_fn` if data is available, otherwise presents nil.
///
/// This behaves like [`presenter`], but `wgt_fn` is not called if `data` is `None`.
pub fn presenter_opt<D: VarValue>(data: impl IntoVar<Option<D>>, wgt_fn: impl IntoVar<WidgetFn<D>>) -> impl UiNode {
    let data = data.into_var();
    let wgt_fn = wgt_fn.into_var();

    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&wgt_fn);
            if let Some(data) = data.get() {
                *c.child() = wgt_fn.get()(data);
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || wgt_fn.is_new() {
                if let Some(data) = data.get() {
                    c.child().deinit();
                    *c.child() = wgt_fn.get()(data);
                    c.child().init();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                } else if c.child().actual_type_id() != TypeId::of::<NilUiNode>() {
                    c.child().deinit();
                    *c.child() = NilUiNode.boxed();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                }
            }
        }
        _ => {}
    })
}

/// Node that presents `list` using `item_fn` for each new list item.
///
/// The node's children is the list mapped to node items, it is kept in sync, any list update is propagated to the node list.
pub fn list_presenter<D: VarValue>(list: impl IntoVar<ObservableVec<D>>, item_fn: impl IntoVar<WidgetFn<D>>) -> impl UiNodeList {
    ListPresenter {
        list: list.into_var(),
        item_fn: item_fn.into_var(),
        view: vec![],
        _e: std::marker::PhantomData,
    }
}

struct ListPresenter<D: VarValue, L: Var<ObservableVec<D>>, E: Var<WidgetFn<D>>> {
    list: L,
    item_fn: E,
    view: Vec<BoxedUiNode>,
    _e: std::marker::PhantomData<D>,
}

impl<D, L, E> UiNodeList for ListPresenter<D, L, E>
where
    D: VarValue,
    L: Var<ObservableVec<D>>,
    E: Var<WidgetFn<D>>,
{
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.view.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.view.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.view.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.view.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.view.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.view.drain_into(vec);
        tracing::warn!("drained `list_presenter`, now out of sync with data");
    }

    fn init_all(&mut self) {
        debug_assert!(self.view.is_empty());
        self.view.clear();

        WIDGET.sub_var(&self.list).sub_var(&self.item_fn);

        let e_fn = self.item_fn.get();
        self.list.with(|l| {
            for el in l.iter() {
                let child = e_fn(el.clone());
                self.view.push(child);
            }
        });

        self.view.init_all();
    }

    fn deinit_all(&mut self) {
        self.view.deinit_all();
        self.view.clear();
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut need_reset = self.item_fn.is_new();

        let is_new = self
            .list
            .with_new(|l| {
                need_reset |= l.changes().is_empty() || l.changes() == [VecChange::Clear];

                if need_reset {
                    return;
                }

                // update before new items to avoid update before init.
                self.view.update_all(updates, observer);

                let e_fn = self.item_fn.get();

                for change in l.changes() {
                    match change {
                        VecChange::Insert { index, count } => {
                            for i in *index..(*index + count) {
                                let mut el = e_fn(l[i].clone());
                                el.init();
                                self.view.insert(i, el);
                                observer.inserted(i);
                            }
                        }
                        VecChange::Remove { index, count } => {
                            let mut count = *count;
                            let index = *index;
                            while count > 0 {
                                count -= 1;

                                let mut el = self.view.remove(index);
                                el.deinit();
                                observer.removed(index);
                            }
                        }
                        VecChange::Move { from_index, to_index } => {
                            let el = self.view.remove(*from_index);
                            self.view.insert(*to_index, el);
                            observer.moved(*from_index, *to_index);
                        }
                        VecChange::Clear => unreachable!(),
                    }
                }
            })
            .is_some();

        if !need_reset && !is_new && self.list.with(|l| l.len() != self.view.len()) {
            need_reset = true;
        }

        if need_reset {
            self.view.deinit_all();
            self.view.clear();

            let e_fn = self.item_fn.get();
            self.list.with(|l| {
                for el in l.iter() {
                    let child = e_fn(el.clone());
                    self.view.push(child);
                }
            });

            self.view.init_all();
        } else if !is_new {
            self.view.update_all(updates, observer);
        }
    }
}

use crate::WidgetFn;
#[doc(inline)]
pub use crate::command_property;
#[doc(inline)]
pub use crate::event_property;
