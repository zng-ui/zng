//! Event handler properties, [`on_click`](fn@gesture::on_click), [`on_key_down`](fn@keyboard::on_key_down),
//! [`on_focus`](fn@focus::on_focus) and more.

use crate::core::event::*;

pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod widget;
