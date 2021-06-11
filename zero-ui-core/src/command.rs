//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

use std::{any::type_name, cell::Cell, marker::PhantomData, rc::Rc, thread::LocalKey};

use crate::{
    context::OwnedStateMap,
    event::{Event, Events},
    text::Text,
    var::{var, var_from, RcVar, ReadOnlyVar, Vars},
};

/// Declares new [`Command`](crate::event::Command) types.
#[macro_export]
macro_rules! command {
    () => {
        compile_error!("TODO")
    };
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use [`command!`](crate::event::command) to declare.
pub trait Command: Event {
    fn meta();

    fn label() -> RcVar<Text>;

    fn enabled() -> ReadOnlyVar<bool, RcVar<bool>>;

    fn has_handlers() -> ReadOnlyVar<bool, RcVar<bool>>;

    fn new_handle(events: &mut Events) -> CommandHandle;
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

pub struct CommandLocalKey<C: Command> {
    key: &'static LocalKey<CommandValue<C>>,
}

pub struct CommandValue<C: Command> {
    _c: PhantomData<C>,
    handle: Rc<CommandHandleData>,
    label: RcVar<Text>,
    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: OwnedStateMap,
}

impl<C: Command> CommandValue<C> {
    fn new() -> Self {
        CommandValue {
            _c: PhantomData,
            handle: Rc::new(CommandHandleData { enabled: Cell::new(0) }),
            label: var_from(type_name::<C>()),
            enabled: var(false),
            has_handlers: var(false),
            meta: OwnedStateMap::default(),
        }
    }

    fn update_state(&self, vars: &Vars) {
        let has_handlers = Rc::strong_count(&self.handle) > 1;
        let enabled = self.handle.enabled.get() > 0;

        self.has_handlers.set_ne(vars, has_handlers);
        self.enabled.set_ne(vars, enabled);
    }

    fn new_handle(&self) -> CommandHandle {
        CommandHandle {
            handle: Rc::clone(&self.handle),
            local_enabled: Cell::new(false),
        }
    }

    fn enabled(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.enabled.clone())
    }

    fn has_handlers(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.has_handlers.clone())
    }

    fn label(&self) -> RcVar<Text> {
        self.label.clone()
    }

    fn meta(&self) -> ! {
        todo!()
    }
}
