use std::{collections::HashSet, sync::Arc};

use zng_txt::Txt;
use zng_view_api::{
    Event,
    dialog::{DialogCapability, DialogId, Notification, NotificationResponse},
};

use crate::{AppEvent, AppEventSender};

/*
notify_rust API issues

We cannot clone the handle and the `wait_for_action` method takes the handle so we need to choose beteween supporting
response messages and notification updates (and closing). For now all update related code is commented until this is sorted.
*/

#[derive(Default)]
pub struct NotificationService {
    // handles: Vec<(DialogId, notify_rust::NotificationHandle)>,
    running_count: Arc<()>,
}

impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        DialogCapability::NOTIFICATION | DialogCapability::NOTIFICATION_ACTIONS
        // | DialogCapability::CLOSE_NOTIFICATION
        // | DialogCapability::UPDATE_NOTIFICATION
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let mut n = notify_rust::Notification::new();
        n.summary(&dialog.summary).body(&dialog.body);
        if let Some(t) = dialog.timeout {
            n.timeout(t);
        }
        n.appname(&zng_env::about().app);

        for a in &dialog.actions {
            n.action(&a.id, &a.label);
        }

        // we need to use a blocking thread for each notification
        if Arc::strong_count(&self.running_count) >= 16 {
            let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(
                id,
                NotificationResponse::Error("reached limit of active notifications".into()),
            )));
            return;
        }

        let app_sender = app_sender.clone();
        let actions: HashSet<_> = dialog.actions.into_iter().map(|a| a.id).collect();
        let count = self.running_count.clone();
        let _ = std::thread::Builder::new()
            .name("notify_rust".to_owned())
            .stack_size(256 * 1024)
            .spawn(move || {
                let _count = count;
                match n.show() {
                    Ok(handle) => {
                        let mut r = Txt::from_static("");
                        handle.wait_for_action(|action| {
                            r = Txt::from_str(action);
                        });
                        let r = if actions.contains(&r) {
                            NotificationResponse::Action(r)
                        } else {
                            NotificationResponse::Dismissed
                        };
                        let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, r)));
                        // self.handles.push((id, handle));
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
            });
    }

    pub fn update_notification(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        let _ = (app_sender, id, dialog);
        // if let Some(i) = self.handles.iter().position(|(i, _)| *i == id) {
        //     if let Some(t) = dialog.timeout
        //         && t == std::time::Duration::ZERO
        //     {
        //         let (id, handle) = self.handles.swap_remove(i);
        //         handle.close();
        //         let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, NotificationResponse::Removed)));
        //         return;
        //     }
        //     let n = &mut self.handles[i].1;
        //     n.summary(&dialog.summary).body(&dialog.body);
        //     if let Some(t) = dialog.timeout {
        //         n.timeout(t);
        //     }
        //     n.actions.clear();
        //     for a in &dialog.actions {
        //         n.action(&a.id, &a.label);
        //     }
        // }
    }
}
