use std::{pin::Pin, time::Duration};

use win32_notif::{
    ManageNotification, NotifError, NotificationActivatedEventHandler, NotificationDataSet, NotificationDismissedEventHandler,
    NotificationFailedEventHandler,
    handler::ToastDismissedReason,
    notification::{
        actions::{ActionButton, action::ActivationType},
        visual::{Text, text::HintStyle},
    },
};
use zng_txt::{ToTxt, Txt, formatx};
use zng_view_api::{
    Event,
    dialog::{DialogCapability, DialogId, Notification, NotificationResponse},
};

use crate::{AppEvent, AppEventSender};

#[derive(Default)]
pub struct NotificationService {
    // SAFETY: not actually 'static, depends on `notifier` lifetime.
    notifications: Vec<(DialogId, Txt, win32_notif::Notification<'static>)>,

    notifier: Option<Pin<Box<win32_notif::ToastsNotifier>>>,
    inited: bool,
    tag_gen: usize,
}
fn is_registered_app_id(app_id: &str) -> bool {
    let h_id = windows::core::HSTRING::from(app_id);
    windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&h_id).is_ok()
}

impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        if self.notifier.is_none() {
            return DialogCapability::empty();
        }
        DialogCapability::NOTIFICATION
            | DialogCapability::NOTIFICATION_ACTIONS
            | DialogCapability::CLOSE_NOTIFICATION
            | DialogCapability::CLOSE_NOTIFICATION
    }

    fn init(&mut self) {
        if self.inited {
            return;
        }
        self.inited = true;

        let about = zng_env::about();
        let id = about.app_id.as_str();

        // The windows ToastNotificationManager does not return an error for invalid ID, so we validate it here.
        //
        // Have observed 0xc0000005 (Access Violation) crashes some times, others a COM error,
        // no idea why its inconsistent, in isolated test without an app running it never returns an error
        // but also does not show the notifications.
        let notifier = if is_registered_app_id(id) {
            win32_notif::ToastsNotifier::new(id)
        } else {
            const FALLBACK_ID: &str = "Microsoft.Windows.Explorer";
            tracing::warn!("{id:?} is not a registered AppUserModelID, will use {FALLBACK_ID:?}");
            win32_notif::ToastsNotifier::new(FALLBACK_ID)
        };
        self.notifier = match notifier {
            Ok(n) => Some(Box::pin(n)),
            Err(e) => {
                tracing::error!("cannot init notifier service, {e}");
                None
            }
        };
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        self.init();

        if let Some(notifier) = &self.notifier {
            let mut note = win32_notif::NotificationBuilder::new()
                .with_use_button_style(true)
                .visual(Text::create_binded(1, "title").with_style(HintStyle::Title))
                .visual(Text::create_binded(2, "message").with_style(HintStyle::Body))
                .value("title", dialog.title.as_str())
                .value("message", dialog.message.as_str());
            if let Some(t) = dialog.timeout {
                note = note.with_expiry(t);
            }
            for a in &dialog.actions {
                note = note.action(
                    ActionButton::create(a.label.as_str())
                        .with_input_id(a.id.as_str())
                        .with_activation_type(ActivationType::Foreground),
                )
            }

            let sender = app_sender.clone();
            note = note.on_activated(NotificationActivatedEventHandler::new(move |_, action| {
                let action = action.and_then(|a| a.button_id).unwrap_or_default();
                let _ = sender.send(AppEvent::Notify(Event::NotificationResponse(
                    id,
                    NotificationResponse::Action(action.into()),
                )));
                Ok(())
            }));
            let sender = app_sender.clone();
            note = note.on_dismissed(NotificationDismissedEventHandler::new(move |_, reason| {
                let r = match reason {
                    Some(ToastDismissedReason::UserCanceled) => NotificationResponse::Dismissed,
                    _ => NotificationResponse::Removed,
                };
                let _ = sender.send(AppEvent::Notify(Event::NotificationResponse(id, r)));
                Ok(())
            }));
            let sender = app_sender.clone();
            note = note.on_failed(NotificationFailedEventHandler::new(move |_, error| {
                let error = error.and_then(|a| a.error).unwrap_or_default();
                let e = formatx!("notification failed, {error}");
                tracing::error!("{e}");
                let _ = sender.send(AppEvent::Notify(Event::NotificationResponse(id, NotificationResponse::Error(e))));
                Ok(())
            }));

            let tag = self.tag_gen.to_txt();
            self.tag_gen += 1;

            let r = (|| -> Result<win32_notif::Notification, NotifError> {
                let note = note.build(1, notifier, &tag, "zng-view")?;
                note.show()?;
                Ok(note)
            })();
            match r {
                Ok(n) => {
                    // SAFETY: we are only casting for storage
                    let sin = unsafe { std::mem::transmute::<win32_notif::Notification<'_>, win32_notif::Notification<'static>>(n) };
                    self.notifications.push((id, tag, sin));
                }
                Err(e) => {
                    let e = formatx!("cannot build notification, {e}");
                    tracing::error!("{e}");
                    let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, NotificationResponse::Error(e))));
                }
            }
        }
    }

    pub fn update_notification<'a>(&'a mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        if self.notifier.is_none() {
            return;
        }

        if let Some(i) = self.notifications.iter().position(|(i, _, _)| *i == id) {
            let notifier = self.notifier.as_ref().unwrap();

            let (id, tag, note) = self.notifications.swap_remove(i);
            // SAFETY: 'a is the real lifetime, it was only casted to 'static for storage
            let note_ref = unsafe { std::mem::transmute::<&win32_notif::Notification<'static>, &win32_notif::Notification<'a>>(&note) };

            if let Some(t) = dialog.timeout
                && t == Duration::ZERO
            {
                // remove
                let r = (|| -> Result<(), NotifError> {
                    if let Some(t) = note_ref.activated_event_handler_token {
                        note_ref.remove_activated_handler(t)?;
                    }
                    if let Some(t) = note_ref.dismissed_event_handler_token {
                        note_ref.remove_dismissed_handler(t)?;
                    }
                    if let Some(t) = note_ref.failed_event_handler_token {
                        note_ref.remove_failed_handler(t)?;
                    }
                    notifier.manager()?.remove_notification_with_tag(&tag)
                })();
                let r = match r {
                    Ok(()) => NotificationResponse::Removed,
                    Err(NotifError::WindowsCore(e)) if e.code().0 == 0x80070490_u32 as i32 => NotificationResponse::Removed,
                    Err(e) => NotificationResponse::Error(formatx!("cannot remove notification, {e}")),
                };
                let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, r)));
            } else {
                // update

                let r = (|| -> Result<(), NotifError> {
                    let update = NotificationDataSet::new()?;
                    update.insert("title", &dialog.title)?;
                    update.insert("message", &dialog.message)?;
                    notifier.update(&update, "zng-view", &tag)?;
                    if let Some(t) = dialog.timeout {
                        note.set_expiration(t)?;
                    }
                    Ok(())
                })();
                if let Err(e) = r {
                    let e = NotificationResponse::Error(formatx!("cannot update notification, {e}"));
                    let _ = app_sender.send(AppEvent::Notify(Event::NotificationResponse(id, e)));
                }

                self.notifications.push((id, tag, note));
            }
        }
    }
}
