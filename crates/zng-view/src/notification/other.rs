#![allow(unused)]

use zng_view_api::{
    Event,
    dialog::{DialogCapability, DialogId, Notification, NotificationResponse},
};

use crate::{AppEvent, AppEventSender};

#[derive(Default)]
pub struct NotificationService {}
impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        DialogCapability::empty()
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let _ = dialog;
        let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(
            id,
            NotificationResponse::Error("notification_dialog not implemented for Android".into()),
        )));
        tracing::error!("notification_dialog not implemented for {}", std::env::consts::OS);
    }

    pub fn update_notification(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let _ = (app_sender, id, dialog);
        tracing::error!("update_notification not implemented for {}", std::env::consts::OS);
    }
}
