//! Event handler properties, [`on_click`](gesture::on_click), [`on_key_down`](keyboard::on_key_down),
//! [`on_focus`](focus::on_focus) and more.

use crate::core::context::*;
use crate::core::event::*;
use crate::core::impl_ui_node;
use crate::core::profiler::profile_scope;
use crate::core::UiNode;
use crate::properties::IsEnabled;

pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod widget;

struct OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.child.update_hp(ctx);

        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

struct OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update(ctx);
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update_hp(ctx);
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_pre_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => { paste::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview Event
        ///
        #[doc = "You can preview this event using [`on_pre_" $event "`]."]
        #[$crate::core::property(event)]
        $vis fn [<on_ $event>](
            child: impl $crate::core::UiNode,
            handler: impl FnMut(&mut $crate::core::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::core::UiNode {
            $crate::properties::events::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [on_" $event "] event."]
        ///
        /// # Preview Events
        ///
        /// Preview events are fired before the main event, if you stop the propagation of a preview event
        /// the main event does not run. See [`on_pre_event`](zero_ui::properties::events::on_pre_event) for more details.
        #[$crate::core::property(event)]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::core::UiNode,
            handler: impl FnMut(&mut $crate::core::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::core::UiNode {
            $crate::properties::events::on_pre_event(child, $Event, $filter, handler)
        }
    } };
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                filter: |ctx, args| $crate::core::event::EventArgs::concerns_widget(args, ctx),
            }
        }
    };
}
/// Declare one or more event properties.
///
/// Each declaration expands to a pair of properties `on_$event` and `on_pre_$event`. The preview property
/// calls [`on_pre_event`](zero_ui::properties::events::on_pre_event),
/// the main event property calls [`on_event`](zero_ui::properties::events::on_event).
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui::properties::events::event_property;
/// # use zero_ui::core::event::EventArgs;
/// # use zero_ui::core::keyboard::*;
/// event_property! {
///     /// on_key_down docs.
///     pub fn key_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| args.concerns_widget(ctx)
///     }
///
///     pub(crate) fn space_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.concerns_widget(ctx) && args.key == Some(Key::Space),
///     }
/// }
/// ```
///
/// # Filter
///
/// App events can be listened from any `UiNode`. An event property must call the event handler only
/// in contexts where the event is relevant. Some event properties can also specialize further on top
/// of a more general app event. To implement this you can use a filter predicate.
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called. See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default [`args.concerns_widget(ctx)`](EventArgs::concerns_widget) is used.
/// So if you want to extend the filter and not fully replace it you must call `args.concerns_widget(ctx)` in your custom filter.
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                $(filter: $filter,)?
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
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
#[inline]
pub fn on_event<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}

/// Helper for declaring preview event properties with a custom filter.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
pub fn on_pre_event<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnPreviewEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}
