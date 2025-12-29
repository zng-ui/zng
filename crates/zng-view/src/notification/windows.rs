use zng_view_api::dialog::{DialogCapability, DialogId, Notification, NotificationResponse};

use crate::AppEventSender;

#[derive(Default)]
pub struct NotificationService {}

impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        DialogCapability::NOTIFICATION
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let mut n = notify_rust::Notification::new();
        n.summary(&dialog.summary).body(&dialog.body);
        if let Some(t) = dialog.timeout {
            n.timeout(t);
        }

        // notify_rust does not implement this for Windows yet
        // for a in &dialog.actions {
        //     n.action(&a.id, &a.label);
        // }
        if !dialog.actions.is_empty() {
            tracing::warn!("notification actions not implemented for {}", std::env::consts::OS);
        }

        match n.show() {
            Ok(()) => {
                tracing::warn!("notification action/dismiss event not implement for {}", std::env::consts::OS);
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
        super::other::NotificationService::default().update_notification(app_sender, id, dialog);
    }
}
