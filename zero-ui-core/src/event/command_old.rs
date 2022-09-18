
/// A command that is a command type in a scope.
///
/// Normal commands apply globally, if there is a handler enabled in any context the status
/// variables indicate its availability. You can use [`Command::scoped`] to change this by
/// creating a new *command* that represents a command type in a *scope* only. The scope can
/// be any of the [`CommandScope`] values.
///
/// # Examples
///
/// Get the a command type scoped to a window:
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # struct FooNode { cmd: ScopedCommand<FooCommand> }
/// # impl FooNode {
/// fn init(&mut self, ctx: &mut WindowContext) {
///     self.cmd = FooCommand.scoped(*ctx.window_id);
/// }
/// # }
/// ```
///
/// # Enabled & Has Handlers
///
/// The [`enabled`] and [`has_handlers`] variables are only `true` when there is
/// a handler created using the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, handler::*, var::*, units::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|mut ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = cmd.scoped(ctx.window_id());
///
///     let enabled = cmd.enabled();
///     let enabled_scoped = cmd_scoped.enabled();
///
///     let handle = cmd_scoped.new_handle(&mut ctx, true);
///     ctx.update().await;
///
///     assert!(!enabled.copy(&ctx));
///     assert!(enabled_scoped.copy(&ctx));
/// })
/// # );
/// ```
///
/// In the example above, only the `enabled_scoped` is `true` after only the `cmd_scoped` is enabled.
///
/// # Metadata
///
/// Metadata is *inherited* from the [not scoped] command type but can be overwritten for the scoped command
/// only, so you can rename or give a different shortcut for the command only in the scope.
///
/// ```
/// # use zero_ui_core::{var::*, command::*, handler::*, context::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = FooCommand.scoped(ctx.window_id());
///
///     // same initial value:
///     assert_eq!(cmd.name().get_clone(&ctx), cmd_scoped.name().get_clone(&ctx));
///     
///     // set a name for all commands, including scoped not overridden:
///     cmd.name().set(&ctx, "Foo!");
///     ctx.update().await;
///     assert_eq!("Foo!", cmd_scoped.name().get_clone(&ctx));
///
///     // name is overridden in the scoped command only:
///     cmd_scoped.name().set(&ctx, "Scoped Only!");
///     ctx.update().await;
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
///     assert_eq!("Foo!", cmd.name().get_clone(&ctx));
///
///     // scoped command no-longer affected:
///     cmd.name().set(&ctx, "F");
///     ctx.update().await;
///     assert_eq!("F", cmd.name().get_clone(&ctx));
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
/// })
/// # );
/// ```
///
/// See [`CommandMetaVar<T>`] for details of how this is implemented.
///
/// # Notify
///
/// Calling [`notify`] from a scoped command **notifies the base type** but sets the [`CommandArgs::scope`]
/// the event will be handled by handlers for the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # fn init(ctx: &mut WindowContext) {
/// let notified = FooCommand.scoped(*ctx.window_id).notify(ctx, None);
/// # }  
/// ```
///
/// In the example above `notified` is `true` only if there are any handlers for the same scope.
///
/// # Update
///
/// Calling [`update`] from a command detects updates for the same command type if the [`CommandArgs::scope`]
/// is equal to the command scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, event::*};
/// # command! { pub FooCommand; }
/// # struct FooNode;
/// # impl FooNode {
/// fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
///     if let Some(args) = FooCommand.scoped(ctx.path.window_id()).update(args) {
///         println!("{:?}", args.scope);
///     }
/// }
/// # }
/// ```
///
/// The example will print only for commands on the scope of [`CommandScope::Window`] with the same id.
///
/// # App Scope
///
/// It is possible to create a scoped command using the [`App`] scope. In this
/// case the scoped command behaves exactly like a default command type.
///
/// [`enabled`]: ScopedCommand::enabled
/// [`notify`]: ScopedCommand::notify
/// [`update`]: ScopedCommand::update
/// [`has_handlers`]: ScopedCommand::has_handlers
/// [`App`]: CommandScope::App
/// [`name`]: CommandNameExt::name
#[derive(Debug, Clone, Copy)]
pub struct ScopedCommand<C: Command> {
    /// Base command type.
    pub command: C,

    /// Command scope.
    pub scope: CommandScope,
}

#[doc(hidden)]
pub struct CommandValue {
    command_type_id: TypeId,
    command_type_name: &'static str,

    scopes: RefCell<HashMap<CommandScope, ScopedValue>>,
    slot: EventSlot,

    handle: HandleOwner<CommandHandleData>,

    enabled: RcVar<bool>,

    has_handlers: RcVar<bool>,

    meta: RefCell<OwnedStateMap<CommandMetaState>>,

    meta_init: Box<dyn Fn()>,
    pending_init: Cell<bool>,
    registered: Cell<bool>,

    notify: Box<dyn Fn(&mut Events, CommandArgs)>,
}
#[allow(missing_docs)] // this is all hidden
impl CommandValue {
    pub fn init<C: Command, I: Fn() + 'static>(command: C, meta_init: I) -> Self {
        CommandValue {
            command_type_id: TypeId::of::<C>(),
            command_type_name: type_name::<C>(),
            scopes: RefCell::default(),
            handle: HandleOwner::dropped(CommandHandleData::default()),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
            meta_init: Box::new(meta_init),
            pending_init: Cell::new(true),
            registered: Cell::new(false),
            slot: EventSlot::next(),
            notify: Box::new(move |events, args| events.notify(command, args)),
        }
    }

    fn update_state(&self, vars: &Vars, scope: CommandScope) {
        if let CommandScope::App = scope {
            self.has_handlers.set_ne(vars, self.has_handlers_value(scope));
            self.enabled.set_ne(vars, self.enabled_value(scope).unwrap_or(false));
        } else {
            let mut has_handlers = false;
            let mut enabled = false;
            if let Some(data) = self.scopes.borrow().get(&scope) {
                has_handlers = !data.handle.is_dropped();
                enabled = data.handle.data().enabled_count.load(Ordering::Relaxed) > 0;
            }

            let scopes = self.scopes.borrow_mut();
            let scope = scopes.get(&scope).unwrap();
            scope.has_handlers.set_ne(vars, has_handlers);
            scope.enabled.set_ne(vars, enabled);
        }
    }

    pub fn on_exit(&self) {
        self.registered.set(false);
        self.scopes.borrow_mut().clear();
        self.meta.borrow_mut().clear();
        self.pending_init.set(true);
    }

    pub fn new_handle<Evs: WithEvents>(
        &self,
        events: &mut Evs,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        events.with_events(|ev| self.new_handle_impl(ev, key, scope, enabled))
    }
    fn new_handle_impl(
        &self,
        events: &mut Events,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        if let CommandScope::App = scope {
            if !self.registered.get() {
                self.registered.set(true);
                events.register_command(AnyCommand(key, CommandScope::App));
            }
            let r = CommandHandle {
                handle: self.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let value = scopes.entry(scope).or_insert_with(|| {
                // register scope first time and can create variables with the updated values already.
                events.register_command(AnyCommand(key, scope));
                ScopedValue {
                    enabled: var(enabled),
                    has_handlers: var(true),
                    handle: HandleOwner::dropped(CommandHandleData::default()),
                    meta: OwnedStateMap::new(),
                    registered: true,
                }
            });
            if !value.registered {
                // register scope first time.
                events.register_command(AnyCommand(key, scope));
                value.registered = true;
            }
            let r = CommandHandle {
                handle: value.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        }
    }

    pub fn slot(&self) -> EventSlot {
        self.slot
    }

    pub fn enabled(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.enabled.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().enabled.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn enabled_value(&self, scope: CommandScope) -> Option<bool> {
        if let CommandScope::App = scope {
            if self.handle.is_dropped() {
                None
            } else {
                Some(self.handle.data().enabled_count.load(Ordering::Relaxed) > 0)
            }
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            if value.handle.is_dropped() {
                None
            } else {
                Some(value.handle.data().enabled_count.load(Ordering::Relaxed) > 0)
            }
        } else {
            None
        }
    }

    pub fn has_handlers(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.has_handlers.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().has_handlers.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn has_handlers_value(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            !self.handle.is_dropped()
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            !value.handle.is_dropped()
        } else {
            false
        }
    }

    pub fn with_meta<F, R>(&self, f: F, scope: CommandScope) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        if self.pending_init.take() {
            (self.meta_init)()
        }

        if let CommandScope::App = scope {
            f(&mut CommandMeta {
                meta: self.meta.borrow_mut().borrow_mut(),
                scope: None,
            })
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let scope = scopes.entry(scope).or_default();
            f(&mut CommandMeta {
                meta: self.meta.borrow_mut().borrow_mut(),
                scope: Some(scope.meta.borrow_mut()),
            })
        }
    }
}



