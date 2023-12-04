//! Helper nodes.
//!
//! This module defines some foundational nodes that can be used for declaring properties and widgets.

use std::sync::Arc;

use zero_ui_app::{
    event::{Command, CommandArgs, Event, EventArgs},
    handler::WidgetHandler,
    widget::{instance::*, WIDGET},
};
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
    ) => { $crate::nodes::paste! {
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
        #[$crate::nodes::zero_ui_app::widget::property(EVENT, default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $event>](
            child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
            handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$Args>,
        ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
            $crate::__event_property!(with($crate::nodes::on_event(child, $EVENT, $filter, handler), false, $($with)?))
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
        #[$crate::nodes::zero_ui_app::widget::property(EVENT, default( $crate::nodes::zero_ui_app::handler::hn!(|_|{}) ))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::nodes::zero_ui_app::widget::instance::UiNode,
            handler: impl $crate::nodes::zero_ui_app::handler::WidgetHandler<$Args>,
        ) -> impl $crate::nodes::zero_ui_app::widget::instance::UiNode {
            $crate::__event_property!(with($crate::nodes::on_pre_event(child, $EVENT, $filter, handler), true, $($with)?))
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

#[doc(inline)]
pub use crate::command_property;
#[doc(inline)]
pub use crate::event_property;
