//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

use std::{
    any::{type_name, Any, TypeId},
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    thread::LocalKey,
};

use crate::{
    context::{OwnedStateMap, StateMap, WidgetContext},
    crate_util::{Handle, HandleOwner},
    event::{Event, Events, WithEvents},
    handler::WidgetHandler,
    impl_ui_node, state_key,
    text::Text,
    var::{var, var_from, BoxedVar, ContextVar, IntoVar, RcVar, ReadOnlyVar, Var, Vars},
    UiNode,
};

/// Declares new [`Command`](crate::command::Command) types.
#[macro_export]
macro_rules! command {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Command:ident $(
                 .$init:ident( $($args:tt)* )
        )*;
    )+) => {$(

        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Command;
        impl $Command {
            std::thread_local! {
                static COMMAND: $crate::command::CommandValue = $crate::command::CommandValue::init::<$Command, _>(||{
                    #[allow(path_statements)] {
                        $Command $(
                        .$init( $($args)* )
                        )*;
                    }
                });
            }

            /// Gets the event arguments if the update is for this event.
            #[inline(always)]
            #[allow(unused)]
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                <Self as $crate::event::Event>::update(self, args)
            }

            /// Schedule an event update if the command is enabled.
            ///
            /// The `parameter` is an optional value for the command handler.
            ///
            /// Returns `true` if notified, only notifies if the command is enabled.
            #[inline]
            #[allow(unused)]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, parameter: Option<std::rc::Rc<dyn std::any::Any>>) -> bool {
                let enabled = Self::COMMAND.with(|c| c.enabled_value());
                if enabled {
                    events.with_events(|evs| evs.notify::<Self>($crate::command::CommandArgs::now(parameter)));
                }
                enabled
            }

            /// Gets a read-only variable that indicates if the command has at least one enabled handler.
            ///
            /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
            /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
            /// visible but disabled.
            #[inline]
            #[allow(unused)]
            pub fn enabled(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::enabled(self)
            }

            /// Gets a read-only variable that indicates if the command has at least one handler.
            ///
            /// When this is `false` the command can be considered *not relevant* in the current app state
            /// and associated command trigger widgets can be hidden.
            #[inline]
            #[allow(unused)]
            pub fn has_handlers(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::has_handlers(self)
            }

            /// Create a new handle to this command.
            ///
            /// A handle indicates that there is an active *handler* for the event, the handle can also
            /// be used to set the [`enabled`](Self::enabled) state.
            #[inline]
            #[allow(unused)]
            pub fn new_handle<Evs: $crate::event::WithEvents>(self, events: &mut Evs, enabled: bool) -> $crate::command::CommandHandle {
                <Self as $crate::command::Command>::new_handle(self, events, enabled)
            }
        }
        impl $crate::event::Event for $Command {
            type Args = $crate::command::CommandArgs;

            #[inline(always)]
            fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: Self::Args) {
                if Self::COMMAND.with(|c| c.enabled_value()) {
                    events.with_events(|evs| evs.notify::<Self>(args));
                }
            }
        }
        impl $crate::command::Command for $Command {
            #[inline]
            fn with_meta<F, R>(self, f: F) -> R
            where
                F: FnOnce(&mut $crate::context::StateMap) -> R,
            {
                Self::COMMAND.with(|c| c.with_meta(f))
            }

            #[inline]
            fn enabled(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                Self::COMMAND.with(|c| c.enabled())
            }

            #[inline]
            fn enabled_value(self) -> bool {
                Self::COMMAND.with(|c| c.enabled_value())
            }

            #[inline]
            fn has_handlers(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                Self::COMMAND.with(|c| c.has_handlers())
            }

            #[inline]
            fn has_handlers_value(self) -> bool {
                Self::COMMAND.with(|c| c.has_handlers_value())
            }

            #[inline]
            fn new_handle<Evs: $crate::event::WithEvents>(self, events: &mut Evs, enabled: bool) -> $crate::command::CommandHandle {
                Self::COMMAND.with(|c| c.new_handle(events, &Self::COMMAND, enabled))
            }

            #[inline]
            fn as_any(self) -> $crate::command::AnyCommand {
                $crate::command::AnyCommand::new(&Self::COMMAND)
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use [`command!`](macro@crate::command::command) to declare.
pub trait Command: Event<Args = CommandArgs> {
    /// Runs `f` with access to the metadata state-map.
    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R;

    /// Gets a read-only variable that indicates if the command has at least one enabled handler.
    ///
    /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
    /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
    /// visible but disabled.
    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>>;

    /// Gets if the command has at least one enabled handler.
    fn enabled_value(self) -> bool;

    /// Gets a read-only variable that indicates if the command has at least one handler.
    ///
    /// When this is `false` the command can be considered *not relevant* in the current app state
    /// and associated command trigger widgets can be hidden.
    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>>;

    /// Gets if the command has at least one handler.
    fn has_handlers_value(self) -> bool;

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle;

    /// Gets a [`AnyCommand`] that represents this command.
    fn as_any(self) -> AnyCommand;
}

/// Represents a [`Command`] type.
#[derive(Clone, Copy)]
pub struct AnyCommand(&'static LocalKey<CommandValue>);
impl AnyCommand {
    #[inline]
    #[doc(hidden)]
    pub fn new(c: &'static LocalKey<CommandValue>) -> Self {
        AnyCommand(c)
    }

    pub(crate) fn update_state(&self, vars: &Vars) {
        self.0.with(|c| c.update_state(vars))
    }

    /// Gets the [`TypeId`] of the command represented by `self`.
    #[inline]
    pub fn command_type_id(self) -> TypeId {
        self.0.with(|c| c.command_type_id)
    }

    /// Gets the [`type_name`] of the command represented by `self`.
    #[inline]
    pub fn command_type_name(self) -> &'static str {
        self.0.with(|c| c.command_type_name)
    }

    /// If the command `C` is represented by `self`.
    #[inline]
    pub fn is<C: Command>(self) -> bool {
        self.command_type_id() == TypeId::of::<C>()
    }

    /// Schedule an event update for the command represented by `self`.
    #[inline]
    pub fn notify(self, events: &mut Events, parameter: Option<Rc<dyn Any>>) {
        Event::notify(self, events, CommandArgs::now(parameter))
    }
}
impl fmt::Debug for AnyCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any {}", self.command_type_name())
    }
}
impl Event for AnyCommand {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        self.0.with(|c| {
            if c.enabled_value() {
                events.with_events(|e| (c.notify)(e, args))
            }
        });
    }
    fn update<U: crate::event::EventUpdateArgs>(self, _: &U) -> Option<&crate::event::EventUpdate<Self>> {
        panic!("`AnyCommand` does not support `Event::update`");
    }
}

impl Command for AnyCommand {
    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R,
    {
        self.0.with(move |c| c.with_meta(f))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.0.with(|c| c.enabled())
    }

    fn enabled_value(self) -> bool {
        self.0.with(|c| c.enabled_value())
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.0.with(|c| c.has_handlers())
    }

    fn has_handlers_value(self) -> bool {
        self.0.with(|c| c.has_handlers_value())
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        self.0.with(|c| c.new_handle(events, self.0, enabled))
    }

    fn as_any(self) -> AnyCommand {
        self
    }
}

/// Adds the [`name`](CommandNameExt) metadata.
pub trait CommandNameExt: Command {
    /// Gets a read-write variable that is the display name for the command.
    fn name(self) -> RcVar<Text>;

    /// Sets the initial name if it is not set.
    fn init_name(self, name: impl Into<Text>) -> Self;

    /// Gets a read-only variable that formats the name and first shortcut in the following format: name (first_shortcut)
    /// Note: If no shortcuts are available this method returns the same as [`name`](Self::name)
    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt;
}
state_key! {
    struct CommandNameKey: RcVar<Text>;
}
impl<C: Command> CommandNameExt for C {
    fn name(self) -> RcVar<Text> {
        self.with_meta(|m| {
            let var = m.entry::<CommandNameKey>().or_insert_with(|| {
                let name = type_name::<C>();
                var_from(name.strip_suffix("Command").unwrap_or(name))
            });
            var.clone()
        })
    }

    fn init_name(self, name: impl Into<Text>) -> Self {
        self.with_meta(|m| {
            let entry = m.entry::<CommandNameKey>();
            entry.or_insert_with(|| var(name.into()));
        });
        self
    }

    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt,
    {
        crate::merge_var!(self.name(), self.shortcut(), |name, shortcut| {
            if shortcut.is_empty() {
                name.clone()
            } else {
                crate::formatx!("{} ({})", name, shortcut[0])
            }
        })
        .boxed()
    }
}

/// Adds the [`info`](CommandInfoExt) metadata.
pub trait CommandInfoExt: Command {
    /// Gets a read-write variable that is a short informational string about the command.
    fn info(self) -> RcVar<Text>;

    /// Sets the initial info if it is not set.
    fn init_info(self, info: impl Into<Text>) -> Self;
}
state_key! {
    struct CommandInfoKey: RcVar<Text>;
}
impl<C: Command> CommandInfoExt for C {
    fn info(self) -> RcVar<Text> {
        self.with_meta(|m| m.entry::<CommandInfoKey>().or_insert_with(|| var_from("")).clone())
    }

    fn init_info(self, info: impl Into<Text>) -> Self {
        self.with_meta(|m| {
            m.entry::<CommandInfoKey>().or_insert_with(|| var(info.into()));
        });
        self
    }
}

/// A handle to a [`Command`].
///
/// Holding the command handle indicates that the command is relevant in the current app state.
/// The handle needs to be enabled to indicate that the command can be issued.
///
/// You can use the [`Command::new_handle`] method in a command type to create a handle.
pub struct CommandHandle {
    handle: Handle<AtomicUsize>,
    local_enabled: Cell<bool>,
}
impl CommandHandle {
    /// Sets if the command event handler is active.
    ///
    /// When at least one [`CommandHandle`] is enabled the command is [`enabled`](Command::enabled).
    pub fn set_enabled(&self, enabled: bool) {
        if self.local_enabled.get() != enabled {
            self.local_enabled.set(enabled);
            if enabled {
                self.handle.data().fetch_add(1, Ordering::Relaxed);
            } else {
                self.handle.data().fetch_sub(1, Ordering::Relaxed);
            };
        }
    }

    /// Returns a dummy [`CommandHandle`] that is not connected to any command.
    pub fn dummy() -> Self {
        CommandHandle {
            handle: Handle::dummy(AtomicUsize::new(0)),
            local_enabled: Cell::new(false),
        }
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        self.set_enabled(false);
    }
}

#[doc(hidden)]
pub struct CommandValue {
    command_type_id: TypeId,
    command_type_name: &'static str,
    handle: HandleOwner<AtomicUsize>,
    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: RefCell<OwnedStateMap>,
    meta_init: Cell<Option<Box<dyn FnOnce()>>>,
    registered: Cell<bool>,
    notify: Box<dyn Fn(&mut Events, CommandArgs)>,
}
#[allow(missing_docs)] // this is all hidden
impl CommandValue {
    pub fn init<C: Command, I: FnOnce() + 'static>(meta_init: I) -> Self {
        CommandValue {
            command_type_id: TypeId::of::<C>(),
            command_type_name: type_name::<C>(),
            handle: HandleOwner::dropped(AtomicUsize::new(0)),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
            meta_init: Cell::new(Some(Box::new(meta_init))),
            registered: Cell::new(false),
            notify: Box::new(|events, args| events.notify::<C>(args)),
        }
    }

    fn update_state(&self, vars: &Vars) {
        self.has_handlers.set_ne(vars, self.has_handlers_value());
        self.enabled.set_ne(vars, self.enabled_value());
    }

    pub fn new_handle<Evs: WithEvents>(&self, events: &mut Evs, key: &'static LocalKey<CommandValue>, enabled: bool) -> CommandHandle {
        if !self.registered.get() {
            self.registered.set(true);
            events.with_events(|e| e.register_command(AnyCommand(key)));
        }
        let r = CommandHandle {
            handle: self.handle.reanimate(),
            local_enabled: Cell::new(false),
        };
        if enabled {
            r.set_enabled(true);
        }
        r
    }

    pub fn enabled(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.enabled.clone())
    }

    pub fn enabled_value(&self) -> bool {
        self.has_handlers_value() && self.handle.data().load(Ordering::Relaxed) > 0
    }

    pub fn has_handlers(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.has_handlers.clone())
    }

    pub fn has_handlers_value(&self) -> bool {
        !self.handle.is_dropped()
    }

    pub fn with_meta<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R,
    {
        if let Some(init) = self.meta_init.take() {
            init()
        }
        f(&mut self.meta.borrow_mut().0)
    }
}

crate::event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub parameter: Option<Rc<dyn Any>>,

        ..

        fn concerns_widget(&self, _: &mut WidgetContext) -> bool {
            true
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    #[inline]
    pub fn parameter<T: Any>(&self) -> Option<&T> {
        self.parameter.as_ref().and_then(|p| p.downcast_ref::<T>())
    }
}

/// Helper for declaring command properties.
#[inline]
pub fn on_command<U, C, E, H>(child: U, command: C, enabled: E, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    E: IntoVar<bool>,
    H: WidgetHandler<CommandArgs>,
{
    struct OnCommandNode<U, C, E, H> {
        child: U,
        command: C,
        enabled: E,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, E, H> UiNode for OnCommandNode<U, C, E, H>
    where
        U: UiNode,
        C: Command,
        E: Var<bool>,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            let enabled = self.enabled.copy(ctx);
            self.handle = Some(self.command.new_handle(ctx, enabled));
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.update(args) {
                self.child.event(ctx, args);

                if !args.stop_propagation_requested() && self.enabled.copy(ctx) {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
        }
    }
    OnCommandNode {
        child,
        command,
        enabled: enabled.into_var(),
        handler,
        handle: None,
    }
}

/// Helper for declaring command properties.
#[inline]
pub fn on_pre_command<U, C, E, H>(child: U, command: C, enabled: E, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    E: IntoVar<bool>,
    H: WidgetHandler<CommandArgs>,
{
    struct OnPreviewCommandNode<U, C, E, H> {
        child: U,
        command: C,
        enabled: E,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, E, H> UiNode for OnPreviewCommandNode<U, C, E, H>
    where
        U: UiNode,
        C: Command,
        E: Var<bool>,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let enabled = self.enabled.copy(ctx);
            self.handle = Some(self.command.new_handle(ctx, enabled));
            self.child.init(ctx);
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.update(args) {
                if !args.stop_propagation_requested() && self.enabled.copy(ctx) {
                    self.handler.event(ctx, args);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
            self.child.update(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
        }
    }
    OnPreviewCommandNode {
        child,
        command,
        enabled: enabled.into_var(),
        handler,
        handle: None,
    }
}

/// Helper for declaring command properties.
pub fn can_command(
    child: impl UiNode,
    enabled: impl ContextVar<Type = bool>,
    update: impl FnMut(&mut WidgetContext) -> bool + 'static,
) -> impl UiNode {
    struct CanCommandNode<C, E, U> {
        child: C,
        enabled: E,
        update: U,

        enabled_value: bool,
        enabled_version: u32,
    }
    #[impl_ui_node(child)]
    impl<C, E, U> UiNode for CanCommandNode<C, E, U>
    where
        C: UiNode,
        E: ContextVar<Type = bool>,
        U: FnMut(&mut WidgetContext) -> bool + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let value = (self.update)(ctx);
            let is_new = self.enabled_value != value;
            if is_new {
                self.enabled_value = value;
                self.enabled_version = self.enabled_version.wrapping_add(1);
            }

            ctx.vars.with_context_var(self.enabled, &value, is_new, self.enabled_version, || {
                self.child.init(ctx);
            });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let value = (self.update)(ctx);
            let is_new = self.enabled_value != value;
            if is_new {
                self.enabled_value = value;
                self.enabled_version = self.enabled_version.wrapping_add(1);
            }

            ctx.vars.with_context_var(self.enabled, &value, is_new, self.enabled_version, || {
                self.child.update(ctx);
            });
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, false, self.enabled_version, || {
                self.child.deinit(ctx);
            });
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, false, self.enabled_version, || {
                self.child.event(ctx, args);
            });
        }

        fn measure(
            &mut self,
            ctx: &mut crate::context::LayoutContext,
            available_size: crate::units::LayoutSize,
        ) -> crate::units::LayoutSize {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, self.enabled_version, || {
                self.child.measure(ctx, available_size)
            })
        }

        fn arrange(&mut self, ctx: &mut crate::context::LayoutContext, final_size: crate::units::LayoutSize) {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, self.enabled_version, || {
                self.child.arrange(ctx, final_size);
            });
        }

        fn render(&self, ctx: &mut crate::context::RenderContext, frame: &mut crate::render::FrameBuilder) {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, self.enabled_version, || {
                self.child.render(ctx, frame);
            });
        }

        fn render_update(&self, ctx: &mut crate::context::RenderContext, update: &mut crate::render::FrameUpdate) {
            let value = self.enabled_value;
            ctx.vars.with_context_var(self.enabled, &value, self.enabled_version, || {
                self.child.render_update(ctx, update);
            });
        }
    }
    CanCommandNode {
        child,
        enabled,
        update,

        enabled_value: false,
        enabled_version: 0,
    }
}

/// Declare command properties.
#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$on_command_attrs:meta])*
        $vis:vis fn $command:ident: $Command:path;
    )+) => {$($crate::paste! {

        $crate::var::context_var! {
            struct [<Can $Command Var>]: bool = const true;
        }

        $(#[$on_command_attrs])*
        ///
        /// # Enable
        ///
        #[doc = "You can control if this property is enabled by setting the [`can_"$command"`](fn.can_"$command".html)."]
        /// property in the same widget or a parent widget.
        ///
        /// # Preview
        ///
        #[doc = "You can preview this command event using [`on_pre_"$command"`](fn.on_pre_"$command".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        pub fn [<on_ $command>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::command::CommandArgs>
        ) -> impl $crate::UiNode {
            $crate::command::on_command(child, $Command, [<Can $Command Var>], handler)
        }

        #[doc = "Preview [`on_"$command"`](fn.on_"$command".html) command event."]
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
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        pub fn [<on_pre_ $command>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::command::CommandArgs>
        ) -> impl $crate::UiNode {
            $crate::command::on_pre_command(child, $Command, [<Can $Command Var>], handler)
        }

        #[doc = "Enable/Disable the [`on_"$command"`](fn.on_"$command".html) command event in the widget or its content."]
        ///
        /// # Commands
        ///
        /// TODO
        #[$crate::property(context, allowed_in_when = false, default( |_|true ))]
        pub fn [<can_ $command>](
            child: impl $crate::UiNode,
            update: impl FnMut(&mut $crate::context::WidgetContext) -> bool + 'static
        ) -> impl $crate::UiNode {
            $crate::command::can_command(child, [<Can $Command Var>], update)
        }
    })+}
}
#[doc(inline)]
pub use crate::command_property;

#[cfg(test)]
mod tests {
    use super::{command, CommandArgs};

    command! {
        FooCommand;
        BarCommand;
    }

    #[test]
    fn parameter_none() {
        let _ = CommandArgs::now(None);
    }
}
