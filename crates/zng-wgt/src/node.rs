//! Helper nodes.
//!
//! This module defines some foundational nodes that can be used for declaring properties and widgets.

use std::{any::Any, sync::Arc};

use crate::WidgetFn;
use zng_app::{
    event::{Command, CommandHandle, Event, EventArgs},
    handler::{Handler, HandlerExt as _},
    render::{FrameBuilder, FrameValueKey},
    update::WidgetUpdates,
    widget::{
        VarLayout, WIDGET,
        border::{BORDER, BORDER_ALIGN_VAR, BORDER_OVER_VAR},
        info::Interactivity,
        node::*,
    },
};
use zng_app_context::{ContextLocal, LocalContext};
use zng_layout::{
    context::LAYOUT,
    unit::{PxConstraints2d, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize, PxVector, SideOffsets},
};
use zng_state_map::{StateId, StateMapRef, StateValue};
use zng_var::*;

#[doc(hidden)]
pub use pastey::paste;

#[doc(hidden)]
pub mod __macro_util {
    pub use zng_app::{
        event::CommandArgs,
        handler::{Handler, hn},
        widget::{
            node::{IntoUiNode, UiNode},
            property,
        },
    };
    pub use zng_var::{IntoVar, context_var};
}

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
/// pub fn foo(child: impl IntoUiNode, value: impl IntoVar<u32>) -> UiNode {
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
/// pub fn foo(child: impl IntoUiNode, value: impl IntoVar<bool>) -> UiNode {
///     with_context_var(
///         child,
///         CONFIG_VAR,
///         merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
///             let mut c = c.clone();
///             c.foo = v;
///             c
///         }),
///     )
/// }
///
/// /// Sets the *bar* config.
/// #[property(CONTEXT, default(false))]
/// pub fn bar(child: impl IntoUiNode, value: impl IntoVar<bool>) -> UiNode {
///     with_context_var(
///         child,
///         CONFIG_VAR,
///         merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
///             let mut c = c.clone();
///             c.bar = v;
///             c
///         }),
///     )
/// }
/// ```
///
/// When set in a widget, the [`merge_var!`] will read the context value of the parent properties, modify a clone of the value and
/// the result will be accessible to the inner properties, the widget user can then set with the composed value in steps and
/// the final consumer of the composed value only need to monitor to a single context variable.
///
/// [`is_new`]: zng_var::AnyVar::is_new
/// [`read_only`]: zng_var::Var::read_only
/// [`default`]: zng_app::widget::property#default
/// [`merge_var!`]: zng_var::merge_var
/// [`UiNode`]: zng_app::widget::node::UiNode
/// [`ContextVar::with_context`]: zng_var::ContextVar::with_context
pub fn with_context_var<T: VarValue>(child: impl IntoUiNode, context_var: ContextVar<T>, value: impl IntoVar<T>) -> UiNode {
    let value = value.into_var();
    let mut actual_value = None;
    let mut id = None;

    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                id = Some(ContextInitHandle::new());
                actual_value = Some(Arc::new(value.current_context().into()));
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
    child: impl IntoUiNode,
    var: ContextVar<T>,
    mut init_value: impl FnMut() -> Var<T> + Send + 'static,
) -> UiNode {
    let mut id = None;
    let mut value = None;
    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                id = Some(ContextInitHandle::new());
                value = Some(Arc::new(init_value().current_context().into()));
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

/// Helper for declaring event properties.
pub struct EventNodeBuilder<A: EventArgs, F, M> {
    event: Event<A>,
    filter_builder: F,
    map_args: M,
}
/// Helper for declaring event properties from variables.
pub struct VarEventNodeBuilder<I, F, M> {
    init_var: I,
    filter_builder: F,
    map_args: M,
}

impl<A: EventArgs> EventNodeBuilder<A, (), ()> {
    /// Node that calls the handler for all args that target the widget and has not stopped propagation.
    pub fn new(event: Event<A>) -> EventNodeBuilder<A, (), ()> {
        EventNodeBuilder {
            event,
            filter_builder: (),
            map_args: (),
        }
    }
}
impl<I, T> VarEventNodeBuilder<I, (), ()>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
{
    /// Node that calls the handler for var updates.
    ///
    /// The `init_var` is called on init to
    pub fn new(init_var: I) -> VarEventNodeBuilder<I, (), ()> {
        VarEventNodeBuilder {
            init_var,
            filter_builder: (),
            map_args: (),
        }
    }
}

impl<A: EventArgs, M> EventNodeBuilder<A, (), M> {
    /// Filter event.
    ///
    /// The `filter_builder` is called on init and on event, it must produce another closure, the filter predicate. The `filter_builder`
    /// runs in the widget context, the filter predicate does not always.
    ///
    /// In the event hook the filter predicate runs in the app context, it is called if the args target the widget, the predicate must
    /// use any captured contextual info to filter the args, this is an optimization, it can save a visit to the widget node.
    ///
    /// If the event is received the second filter predicate is called again to confirm the event.
    /// The second instance is called if [`propagation`] was not stopped, if it returns `true` the `handler` closure is called.
    ///
    /// Note that events that represent an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets,
    /// event properties should probably distinguish if they fire on normal interactions vs on *disabled* interactions.
    pub fn filter<FB, F>(self, filter_builder: FB) -> EventNodeBuilder<A, FB, M>
    where
        FB: FnMut() -> F + Send + 'static,
        F: Fn(&A) -> bool + Send + Sync + 'static,
    {
        EventNodeBuilder {
            event: self.event,
            filter_builder,
            map_args: self.map_args,
        }
    }
}
impl<T, I, M> VarEventNodeBuilder<I, (), M>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
{
    /// Filter event.
    ///
    /// The `filter_builder` is called on init and on new, it must produce another closure, the filter predicate. The `filter_builder`
    /// runs in the widget context, the filter predicate does not always.
    ///
    /// In the variable hook the filter predicate runs in the app context, it is called if the args target the widget, the predicate must
    /// use any captured contextual info to filter the args, this is an optimization, it can save a visit to the widget node.
    ///
    /// If the update is received the second filter predicate is called again to confirm the update.
    /// If it returns `true` the `handler` closure is called.
    pub fn filter<FB, F>(self, filter_builder: FB) -> VarEventNodeBuilder<I, FB, M>
    where
        FB: FnMut() -> F + Send + 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        VarEventNodeBuilder {
            init_var: self.init_var,
            filter_builder,
            map_args: self.map_args,
        }
    }
}

impl<A: EventArgs, F> EventNodeBuilder<A, F, ()> {
    /// Convert args.
    ///
    /// The `map_args` closure is called in context, just before the handler is called.
    pub fn map_args<M, MA>(self, map_args: M) -> EventNodeBuilder<A, F, M>
    where
        M: FnMut(&A) -> MA + Send + 'static,
        MA: Clone + 'static,
    {
        EventNodeBuilder {
            event: self.event,
            filter_builder: self.filter_builder,
            map_args,
        }
    }
}
impl<T, I, F> VarEventNodeBuilder<I, F, ()>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
{
    /// Convert args.
    ///
    /// The `map_args` closure is called in context, just before the handler is called.
    ///
    /// Note that if the args is a full [`EventArgs`] type it must share the same propagation handle in the preview and normal route
    /// properties, if the source type is also a full args just clone [`EventArgs::propagation`], otherwise you must use [`WIDGET::set_state`]
    /// to communicate between the properties.
    pub fn map_args<M, MA>(self, map_args: M) -> VarEventNodeBuilder<I, F, M>
    where
        M: FnMut(&T) -> MA + Send + 'static,
        MA: Clone + 'static,
    {
        VarEventNodeBuilder {
            init_var: self.init_var,
            filter_builder: self.filter_builder,
            map_args,
        }
    }
}

/// Build with filter and args mapping.
impl<A, F, FB, MA, M> EventNodeBuilder<A, FB, M>
where
    A: EventArgs,
    F: Fn(&A) -> bool + Send + Sync + 'static,
    FB: FnMut() -> F + Send + 'static,
    MA: Clone + 'static,
    M: FnMut(&A) -> MA + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<MA>) -> UiNode {
        let Self {
            event,
            mut filter_builder,
            mut map_args,
        } = self;
        let mut handler = handler.into_wgt_runner();
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_event_when(&event, filter_builder());
            }
            UiNodeOp::Deinit => {
                handler.deinit();
            }
            UiNodeOp::Update { updates } => {
                if !PRE {
                    child.update(updates);
                }

                handler.update();

                let mut f = None;
                event.each_update(false, |args| {
                    if f.get_or_insert_with(|| filter_builder())(args) {
                        handler.event(&map_args(args));
                    }
                });
            }
            _ => {}
        })
    }
}

/// Build with filter and args mapping.
impl<T, I, F, FB, MA, M> VarEventNodeBuilder<I, FB, M>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
    FB: FnMut() -> F + Send + 'static,
    MA: Clone + 'static,
    M: FnMut(&T) -> MA + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<MA>) -> UiNode {
        let Self {
            mut init_var,
            mut filter_builder,
            mut map_args,
        } = self;
        let mut handler = handler.into_wgt_runner();
        let mut var = None;
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                let v = init_var();
                let f = filter_builder();
                WIDGET.sub_var_when(&v, move |a| f(a.value()));
                var = Some(v);
            }
            UiNodeOp::Deinit => {
                handler.deinit();
                var = None;
            }
            UiNodeOp::Update { updates } => {
                if PRE {
                    child.update(updates);
                }

                handler.update();

                var.as_ref().unwrap().with_new(|t| {
                    if filter_builder()(t) {
                        handler.event(&map_args(t));
                    }
                });
            }
            _ => {}
        })
    }
}

/// Build with filter and without args mapping.
impl<A, F, FB> EventNodeBuilder<A, FB, ()>
where
    A: EventArgs,
    F: Fn(&A) -> bool + Send + Sync + 'static,
    FB: FnMut() -> F + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<A>) -> UiNode {
        let Self {
            event, mut filter_builder, ..
        } = self;
        let mut handler = handler.into_wgt_runner();
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_event_when(&event, filter_builder());
            }
            UiNodeOp::Deinit => {
                handler.deinit();
            }
            UiNodeOp::Update { updates } => {
                if !PRE {
                    child.update(updates);
                }

                handler.update();

                let mut f = None;
                event.each_update(false, |args| {
                    if f.get_or_insert_with(|| filter_builder())(args) {
                        handler.event(args);
                    }
                });
            }
            _ => {}
        })
    }
}
/// Build with filter and without args mapping.
impl<T, I, F, FB> VarEventNodeBuilder<I, FB, ()>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
    FB: FnMut() -> F + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<T>) -> UiNode {
        let Self {
            mut init_var,
            mut filter_builder,
            ..
        } = self;
        let mut handler = handler.into_wgt_runner();
        let mut var = None;
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                let v = init_var();
                let f = filter_builder();
                WIDGET.sub_var_when(&v, move |a| f(a.value()));
                var = Some(v);
            }
            UiNodeOp::Deinit => {
                handler.deinit();
                var = None;
            }
            UiNodeOp::Update { updates } => {
                if !PRE {
                    child.update(updates);
                }

                handler.update();

                var.as_ref().unwrap().with_new(|t| {
                    if filter_builder()(t) {
                        handler.event(t);
                    }
                });
            }
            _ => {}
        })
    }
}

/// Build without filter and without args mapping.
impl<A> EventNodeBuilder<A, (), ()>
where
    A: EventArgs,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<A>) -> UiNode {
        let Self { event, .. } = self;
        let mut handler = handler.into_wgt_runner();
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_event(&event);
            }
            UiNodeOp::Deinit => {
                handler.deinit();
            }
            UiNodeOp::Update { updates } => {
                if !PRE {
                    child.update(updates);
                }

                handler.update();

                event.each_update(false, |args| {
                    handler.event(args);
                });
            }
            _ => {}
        })
    }
}
/// Build without filter and without args mapping.
impl<T, I> VarEventNodeBuilder<I, (), ()>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<T>) -> UiNode {
        let Self { mut init_var, .. } = self;
        let mut handler = handler.into_wgt_runner();
        let mut var = None;
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                let v = init_var();
                WIDGET.sub_var(&v);
                var = Some(v);
            }
            UiNodeOp::Deinit => {
                handler.deinit();
                var = None;
            }
            UiNodeOp::Update { updates } => {
                if !PRE {
                    child.update(updates);
                }

                handler.update();

                var.as_ref().unwrap().with_new(|t| {
                    handler.event(t);
                });
            }
            _ => {}
        })
    }
}

/// Build with no filter and args mapping.
impl<A, MA, M> EventNodeBuilder<A, (), M>
where
    A: EventArgs,
    MA: Clone + 'static,
    M: FnMut(&A) -> MA + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<MA>) -> UiNode {
        self.filter(|| |_| true).build::<PRE>(child, handler)
    }
}
/// Build with no filter and args mapping.
impl<T, I, MA, M> VarEventNodeBuilder<I, (), M>
where
    T: VarValue,
    I: FnMut() -> Var<T> + Send + 'static,
    MA: Clone + 'static,
    M: FnMut(&T) -> MA + Send + 'static,
{
    /// Build node.
    ///
    /// If `PRE` is `true` the handler is called before the children, *preview* route.
    pub fn build<const PRE: bool>(self, child: impl IntoUiNode, handler: Handler<MA>) -> UiNode {
        self.filter(|| |_| true).build::<PRE>(child, handler)
    }
}

///<span data-del-macro-root></span> Declare event properties.
///
/// Each declaration can expand to an `on_event` and optionally an `on_pre_event`. The body can be declared using [`EventNodeBuilder`] or
/// [`VarEventNodeBuilder`].
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
/// # struct CONTEXT;
/// # impl CONTEXT { pub fn state(&self) -> zng_var::Var<bool> { zng_var::var(true) } }
/// event_property! {
///     /// Docs copied for `on_key_input` and `on_pre_key_input`.
///     ///
///     /// The macro also generates docs linking between the two properties.
///     #[property(EVENT)]
///     pub fn on_key_input<on_pre_key_input>(child: impl IntoUiNode, handler: Handler<KeyInputArgs>) -> UiNode {
///         // Preview flag, only if the signature contains the `<on_pre...>` part,
///         // the macro matches `const $IDENT: bool;` and expands to `const IDENT: bool = true/false;`.
///         const PRE: bool;
///
///         // rest of the body can be anything the builds a node.
///         EventNodeBuilder::new(KEY_INPUT_EVENT).build::<PRE>(child, handler)
///     }
///
///     /// Another property.
///     #[property(EVENT)]
///     pub fn on_key_down<on_key_down>(child: impl IntoUiNode, handler: Handler<KeyInputArgs>) -> UiNode {
///         const PRE: bool;
///         EventNodeBuilder::new(KEY_INPUT_EVENT)
///             .filter(|a| a.state == KeyState::Pressed)
///             .build::<PRE>(child, handler)
///     }
///
///     /// Another, this time derived from a var source, and without the optional preview property.
///     #[property(EVENT)]
///     pub fn on_state(child: impl IntoUiNode, handler: Handler<bool>) -> UiNode {
///         VarEventNodeBuilder::new(|| CONTEXT.state())
///             .map_args(|b| !*b)
///             .build::<false>(child, handler)
///     }
/// }
/// ```
///
/// The example above generates five event properties.
///
/// # Route
///
/// Note that is an event property has an `on_pre_*` pair it is expected to be representing a fully routing event, with args that
/// implement [`EventArgs`]. If the property does not have a preview pair it is expected to be a *direct* event. This is the event
/// property pattern and is explained in the generated documentation, don't declare a non-standard pair using this macro.
///
/// # Commands
///
/// You can use [`command_property`] to declare command event properties, it also generates enabled control properties.
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident $(< $on_pre_ident:ident >)? (
            $child:ident: impl $IntoUiNode:path,
            $handler:ident: $Handler:ty
        ) -> $UiNode:path {
            $($body:tt)+
        }
    )+) => {$(
       $crate::event_property_impl! {
            $(#[$meta])+
            $vis fn $on_ident $(< $on_pre_ident >)? ($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
                $($body)+
            }
       }
    )+};
}
#[doc(inline)]
pub use event_property;

#[doc(hidden)]
#[macro_export]
macro_rules! event_property_impl {
    (
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident < $on_pre_ident:ident > ($child:ident : impl $IntoUiNode:path, $handler:ident : $Handler:ty) -> $UiNode:path {
            const $PRE:ident : bool;
            $($body:tt)+
        }
    ) => {
        $(#[$meta])+
        ///
        /// # Route
        ///
        /// This event property uses the normal route, that is, the `handler` is called after the children widget handlers and after the
        #[doc = concat!("[`", stringify!($pn_pre_ident), "`](fn@", stringify!($pn_pre_ident), ")")]
        /// handlers.
        $vis fn $on_ident($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
            const $PRE: bool = false;
            $($body)+
        }

        $(#[$meta])+
        ///
        /// # Route
        ///
        /// This event property uses the preview route, that is, the `handler` is called before the children widget handlers and before the
        #[doc = concat!("[`", stringify!($pn_ident), "`](fn@", stringify!($pn_ident), ")")]
        /// handlers.
        $vis fn $on_pre_ident($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
            const $PRE: bool = true;
            $($body)+
        }
    };

    (
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident ($child:ident : impl $IntoUiNode:path, $handler:ident : $Handler:path) -> $UiNode:path {
            $($body:tt)+
        }
    ) => {
        $(#[$meta])+
        ///
        /// # Route
        ///
        /// This event property uses a *direct* route, that is, it cannot be intercepted in parent widgets.
        $vis fn $on_ident($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
            $($body)+
        }
    };
}

#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident < $on_pre_ident:ident $($can_ident:ident)?> (
            $child:ident: impl $IntoUiNode:path,
            $handler:ident: $Handler:ty
        ) -> $UiNode:path {
            $COMMAND:path
        }
    )+) => {$(
       $crate::command_property_impl! {
            $(#[$meta])+
            $vis fn $on_ident<$on_pre_ident $(, $can_ident)?>($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
                $COMMAND
            }
       }
    )+};
}
#[doc(hidden)]
#[macro_export]
macro_rules! command_property_impl {
    (
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident < $on_pre_ident:ident, $can_ident:ident> (
            $child:ident: impl $IntoUiNode:path,
            $handler:ident: $Handler:ty
        ) -> $UiNode:path {
            $COMMAND:path
        }
    ) => {
        $crate::paste! {
            $crate::node::__macro_util::context_var! {
                /// Defines if
                #[doc = concat!("[`", stringify!($on_ident), "`](fn@", stringify!($on_ident), ")")]
                /// and
                #[doc = concat!("[`", stringify!($on_pre_ident), "`](fn@", stringify!($on_pre_ident), ")")]
                /// command handlers are enabled in a widget and descendants.
                ///
                /// Use
                #[doc = concat!("[`", stringify!($can_ident), "`](fn@", stringify!($can_ident), ")")]
                /// to set. Is enabled by default.
                $vis [<$can_ident:upper _VAR>]: bool = true;
            }

            /// Defines if
            #[doc = concat!("[`", stringify!($on_ident), "`](fn@", stringify!($on_ident), ")")]
            /// and
            #[doc = concat!("[`", stringify!($on_pre_ident), "`](fn@", stringify!($on_pre_ident), ")")]
            /// command handlers are enabled in the widget and descendants.
            ///
            #[doc = "Sets the [`"$can_ident:upper "_VAR`]."]
            $vis fn $can_ident(
                child: impl $crate::node::__macro_util::IntoUiNode,
                enabled: impl $crate::node::__macro_util::IntoVar<bool>,
            ) -> $crate::node::__macro_util::UiNode {
                $crate::node::with_context_var(child, self::[<$can_ident:upper _VAR>], enabled)
            }

            $crate::event_property! {
                $(#[$meta])+
                ///
                /// # Enabled
                ///
                /// The command handle is enabled by default and can be disabled using the contextual property
                #[doc = concat!("[`", stringify!($can_ident), "`](fn@", stringify!($can_ident), ")")]
                /// .
                ///
                $vis fn $on_ident<$on_pre_ident>($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
                    const PRE: bool;
                    $crate::node::EventNodeBuilder::new($COMMAND)
                    .filter(|| {
                        let enabled = self::[<$can_ident:upper _VAR>].current_context();
                        move |_| enabled.get()
                    })
                    .build::<PRE>($child, $handler)
                }
            }
        }
    };
    (
        $(#[$meta:meta])+
        $vis:vis fn $on_ident:ident < $on_pre_ident:ident> (
            $child:ident: impl $IntoUiNode:path,
            $handler:ident: $Handler:ty
        ) -> $UiNode:path {
            $COMMAND:path
        }
    ) => {
        $crate::event_property! {
            $(#[$meta])+
            ///
            /// # Enabled
            ///
            /// The command handle is always enabled.
            ///
            $vis fn $on_ident<$on_pre_ident>($child: impl $IntoUiNode, $handler: $Handler) -> $UiNode {
                const PRE: bool;
                $crate::node::EventNodeBuilder::new($COMMAND).build::<PRE>($child, $handler)
            }
        }
    };
}

#[doc(hidden)]
pub fn command_always_enabled(child: UiNode, cmd: Command) -> UiNode {
    let mut _handle = CommandHandle::dummy();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            _handle = cmd.scoped(WIDGET.id()).subscribe(true);
        }
        UiNodeOp::Deinit => {
            _handle = CommandHandle::dummy();
        }
        _ => {}
    })
}

/// Logs an error if the `_var` is always read-only.
pub fn validate_getter_var<T: VarValue>(_var: &Var<T>) {
    #[cfg(debug_assertions)]
    if _var.capabilities().is_always_read_only() {
        tracing::error!(
            "`is_`, `has_` or `get_` property inited with read-only var in `{}`",
            WIDGET.trace_id()
        );
    }
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create state properties
/// that map from a context variable or to create composite properties that merge other state properties.
pub fn bind_state<T: VarValue>(child: impl IntoUiNode, source: impl IntoVar<T>, state: impl IntoVar<T>) -> UiNode {
    let source = source.into_var();
    bind_state_init(child, state, move |state| {
        state.set_from(&source);
        source.bind(&state)
    })
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `bind` closure is called with the `state` variable, it must set and bind it.
pub fn bind_state_init<T>(
    child: impl IntoUiNode,
    state: impl IntoVar<T>,
    mut bind: impl FnMut(&Var<T>) -> VarHandle + Send + 'static,
) -> UiNode
where
    T: VarValue,
{
    let state = state.into_var();
    let mut _binding = VarHandle::dummy();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            _binding = bind(&state);
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
    child: impl IntoUiNode,
    predicate: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    deinit: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    state: impl IntoVar<bool>,
) -> UiNode {
    let state = state.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                state.set(s);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();
            let s = WIDGET.with_state(&deinit);
            if s != state.get() {
                state.set(s);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                state.set(s);
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
    child: impl IntoUiNode,
    get_new: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    get_deinit: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    state: impl IntoVar<T>,
) -> UiNode {
    let state = state.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                state.set(new);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();

            let new = state.with(|s| WIDGET.with_state(|w| get_deinit(w, s)));
            if let Some(new) = new {
                state.set(new);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                state.set(new);
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
pub fn fill_node(content: impl IntoUiNode) -> UiNode {
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
pub fn border_node(child: impl IntoUiNode, border_offsets: impl IntoVar<SideOffsets>, border_visual: impl IntoUiNode) -> UiNode {
    let offsets = border_offsets.into_var();
    let mut render_offsets = PxSideOffsets::zero();
    let mut border_rect = PxRect::zero();

    match_node(ui_vec![child, border_visual], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offsets).sub_var_render(&BORDER_OVER_VAR);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let offsets = offsets.layout();
            *desired_size = BORDER.measure_border(offsets, || {
                LAYOUT.with_sub_size(PxSize::new(offsets.horizontal(), offsets.vertical()), || {
                    children.node().with_child(0, |n| wm.measure_block(n))
                })
            });
            children.delegated();
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
                border_rect.size = LAYOUT.with_sub_size(taken_size, || children.node().with_child(0, |n| n.layout(wl)));

                // layout border visual
                LAYOUT.with_constraints(PxConstraints2d::new_exact_size(border_rect.size), || {
                    BORDER.with_border_layout(border_rect, offsets, || {
                        children.node().with_child(1, |n| n.layout(wl));
                    });
                });
            });
            children.delegated();

            *final_size = border_rect.size;
        }
        UiNodeOp::Render { frame } => {
            if BORDER_OVER_VAR.get() {
                children.node().with_child(0, |c| c.render(frame));
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.node().with_child(1, |c| c.render(frame));
                });
            } else {
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.node().with_child(1, |c| c.render(frame));
                });
                children.node().with_child(0, |c| c.render(frame));
            }
            children.delegated();
        }
        UiNodeOp::RenderUpdate { update } => {
            children.node().with_child(0, |c| c.render_update(update));
            BORDER.with_border_layout(border_rect, render_offsets, || {
                children.node().with_child(1, |c| c.render_update(update));
            });
            children.delegated();
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
    child: impl IntoUiNode,
    context: &'static ContextLocal<T>,
    value: impl Into<T>,
) -> UiNode {
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
    child: impl IntoUiNode,
    context: &'static ContextLocal<T>,
    init_value: impl FnMut() -> T + Send + 'static,
) -> UiNode {
    with_context_local_init_impl(child.into_node(), context, init_value)
}
fn with_context_local_init_impl<T: Any + Send + Sync + 'static>(
    child: UiNode,
    context: &'static ContextLocal<T>,
    mut init_value: impl FnMut() -> T + Send + 'static,
) -> UiNode {
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
/// node will delegate all node operations to inside the blend. The [`WidgetUiNode::with_context`]
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
/// [`LocalContext::with_context_blend`]: zng_app_context::LocalContext::with_context_blend
pub fn with_context_blend(mut ctx: LocalContext, over: bool, child: impl IntoUiNode) -> UiNode {
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
/// # use zng_app::{widget::{property, node::{UiNode, IntoUiNode}, WIDGET, WidgetUpdateMode}};
/// # use zng_var::IntoVar;
/// # use zng_wgt::node::with_widget_state;
/// # use zng_state_map::{StateId, static_id};
/// #
/// static_id! {
///     pub static ref FOO_ID: StateId<u32>;
/// }
///
/// #[property(CONTEXT)]
/// pub fn foo(child: impl IntoUiNode, value: impl IntoVar<u32>) -> UiNode {
///     with_widget_state(child, *FOO_ID, || 0, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &mut UiNode) -> u32 {
///     if let Some(mut wgt) = widget.as_widget() {
///         wgt.with_context(WidgetUpdateMode::Ignore, || WIDGET.get_state(*FOO_ID))
///             .unwrap_or(0)
///     } else {
///         0
///     }
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner() -> u32 {
///     WIDGET.get_state(*FOO_ID).unwrap_or_default()
/// }
/// ```
///
/// [`WIDGET`]: zng_app::widget::WIDGET
pub fn with_widget_state<U, I, T>(child: U, id: impl Into<StateId<T>>, default: I, value: impl IntoVar<T>) -> UiNode
where
    U: IntoUiNode,
    I: Fn() -> T + Send + 'static,
    T: StateValue + VarValue,
{
    with_widget_state_impl(child.into_node(), id.into(), default, value.into_var())
}
fn with_widget_state_impl<I, T>(child: UiNode, id: impl Into<StateId<T>>, default: I, value: impl IntoVar<T>) -> UiNode
where
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
pub fn with_widget_state_modify<U, S, V, I, M>(child: U, id: impl Into<StateId<S>>, value: impl IntoVar<V>, default: I, modify: M) -> UiNode
where
    U: IntoUiNode,
    S: StateValue,
    V: VarValue,
    I: Fn() -> S + Send + 'static,
    M: FnMut(&mut S, &V) + Send + 'static,
{
    with_widget_state_modify_impl(child.into_node(), id.into(), value.into_var(), default, modify)
}
fn with_widget_state_modify_impl<S, V, I, M>(
    child: UiNode,
    id: impl Into<StateId<S>>,
    value: impl IntoVar<V>,
    default: I,
    mut modify: M,
) -> UiNode
where
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
pub fn interactive_node(child: impl IntoUiNode, interactive: impl IntoVar<bool>) -> UiNode {
    let interactive = interactive.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&interactive);
        }
        UiNodeOp::Info { info } => {
            if interactive.get() {
                child.info(info);
            } else if let Some(mut wgt) = child.node().as_widget() {
                let id = wgt.id();
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
                        if let Some(parent) = args.info.parent()
                            && parent.id() == id
                        {
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
    child: impl IntoUiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> UiNode {
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
            if let Some(parent) = info.parent()
                && let Some(mut c) = PanelListRange::update(&parent, panel_list_id, &mut version)
            {
                let id = info.id();
                let p = c.position(|w| w.id() == id);
                update(p);
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the reverse index of the widget in the parent panel.
///
/// See [`with_index_len_node`] for more details.
pub fn with_rev_index_node(
    child: impl IntoUiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent()
                && let Some(c) = PanelListRange::update(&parent, panel_list_id, &mut version)
            {
                let id = info.id();
                let p = c.rev().position(|w| w.id() == id);
                update(p);
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
    child: impl IntoUiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<(usize, usize)>) + Send + 'static,
) -> UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent()
                && let Some(mut iter) = PanelListRange::update(&parent, panel_list_id, &mut version)
            {
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
        _ => {}
    })
}

/// Node that presents `data` using `wgt_fn`.
///
/// The node's child is always the result of `wgt_fn` called for the `data` value, it is reinited every time
/// either variable changes. If the child is an widget the node becomes it.
///
/// See also [`presenter_opt`] for a presenter that is nil with the data is `None`.
///
/// See also the [`present`](VarPresent::present) method that can be called on the `data`` variable and [`present_data`](VarPresentData::present_data)
/// that can be called on the `wgt_fn` variable.
pub fn presenter<D: VarValue>(data: impl IntoVar<D>, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
    let data = data.into_var();
    let wgt_fn = wgt_fn.into_var();

    match_widget(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&wgt_fn);
            *c.node() = wgt_fn.get()(data.get());
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || wgt_fn.is_new() {
                c.node().deinit();
                *c.node() = wgt_fn.get()(data.get());
                c.node().init();
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
///
/// See also the [`present_opt`](VarPresentOpt::present_opt) method that can be called on the data variable.
pub fn presenter_opt<D: VarValue>(data: impl IntoVar<Option<D>>, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
    let data = data.into_var();
    let wgt_fn = wgt_fn.into_var();

    match_widget(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&wgt_fn);
            if let Some(data) = data.get() {
                *c.node() = wgt_fn.get()(data);
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || wgt_fn.is_new() {
                if let Some(data) = data.get() {
                    c.node().deinit();
                    *c.node() = wgt_fn.get()(data);
                    c.node().init();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                } else if !c.node().is_nil() {
                    c.node().deinit();
                    *c.node() = UiNode::nil();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                }
            }
        }
        _ => {}
    })
}

/// Node list that presents `list` using `item_fn` for each new list item.
///
/// The node's children is the list mapped to node items, it is kept in sync, any list update is propagated to the node list.
///
/// See also the [`present_list`](VarPresentList::present_list) method that can be called on the list variable.
pub fn list_presenter<D: VarValue>(list: impl IntoVar<ObservableVec<D>>, item_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
    ListPresenter {
        list: list.into_var(),
        item_fn: item_fn.into_var(),
        view: ui_vec![],
        _e: std::marker::PhantomData,
    }
    .into_node()
}

/// Node list that presents `list` using `item_fn` for each list item.
///
/// The node's children are **regenerated** for each change in `list`, if possible prefer using [`ObservableVec`] with [`list_presenter`].
///
/// See also the [`present_list_from_iter`](VarPresentListFromIter::present_list_from_iter) method that can be called on the list variable.
pub fn list_presenter_from_iter<D, L>(list: impl IntoVar<L>, item_fn: impl IntoVar<WidgetFn<D>>) -> UiNode
where
    D: VarValue,
    L: IntoIterator<Item = D> + VarValue,
{
    ListPresenterFromIter {
        list: list.into_var(),
        item_fn: item_fn.into_var(),
        view: ui_vec![],
        _e: std::marker::PhantomData,
    }
    .into_node()
}

struct ListPresenter<D>
where
    D: VarValue,
{
    list: Var<ObservableVec<D>>,
    item_fn: Var<WidgetFn<D>>,
    view: UiVec,
    _e: std::marker::PhantomData<D>,
}

impl<D> UiNodeImpl for ListPresenter<D>
where
    D: VarValue,
{
    fn children_len(&self) -> usize {
        self.view.len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.view.with_child(index, visitor)
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.view.for_each_child(visitor);
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> std::ops::ControlFlow<BoxAnyVarValue>,
    ) -> std::ops::ControlFlow<BoxAnyVarValue> {
        self.view.try_for_each_child(visitor)
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.view.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.view.par_fold_reduce(identity, fold, reduce)
    }

    fn init(&mut self) {
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

        self.view.init();
    }

    fn deinit(&mut self) {
        self.view.deinit();
        self.view.clear();
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.update_list(updates, &mut ());
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut need_reset = self.item_fn.is_new();

        let is_new = self
            .list
            .with_new(|l| {
                need_reset |= l.changes().is_empty() || l.changes() == [VecChange::Clear];

                if need_reset {
                    return;
                }

                // update before new items to avoid update before init.
                self.view.update_list(updates, observer);

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
            self.view.deinit();
            self.view.clear();

            let e_fn = self.item_fn.get();
            self.list.with(|l| {
                for el in l.iter() {
                    let child = e_fn(el.clone());
                    self.view.push(child);
                }
            });

            self.view.init();
        } else if !is_new {
            self.view.update_list(updates, observer);
        }
    }

    fn info(&mut self, info: &mut zng_app::widget::info::WidgetInfoBuilder) {
        self.view.info(info);
    }

    fn measure(&mut self, wm: &mut zng_app::widget::info::WidgetMeasure) -> PxSize {
        self.view.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut zng_app::widget::info::WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut zng_app::widget::info::WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.view.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut zng_app::widget::info::WidgetLayout) -> PxSize {
        self.view.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut zng_app::widget::info::WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut zng_app::widget::info::WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.view.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.view.render(frame);
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.view.render_list(frame, render);
    }

    fn render_update(&mut self, update: &mut zng_app::render::FrameUpdate) {
        self.view.render_update(update);
    }

    fn render_update_list(
        &mut self,
        update: &mut zng_app::render::FrameUpdate,
        render_update: &(dyn Fn(usize, &mut UiNode, &mut zng_app::render::FrameUpdate) + Sync),
    ) {
        self.view.render_update_list(update, render_update);
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

struct ListPresenterFromIter<D, L>
where
    D: VarValue,
    L: IntoIterator<Item = D> + VarValue,
{
    list: Var<L>,
    item_fn: Var<WidgetFn<D>>,
    view: UiVec,
    _e: std::marker::PhantomData<(D, L)>,
}

impl<D, L> UiNodeImpl for ListPresenterFromIter<D, L>
where
    D: VarValue,
    L: IntoIterator<Item = D> + VarValue,
{
    fn children_len(&self) -> usize {
        self.view.len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.view.with_child(index, visitor)
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.view.for_each_child(visitor)
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> std::ops::ControlFlow<BoxAnyVarValue>,
    ) -> std::ops::ControlFlow<BoxAnyVarValue> {
        self.view.try_for_each_child(visitor)
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.view.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.view.par_fold_reduce(identity, fold, reduce)
    }

    fn is_list(&self) -> bool {
        true
    }

    fn init(&mut self) {
        debug_assert!(self.view.is_empty());
        self.view.clear();

        WIDGET.sub_var(&self.list).sub_var(&self.item_fn);

        let e_fn = self.item_fn.get();

        self.view.extend(self.list.get().into_iter().map(&*e_fn));
        self.view.init();
    }

    fn deinit(&mut self) {
        self.view.deinit();
        self.view.clear();
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.update_list(updates, &mut ())
    }
    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if self.list.is_new() || self.item_fn.is_new() {
            self.view.deinit();
            self.view.clear();
            let e_fn = self.item_fn.get();
            self.view.extend(self.list.get().into_iter().map(&*e_fn));
            self.view.init();
            observer.reset();
        } else {
            self.view.update_list(updates, observer);
        }
    }

    fn info(&mut self, info: &mut zng_app::widget::info::WidgetInfoBuilder) {
        self.view.info(info)
    }

    fn measure(&mut self, wm: &mut zng_app::widget::info::WidgetMeasure) -> PxSize {
        self.view.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut zng_app::widget::info::WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut zng_app::widget::info::WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.view.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut zng_app::widget::info::WidgetLayout) -> PxSize {
        self.view.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut zng_app::widget::info::WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut zng_app::widget::info::WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.view.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.view.render(frame);
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.view.render_list(frame, render);
    }

    fn render_update(&mut self, update: &mut zng_app::render::FrameUpdate) {
        self.view.render_update(update);
    }

    fn render_update_list(
        &mut self,
        update: &mut zng_app::render::FrameUpdate,
        render_update: &(dyn Fn(usize, &mut UiNode, &mut zng_app::render::FrameUpdate) + Sync),
    ) {
        self.view.render_update_list(update, render_update);
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// Extension method to *convert* a variable to a node.
pub trait VarPresent<D: VarValue> {
    /// Present the variable data using a [`presenter`] node.
    fn present(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode;
}
impl<D: VarValue> VarPresent<D> for Var<D> {
    fn present(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
        presenter(self.clone(), wgt_fn)
    }
}

/// Extension method to *convert* a variable to a node.
pub trait VarPresentOpt<D: VarValue> {
    /// Present the variable data using a [`presenter_opt`] node.
    fn present_opt(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode;
}
impl<D: VarValue> VarPresentOpt<D> for Var<Option<D>> {
    fn present_opt(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
        presenter_opt(self.clone(), wgt_fn)
    }
}

/// Extension method fo *convert* a variable to a node list.
pub trait VarPresentList<D: VarValue> {
    /// Present the variable data using a [`list_presenter`] node list.
    fn present_list(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode;
}
impl<D: VarValue> VarPresentList<D> for Var<ObservableVec<D>> {
    fn present_list(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
        list_presenter(self.clone(), wgt_fn)
    }
}

/// Extension method fo *convert* a variable to a node list.
pub trait VarPresentListFromIter<D: VarValue, L: IntoIterator<Item = D> + VarValue> {
    /// Present the variable data using a [`list_presenter_from_iter`] node list.
    fn present_list_from_iter(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode;
}
impl<D: VarValue, L: IntoIterator<Item = D> + VarValue> VarPresentListFromIter<D, L> for Var<L> {
    fn present_list_from_iter(&self, wgt_fn: impl IntoVar<WidgetFn<D>>) -> UiNode {
        list_presenter_from_iter(self.clone(), wgt_fn)
    }
}

/// Extension method to *convert* a variable to a node.
pub trait VarPresentData<D: VarValue> {
    /// Present the `data` variable using a [`presenter`] node.
    fn present_data(&self, data: impl IntoVar<D>) -> UiNode;
}
impl<D: VarValue> VarPresentData<D> for Var<WidgetFn<D>> {
    fn present_data(&self, data: impl IntoVar<D>) -> UiNode {
        presenter(data, self.clone())
    }
}
