use super::{
    AppExtension, AppRegister, EventNotifier, EventUpdate, Service, UpdateNotice, UpdateNotifier, WindowEvent, WindowId,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// New window event.
pub struct NewWindow;

/// [NewWindow] event args.
#[derive(Debug, Clone)]
pub struct NewWindowArgs {
    pub when: Instant,
    pub window_id: WindowId,
}

impl EventNotifier for NewWindow {
    type Args = NewWindowArgs;
}

pub(crate) struct WindowsExt {
    service: Windows,
    new_window: UpdateNotifier<NewWindowArgs>,
}

impl Default for WindowsExt {
    fn default() -> Self {
        WindowsExt {
            service: Windows::default(),
            new_window: UpdateNotifier::new(false),
        }
    }
}

impl AppExtension for WindowsExt {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_service::<Windows>(self.service.clone());
        r.register_event::<NewWindow>(self.new_window.listener());
    }
}

#[derive(Default)]
struct WindowsRequests {}

/// Windows service.
#[derive(Clone, Default)]
pub struct Windows {
    requests: Rc<RefCell<WindowsRequests>>,
}

impl Service for Windows {}

impl Windows {
    /// Requests a new window. Returns a notice that gets updated once
    /// when the window is launched.
    pub fn new_window(&self) -> UpdateNotice<NewWindowArgs> {
        todo!()
    }
}
