//! Application system menu.

use bitflags::bitflags;
use zng_txt::Txt;

use crate::image::ImageId;

/// Represents a menu command or submenu header.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum MenuItem {
    /// Clickable action.
    #[non_exhaustive]
    Command {
        /// Unique ID for this command within all menu items in the app.
        ///
        /// If this id is empty the menu item is disabled.
        id: Txt,
        /// Display text.
        label: Txt,
    },
    /// Submenu.
    #[non_exhaustive]
    SubMenu {
        /// Display text.
        label: Txt,
        /// Children items.
        children: Vec<MenuItem>,
    },
    /// Separation line.
    Separator,
}
impl MenuItem {
    /// New command.
    pub fn command(id: impl Into<Txt>, label: impl Into<Txt>) -> Self {
        Self::Command {
            id: id.into(),
            label: label.into(),
        }
    }

    /// New submenu.
    pub fn sub_menu(label: impl Into<Txt>, children: Vec<MenuItem>) -> Self {
        Self::SubMenu {
            label: label.into(),
            children,
        }
    }
}

/// Represents a system application menu.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AppMenu {
    /// The menu items.
    ///
    /// If empty no application menu is set.
    pub children: Vec<MenuItem>,
}
impl AppMenu {
    /// New.
    pub fn new(children: Vec<MenuItem>) -> Self {
        Self { children }
    }

    /// Value that represents no app menu.
    pub fn none() -> Self {
        Self::new(vec![])
    }
}

/// Represents a *tray icon* status indicator.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct TrayIcon {
    /// Icon image.
    ///
    /// The tray icon will only be inserted when this image is valid and loaded.
    pub icon: ImageId,
    /// Optional context menu.
    ///
    /// If not empty a context menu shows on context clock.
    pub context_menu: Vec<MenuItem>,
    /// A command ID for a primary click on the icon.
    ///
    /// If set an [`Event::MenuCommand`] notifies on click, otherwise the context menu also opens on primary click.
    ///
    /// [`Event::MenuCommand`]: crate::types::Event::MenuCommand
    pub primary_command_id: Txt,
}
impl TrayIcon {
    /// New.
    pub fn new(icon: ImageId, context_menu: Vec<MenuItem>) -> Self {
        Self {
            icon,
            context_menu,
            primary_command_id: Txt::from_static(""),
        }
    }

    /// Value that indicates no tray icon.
    pub fn none() -> Self {
        Self::new(ImageId::INVALID, vec![])
    }
}

bitflags! {
    /// System menu capability.
    #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct MenuCapability: u32 {
        /// View-process can set application menu items.
        ///
        /// The application menu is shown outside the app windows, usually at the top of the main screen in macOS and Gnome desktops.
        const APP_MENU = 1;
        /// View-process can set tray icon with context menu.
        ///
        /// This is a small status indicator icon displayed near the notifications area.
        const TRAY_ICON = 1 << 1;
    }
}
