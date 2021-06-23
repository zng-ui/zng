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
    context::{OwnedStateMap, StateMap},
    crate_util::{Handle, HandleOwner},
    event::{Event, Events, WithEvents},
    state_key,
    text::Text,
    var::{var, var_from, RcVar, ReadOnlyVar, Vars},
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
            /// `parameter` is an optional value for the command handler.
            #[inline]
            #[allow(unused)]
            pub fn notify(self, events: &mut $crate::event::Events, parameter: Option<std::rc::Rc<dyn std::any::Any>>) {
                <Self as $crate::event::Event>::notify(self, events, $crate::command::CommandArgs::now(parameter));
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
            fn new_handle(self, events: &mut $crate::event::Events) -> $crate::command::CommandHandle {
                Self::COMMAND.with(|c| c.new_handle(events, &Self::COMMAND))
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
    fn new_handle(self, events: &mut Events) -> CommandHandle;

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
    pub fn notify(self, events: &mut Events, args: CommandArgs) {
        Event::notify(self, events, args)
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

    fn new_handle(self, events: &mut Events) -> CommandHandle {
        self.0.with(|c| c.new_handle(events, self.0))
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
}
state_key! {
    struct CommandNameKey: RcVar<Text>;
}
impl<C: Command> CommandNameExt for C {
    fn name(self) -> RcVar<Text> {
        self.with_meta(|m| {
            let entry = m.entry::<CommandNameKey>();
            let var = entry.or_insert_with(|| var_from(type_name::<C>()));
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
        self.with_meta(|m| {
            let entry = m.entry::<CommandInfoKey>();
            let var = entry.or_insert_with(|| var_from(""));
            var.clone()
        })
    }

    fn init_info(self, info: impl Into<Text>) -> Self {
        self.with_meta(|m| {
            let entry = m.entry::<CommandInfoKey>();
            entry.or_insert_with(|| var(info.into()));
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

    pub fn new_handle(&self, events: &mut Events, key: &'static LocalKey<CommandValue>) -> CommandHandle {
        if self.registered.get() {
            self.registered.set(true);
            events.register_command(AnyCommand(key));
        }
        CommandHandle {
            handle: self.handle.reanimate(),
            local_enabled: Cell::new(false),
        }
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
