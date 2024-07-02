use zng_app_context::app_local;
use zng_time::INSTANT_APP;

use crate::update::{UpdatesTrace, UPDATES};

use super::*;

app_local! {
    pub(crate) static EVENTS_SV: EventsService = const { EventsService::new() };
}

pub(crate) struct EventsService {
    updates: Mutex<Vec<EventUpdate>>, // not locked, used to make service Sync.
    commands: Vec<Command>,
    register_commands: Vec<Command>,
}

impl EventsService {
    const fn new() -> Self {
        Self {
            updates: Mutex::new(vec![]),
            commands: vec![],
            register_commands: vec![],
        }
    }

    pub(super) fn register_command(&mut self, command: Command) {
        if self.register_commands.is_empty() {
            UPDATES.update(None);
        }
        self.register_commands.push(command);
    }

    pub(super) fn sender<A>(&mut self, event: Event<A>) -> EventSender<A>
    where
        A: EventArgs + Send,
    {
        EventSender {
            sender: UPDATES.sender(),
            event,
        }
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.get_mut().is_empty()
    }
}

/// App events and commands service.
pub struct EVENTS;
impl EVENTS {
    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::subscribe`] is called for the first time in an app, the command gets added
    /// to this list after the current update, if the command is app scoped it remains on the list for
    /// the lifetime of the app, if it is window or widget scoped it only remains while there are handles.
    ///
    /// [`Command::subscribe`]: crate::event::Command::subscribe
    pub fn commands(&self) -> Vec<Command> {
        EVENTS_SV.read().commands.clone()
    }

    /// Schedules the raw event update.
    pub fn notify(&self, update: EventUpdate) {
        UpdatesTrace::log_event(update.event);
        EVENTS_SV.write().updates.get_mut().push(update);
        UPDATES.send_awake();
    }

    #[must_use]
    pub(crate) fn apply_updates(&self) -> Vec<EventUpdate> {
        let _s = tracing::trace_span!("EVENTS").entered();

        let mut ev = EVENTS_SV.write();
        ev.commands.retain(|c| c.update_state());

        {
            let ev = &mut *ev;
            for cmd in ev.register_commands.drain(..) {
                if cmd.update_state() {
                    if ev.commands.iter().any(|c| c == &cmd) {
                        tracing::error!("command `{cmd:?}` is already registered")
                    } else {
                        ev.commands.push(cmd);
                    }
                }
            }
        }

        let mut updates: Vec<_> = ev.updates.get_mut().drain(..).collect();
        drop(ev);

        if !updates.is_empty() {
            let _t = INSTANT_APP.pause_for_update();

            for u in &mut updates {
                let ev = u.event;
                ev.on_update(u);
            }
        }
        updates
    }
}

event_args! {
    /// Arguments for [`COMMANDS_CHANGED_EVENT`].
    pub struct CommandsChangedArgs {
        /// Scoped commands that lost all handlers.
        pub removed: Vec<Command>,

        /// New commands.
        pub added: Vec<Command>,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }
}

event! {
    /// Event when [`EVENTS.commands`] list changes.
    ///
    /// [`EVENTS.commands`]: EVENTS::commands
    pub static COMMANDS_CHANGED_EVENT: CommandsChangedArgs;
}
