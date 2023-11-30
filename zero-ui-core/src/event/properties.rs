use super::*;
use crate::{
    handler::WidgetHandler,
    widget_instance::{match_node, UiNode, UiNodeOp},
};

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            filter: $filter:expr,
            with: $($with:expr)? $(,)?
        }
    ) => { $crate::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this event using [`on_pre_"$event "`](fn.on_pre_"$event ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $event>](
            child: impl $crate::widget_instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::widget_instance::UiNode {
            $crate::__event_property!(with($crate::event::on_event(child, $EVENT, $filter, handler), false, $($with)?))
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
        #[$crate::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::widget_instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::widget_instance::UiNode {
            $crate::__event_property!(with($crate::event::on_pre_event(child, $EVENT, $filter, handler), true, $($with)?))
        }
    } };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: |_args| true,
                with:
            }
        }
    };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: $filter,
                with:
            }
        }
    };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            with: $with:expr,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: |_args| true,
                with: $with,
            }
        }
    };

    (with($child:expr, $preview:expr,)) => { $child };
    (with($child:expr, $preview:expr, $with:expr)) => { ($with)($child, $preview) };
}
///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event`, `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
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
/// App events are delivered to all `UiNode` inside all widgets in the [`UpdateDeliveryList`] and event subscribers list,
/// event properties can specialize further by defining a filter predicate.
///
/// The `filter` predicate is called if [`propagation`] is not stopped. It must return `true` if the event arguments
/// are relevant in the context of the widget and event property. If it returns `true` the `handler` closure is called.
/// See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default always allows, so all app events targeting the widget and not already handled
/// are allowed by default.  Note that events that represent an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`]
/// targets, event properties should probably distinguish if they fire on normal interactions vs on *disabled* interactions.
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
/// # With Extra Nodes
///
/// You can wrap the event handler node with extra nodes by setting the optional `with` closure:
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
/// # fn some_node(child: impl zero_ui_core::widget_instance::UiNode) -> impl zero_ui_core::widget_instance::UiNode { child }
/// event_property! {
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         with: |child, _preview| some_node(child),
///     }
/// }
/// ```
///
/// The closure receives two arguments, the handler `UiNode` and a `bool` that is `true` if the closure is called in in the *on_pre_*
/// property or `false` when called in the *on_* property.
///
/// [`on_pre_event`]: crate::event::on_pre_event
/// [`on_event`]: crate::event::on_event
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,
            with: $with:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                $(filter: $filter,)?
                $(with: $with,)?
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event_property;

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
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
pub fn on_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(dyn_closure)]
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
                    handler.event(args);
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
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
pub fn on_pre_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(dyn_closure)]
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
                    handler.event(args);
                }
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();
        }
        _ => {}
    })
}
