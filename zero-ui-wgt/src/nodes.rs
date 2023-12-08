//! Helper nodes.
//!
//! This module defines some foundational nodes that can be used for declaring properties and widgets.

use std::{any::Any, sync::Arc};

use zero_ui_app::{
    event::{Command, CommandArgs, Event, EventArgs},
    handler::WidgetHandler,
    render::{FrameBuilder, FrameValueKey},
    widget::{
        border::{BORDER, BORDER_ALIGN_VAR, BORDER_OVER_VAR},
        info::Interactivity,
        instance::*,
        VarLayout, WidgetUpdateMode, WIDGET,
    },
};
use zero_ui_app_context::{ContextLocal, LocalContext};
use zero_ui_layout::{
    context::LAYOUT,
    units::{PxConstraints2d, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize, PxVector, SideOffsets},
};
use zero_ui_state_map::{StateId, StateMapRef, StateValue};
use zero_ui_var::*;

#[doc(hidden)]
pub use paste::paste;

#[doc(hidden)]
pub use zero_ui_app;

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
/// # use zero_ui_app::{*, widget::{instance::*, *}};
/// # use zero_ui_var::*;
/// # use zero_ui_wgt::nodes::*;
/// #
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
/// # use zero_ui_app::{*, widget::{instance::*, *}};
/// # use zero_ui_var::*;
/// # use zero_ui_wgt::nodes::*;
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
/// [`is_new`]: zero_ui_var::AnyVar::is_new
/// [`read_only`]: zero_ui_var::Var::read_only
/// [`actual_var`]: zero_ui_var::Var::actual_var
/// [`default`]: zero_ui_app::widget::property#default
/// [`merge_var!`]: zero_ui_var::merge_var
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

///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event`, `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::event::*;
/// # use zero_ui_wgt::nodes::*;
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
/// # Implement For
///
/// You can implement the new properties for a widget or mix-in using `widget_impl`:
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{event::*, widget::{instance::UiNode, widget_mixin}};
/// # use zero_ui_wgt::nodes::*;
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
/// You can wrap the event handler node with extra nodes by setting the optional `with` closure:
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{event::*, widget::instance::UiNode};
/// # use zero_ui_wgt::nodes::*;
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
/// The closure receives two arguments, the handler `UiNode` and a `bool` that is `true` if the closure is called in in the *on_pre_*
/// property or `false` when called in the *on_* property.
///
/// [`on_pre_event`]: crate::nodes::on_pre_event
/// [`on_event`]: crate::nodes::on_event
/// [`propagation`]: zero_ui_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zero_ui_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zero_ui_app::widget::info::Interactivity::DISABLED
/// [`UpdateDeliveryList`]: zero_ui_app::update::UpdateDeliveryList
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            $(filter: $filter:expr,)?
            $(widget_impl: $Wgt:ty,)?
            $(with: $with:expr,)?
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
        $crate::nodes::paste! {
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
            #[$crate::nodes::zero_ui_app::widget::property(
                EVENT,
                default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) )
                $($widget_impl)*
            )]
            $vis fn [<on_ $event>](
                child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
                handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$Args>,
            ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
                $crate::__event_property!(=> with($crate::nodes::on_event(child, $EVENT, $filter, handler), false, $($with)?))
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
            #[$crate::nodes::zero_ui_app::widget::property(
                EVENT,
                default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) )
                $($widget_impl)*
            )]
            $vis fn [<on_pre_ $event>](
                child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
                handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$Args>,
            ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
                $crate::__event_property!(=> with($crate::nodes::on_pre_event(child, $EVENT, $filter, handler), true, $($with)?))
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
/// [`propagation`]: zero_ui_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zero_ui_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zero_ui_app::widget::info::Interactivity::DISABLED
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
/// [`propagation`]: zero_ui_app::event::AnyEventArgs::propagation
/// [`ENABLED`]: zero_ui_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zero_ui_app::widget::info::Interactivity::DISABLED
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

#[doc(hidden)]
#[macro_export]
macro_rules! __command_property {
    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr,
            enabled: $enabled_var:expr,
        }
    ) => { $crate::nodes::paste! {
        $(#[$on_cmd_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this command event using [`on_pre_"$command "`](fn.on_pre_"$command ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::nodes::zero_ui_app::widget::property(EVENT, default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $command>](
            child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
            handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$crate::nodes::zero_ui_app::event::CommandArgs>,
        ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
            $crate::nodes::on_command(child, || $cmd_init, || $enabled_var, handler)
        }

        #[doc = "Preview [`on_"$command "`](fn.on_"$command ".html) event."]
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
        #[$crate::nodes::zero_ui_app::widget::property(EVENT, default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) ))]
        $vis fn [<on_pre_ $command>](
            child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
            handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$crate::nodes::zero_ui_app::event::CommandArgs>,
        ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
            $crate::nodes::on_pre_command(child, || $cmd_init, || $enabled_var, handler)
        }
    } };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr,
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd: $cmd_init,
                enabled: $crate::nodes::zero_ui_app::var::LocalVar(true),
            }
        }
    };
}

///<span data-del-macro-root></span> Declare one or more command event properties.
///
/// Each declaration expands to two properties `on_$command`, `on_pre_$command`.
/// The preview properties call [`on_pre_command`], the main event properties call [`on_command`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{event::*, widget::*};
/// # use zero_ui_app::var::*;
/// # use zero_ui_wgt::nodes::*;
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
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// If not provided the command is always enabled.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr$(,
            enabled: $enabled_var:expr)? $(,)?
        }
    )+) => {$(
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd: $cmd_init,
                $(enabled: $enabled_var,)?
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
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
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
/// [`propagation`]: zero_ui_app::event::AnyEventArgs::propagation
pub fn on_command<U, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    #[cfg(dyn_closure)]
    let command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(command_builder);
    #[cfg(dyn_closure)]
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
    let mut handle = None;
    let mut command = None;

    let mut handler = handler.cfg_boxed();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            let c = command_builder();
            handle = Some(c.subscribe_wgt(is_enabled, WIDGET.id()));
            command = Some(c);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = None;
            command = None;
        }

        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = command.expect("node not inited").on_unhandled(update) {
                handler.event(args);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("node not inited").get_new() {
                handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        _ => {}
    })
}

/// Helper for declaring command preview handlers.
///
/// # Command
///
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// Note that the command handler can be enabled even when the widget is disabled, the widget
/// will receive the event while disabled in this case, you can use this to show feedback explaining
/// why the command cannot run.
///
/// # Route
///
/// The event `handler` is called before the [`on_command`] equivalent at the same context level. If the command event
/// targets more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///  
/// [`propagation`]: zero_ui_app::event::AnyEventArgs::propagation
pub fn on_pre_command<U, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    #[cfg(dyn_closure)]
    let command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(command_builder);
    #[cfg(dyn_closure)]
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
    let mut handle = None;
    let mut command = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            let c = command_builder();
            handle = Some(c.subscribe_wgt(is_enabled, WIDGET.id()));
            command = Some(c);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = None;
            command = None;
        }

        UiNodeOp::Event { update } => {
            if let Some(args) = command.expect("on_pre_command not initialized").on_unhandled(update) {
                handler.event(args);
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("on_pre_command not initialized").get_new() {
                handle.as_ref().unwrap().set_enabled(enabled);
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

/// Transforms and clips the `content` node according with the default widget border behavior.
///
/// Properties that *fill* the widget can wrap their fill content in this node to automatically implement
/// the expected behavior of interaction with the widget borders, the content will positioned, sized and clipped according to the
/// widget borders, corner radius and border align.
///
/// Note that this node should **not** be used for the property child node (first argument), only other
/// content that fills the widget, for examples, a *background* property would wrap its background node with this
/// but just pass thought layout and render for its child node.
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

/// Creates a border node that delegates rendering to a `border_visual`, but manages the `border_offsets` coordinating
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

/// Helper for declaring nodes that sets a context local.
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

/// Helper for declaring nodes that sets a context local with a value generated on init.
///
/// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextLocal<T>`]
/// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
///
/// Apart from the value initialization this behaves just like [`with_context_local`].
pub fn with_context_local_init<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    init_value: impl FnMut() -> T + Send + 'static,
) -> impl UiNode {
    #[cfg(dyn_closure)]
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
/// [`NestGroup::CHILD`]: zero_ui_app::widget::builder::NestGroup::CHILD
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
/// use zero_ui_app::{widget::{property, instance::UiNode, WIDGET, WidgetUpdateMode}};
/// use zero_ui_var::IntoVar;
/// use zero_ui_wgt::nodes::with_widget_state;
/// use zero_ui_state_map::{StaticStateId, StateId};
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

/// Create a node that disables interaction for all widget inside `node` using [`BLOCKED`].
///
/// Unlike the `interactive` property this does not apply to the contextual widget, only `child` and descendants.
///
/// The node works for both if the `child` is a widget or if it contains widgets, the performance
/// is slightly better if the `child` is a widget directly.
///
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

/// Helper for a property that gets the *index* of the widget in the parent panel.
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

/// Helper for a property that gets the *index* of the widget in the parent panel.
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

/// Helper for a property that gets the *index* of the widget in the parent panel and the number of children.
///  
/// Panels must use [`PanelList::track_info_range`] to collect the `panel_list_id`, then implement getter properties
/// using the methods in this module. See the [`stack`] getter properties for examples.
///
/// [`stack`]: crate::widgets::layouts::stack
/// [`PanelList::track_info_range`]: crate::core::widget_instance::PanelList::track_info_range
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

#[doc(inline)]
pub use crate::command_property;
#[doc(inline)]
pub use crate::event_property;
