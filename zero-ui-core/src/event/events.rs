use crate::context::{app_local, UPDATES};

use super::*;

app_local! {
    pub(crate) static EVENTS_SV: EventsService = const { EventsService::new() };
}

pub(crate) struct EventsService {
    updates: Mutex<Vec<EventUpdate>>, // not locked, used to make service Sync.
    commands: Vec<Command>,
}

impl EventsService {
    const fn new() -> Self {
        Self {
            updates: Mutex::new(vec![]),
            commands: vec![],
        }
    }

    pub(super) fn register_command(&mut self, command: Command) {
        if self.commands.iter().any(|c| c == &command) {
            panic!("command `{command:?}` is already registered")
        }
        self.commands.push(command);
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
    /// Schedules the raw event update.
    pub fn notify(&self, update: EventUpdate) {
        EVENTS_SV.write().updates.get_mut().push(update);
        UPDATES.send_awake();
    }

    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::subscribe`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::subscribe`]: crate::event::Command::subscribe
    pub fn commands(&self) -> Vec<Command> {
        EVENTS_SV.read().commands.clone()
    }

    #[must_use]
    pub(crate) fn apply_updates(&self) -> Vec<EventUpdate> {
        let _s = tracing::trace_span!("EVENTS").entered();

        let mut ev = EVENTS_SV.write();
        for command in &ev.commands {
            command.update_state();
        }
        let mut updates: Vec<_> = ev.updates.get_mut().drain(..).collect();
        drop(ev);
        for u in &mut updates {
            let ev = u.event;
            ev.on_update(u);
        }
        updates
    }
}
