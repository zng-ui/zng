//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

use std::{
    any::type_name,
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    context::{OwnedStateMap, StateMap},
    event::{Event, Events},
    state_key,
    text::Text,
    var::{var, var_from, RcVar, ReadOnlyVar, Vars},
};

/// Declares new [`Command`](crate::event::Command) types.
#[macro_export]
macro_rules! command {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Command:ident $(: $Args:path)?
    );+$(;)?) => {$(
        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Command;
        impl $Command {
            std::thread_local! {
                static COMMAND: $crate::command::CommandValue<$Command> = $crate::command::CommandValue::init();
            }

            /// Gets the event arguments if the update is for this event.
            #[inline(always)]
            pub fn update<U: $crate::event::EventUpdateArgs>(args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                <Self as $crate::event::Event>::update(args)
            }

            /// Schedule an event update if the command is enabled.
            #[inline]
            pub fn notify(events: &mut $crate::event::Events, args: $crate::command::CommandArgs) {
                <Self as $crate::event::Event>::notify(events, args);
            }
        }
        impl $crate::event::Event for $Command {
            type Args = $crate::command::CommandArgs;// TODO $Args

            #[inline(always)]
            fn notify(events: &mut $crate::event::Events, args: Self::Args) {
                if Self::COMMAND.with(|c| c.handle.enabled.get() > 0) {
                    events.notify::<Self>(args);
                }
            }
        }
        impl $crate::command::Command for $Command {
            #[inline]
            fn with_meta<F, R>(f: F) -> R
            where
                F: FnOnce(&mut $crate::context::StateMap) -> R,
            {
                Self::COMMAND.with(|c| c.with_meta(f))
            }

            #[inline]
            fn enabled() -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                Self::COMMAND.with(|c| c.enabled())
            }

            #[inline]
            fn has_handlers() -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                Self::COMMAND.with(|c| c.has_handlers())
            }

            #[inline]
            fn new_handle(events: &mut $crate::event::Events) -> $crate::command::CommandHandle {
                Self::COMMAND.with(|c| c.new_handle(events))
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use [`command!`](crate::event::command) to declare.
pub trait Command: Event {
    fn with_meta<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R;

    fn enabled() -> ReadOnlyVar<bool, RcVar<bool>>;

    fn has_handlers() -> ReadOnlyVar<bool, RcVar<bool>>;

    fn new_handle(events: &mut Events) -> CommandHandle;
}

state_key! {
    struct CommandLabelKey: RcVar<Text>;
}

pub trait CommandLabelExt: Command {
    fn label() -> RcVar<Text>;
}
impl<C: Command> CommandLabelExt for C {
    fn label() -> RcVar<Text> {
        C::with_meta(|m| {
            let entry = m.entry::<CommandLabelKey>();
            let var = entry.or_insert_with(|| var_from(type_name::<C>()));
            var.clone()
        })
    }
}

pub struct CommandHandle {
    handle: Rc<CommandHandleData>,
    local_enabled: Cell<bool>,
}
struct CommandHandleData {
    enabled: Cell<usize>,
}
impl CommandHandle {
    pub fn set_enabled(&self, enabled: bool) {
        if self.local_enabled.get() != enabled {
            self.local_enabled.set(enabled);
            let new_count = if enabled {
                self.handle.enabled.get() + 1
            } else {
                self.handle.enabled.get() - 1
            };
            self.handle.enabled.set(new_count);
        }
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        self.set_enabled(false);
    }
}

#[doc(hidden)]
pub struct CommandValue<C: Command> {
    _c: PhantomData<C>,
    handle: Rc<CommandHandleData>,
    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: RefCell<OwnedStateMap>,
}
#[allow(missing_docs)] // this is all hidden
impl<C: Command> CommandValue<C> {
    pub fn init() -> Self {
        CommandValue {
            _c: PhantomData,
            handle: Rc::new(CommandHandleData { enabled: Cell::new(0) }),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
        }
    }

    fn update_state(&self, vars: &Vars) {
        let has_handlers = Rc::strong_count(&self.handle) > 1;
        let enabled = self.handle.enabled.get() > 0;

        self.has_handlers.set_ne(vars, has_handlers);
        self.enabled.set_ne(vars, enabled);
    }

    pub fn new_handle(&self, events: &mut Events) -> CommandHandle {
        CommandHandle {
            handle: Rc::clone(&self.handle),
            local_enabled: Cell::new(false),
        }
    }

    pub fn enabled(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.enabled.clone())
    }

    pub fn has_handlers(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.has_handlers.clone())
    }

    pub fn with_meta<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R,
    {
        f(&mut self.meta.borrow_mut().0)
    }
}
crate::event_args! {
    pub struct CommandArgs{
        ..
        fn concerns_widget(&self, _: &mut WidgetContext) -> bool {
            true
        }
    }
}
