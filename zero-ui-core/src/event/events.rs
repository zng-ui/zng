use crate::{app::AppEventSender, context::app_local, var::Vars};

use super::*;

thread_singleton!(SingletonEvents);

app_local! {
    pub(crate) static EVENTS_SV: EventsService = EventsService::new();
}

pub(crate) struct EventsService {
    app_event_sender: Option<AppEventSender>,
    updates: Mutex<Vec<EventUpdate>>, // not locked, used to make service Sync.
    commands: Vec<Command>,
}

impl EventsService {
    fn new() -> Self {
        Self {
            app_event_sender: None,
            updates: Mutex::new(vec![]),
            commands: vec![],
        }
    }

    pub(crate) fn init(&mut self, app_event_sender: AppEventSender) {
        self.app_event_sender = Some(app_event_sender);
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
            sender: self.app_event_sender.as_ref().unwrap().clone(),
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
    pub(crate) fn apply_updates(&self, vars: &Vars) -> Vec<EventUpdate> {
        let _s = tracing::trace_span!("Events").entered();

        let mut ev = EVENTS_SV.write();
        for command in &ev.commands {
            command.update_state(vars);
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
