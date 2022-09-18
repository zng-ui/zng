#[macro_export]
macro_rules! command {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident $(= { $init:expr })? ;
    )+) => {
        $(
            paste::paste! {
                #[doc(hidden)]
                std::thread_local! {
                    static [<$COMMAND _LOCAL>] = $crate::event::EventData::new(std::stringifly!($EVENT));
                    static [<$COMMAND _DATA>] = $crate::event::CommandData::new();
                }

                $(#[$attr])*
                $vis static $COMMAND: $crate::event::Command = $crate::event::Command::new([<$COMMAND _LOCAL>], [<$COMMAND _DATA>]);
            }
        )+
    }
}

#[doc(hidden)]
pub struct CommandData {
    meta_init: Option<Box<dyn Fn(StateMapMut<EventMeta>)>>,
    meta_inited: Cell<bool>,
    meta: RefCell<OwnedStateMap<EventMeta>>,
    scoped_meta: RefCell<FxHashMap<CommandScope, OwnedStateMap<EventMeta>>>,
}
impl CommandData {}

/// Presents a command event.
#[derive(Clone, Copy)]
pub struct Command {
    event: Event<CommandArgs>,
    local: &'static LocalKey<CommandData>,
    scope: CommandScope,
}
impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Command")
                .field("event", &self.event)
                .field("scope", &self.scope)
                .finish_non_exhaustive()
        } else {
            write!(f, "{}", self.event.name())?;
            match self.scope {
                CommandScope::App => Ok(()),
                CommandScope::Window(id) => write!(f, "({id})"),
                CommandScope::Widget(id) => write!(f, "({id})"),
            }
        }
    }
}
impl Command {
    #[doc(hidden)]
    pub fn new(event_local: &'static LocalKey<EventData>, command_local: &'static LocalKey<CommandData>) -> Self {
        Command {
            event: Event::new(event_local),
            local: command_local,
            scope: CommandScope::App,
        }
    }

    /// Raw command event.
    pub fn event(&self) -> Event<CommandArgs> {
        self.event
    }

    /// Command operating scope.
    pub fn scope(&self) -> CommandScope {
        self.scope
    }

    /// Gets the command in a new `scope`.
    pub fn scoped(mut self, scope: CommandScope) -> Command {
        self.scope = scope;
        self
    }

    /// Visit the command custom metadata of the current scope.
    pub fn with_meta<R>(&self, visit: impl FnOnce(&mut CommandMeta) -> R) -> R {
        todo!()
    }

    /// Returns `true` if the update is for this command and scope.
    pub fn has(&self, update: &EventUpdate) -> bool {
        self.on(update).is_some()
    }

    /// Get the command update args if the update is for this command and scope.
    pub fn on<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event.on(update).filter(|a| a.scope == self.scope)
    }

    /// Get the event update args if the update is for this event and propagation is not stopped.
    pub fn on_unhandled<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event
            .on(update)
            .filter(|a| a.scope == self.scope && a.propagation().is_stopped())
    }

    /// Calls `handler` if the update is for this event and propagation is not stopped, after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&CommandArgs) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }

    /// Gets a variable that tracks if this command has any live handlers.
    pub fn has_handlers(&self) -> ReadOnlyRcVar<bool> {
        todo!()
    }

    /// Gets a variable that tracks if this command has any enabled live handlers.
    pub fn is_enabled(&self) -> ReadOnlyRcVar<bool> {
        todo!()
    }

    fn is_enabled_value(&self) -> bool {
        todo!()
    }

    /// Schedule a command update without param.
    pub fn notify<Ev: WithEvents>(&self, events: &mut Ev) {
        self.event.notify(events, CommandArgs::now(None, self.scope, self.is_enabled_value()))
    }

    /// Schedule a command update with custom `param`.
    pub fn notify_param<Ev: WithEvents>(&self, events: &mut Ev, param: impl Any) {
        self.event
            .notify(events, CommandArgs::now(CommandParam::new(param), self.scope, self.is_enabled_value()));
    }
}
impl Deref for Command {
    type Target = Event<CommandArgs>;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event && self.scope == other.scope
    }
}
impl Eq for Command {}
