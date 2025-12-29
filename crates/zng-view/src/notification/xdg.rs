use zng_view_api::dialog::{DialogCapability, DialogId, Notification, NotificationResponse};

use crate::AppEventSender;

#[derive(Default)]
pub struct NotificationService {
    handles: Vec<(DialogId, notify_rust::NotificationHandle)>,
}

impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        DialogCapability::NOTIFICATION
            | DialogCapability::NOTIFICATION_ACTIONS
            | DialogCapability::CLOSE_NOTIFICATION
            | DialogCapability::UPDATE_NOTIFICATION
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let mut n = notify_rust::Notification::new();
        n.summary(&dialog.summary).body(&dialog.body);
        if let Some(t) = dialog.timeout {
            n.timeout(t);
        }

        for a in &dialog.actions {
            n.action(&a.id, &a.label);
        }

        match n.show() {
            Ok(_handle) => {
                // !!: TODO hook, on_close and on_action takes the handle, so we can't hook two and can't update after either?
            }
            Err(e) => {
                use zng_txt::ToTxt as _;
                use zng_view_api::Event;

                use crate::AppEvent;

                tracing::error!("failed to show notification, {e}");
                let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(
                    id,
                    NotificationResponse::Error(e.to_txt()),
                )));
            }
        }
    }

    pub fn update_notification(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        if let Some(i) = self.handles.iter().position(|(i, _)| *i == id) {
            if let Some(t) = dialog.timeout
                && t == std::time::Duration::ZERO
            {
                let (id, handle) = self.handles.swap_remove(i);
                handle.close();
                app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, NotificationResponse::Removed)));
                return;
            }
            let mut n = &mut self.handles[i].1;
            n.summary(&dialog.summary).body(&dialog.body);
            if let Some(t) = dialog.timeout {
                n.timeout(t);
            }
            n.actions.clear();
            for a in &dialog.actions {
                n.action(&a.id, &a.label);
            }
        }
    }
}
