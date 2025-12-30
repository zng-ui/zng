use zng_view_api::dialog::{DialogCapability, DialogId, Notification};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
use android as platform;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod xdg;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use xdg as platform;

mod other;
#[cfg(not(any(
    windows,
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "android",
)))]
use other as platform;

use crate::AppEventSender;

#[derive(Default)]
pub struct NotificationService {
    service: platform::NotificationService,
}
impl NotificationService {
    pub fn capabilities(&self) -> DialogCapability {
        self.service.capabilities()
    }

    pub fn notification_dialog(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        self.service.notification_dialog(app_sender, id, dialog);
    }

    pub fn update_notification(&mut self, app_sender: &AppEventSender, id: DialogId, dialog: Notification) {
        self.service.update_notification(app_sender, id, dialog);
    }
}
