//! Common widget properties.

pub mod access;

pub mod inspector;

mod layout;
pub use layout::*;

mod visual;
pub use visual::*;

mod border_;
pub use border_::{border, border_align, corner_radius, corner_radius_fit, CornerRadiusFit};

pub mod mask;
pub use mask::{mask_align, mask_fit, mask_image, mask_mode};

pub mod commands;

pub mod data_context;
pub use data_context::{
    data, data_error, data_info, data_warn, get_data_error, get_data_error_txt, get_data_info, get_data_info_txt, get_data_warn,
    get_data_warn_txt, has_data_error, has_data_info, has_data_warn, DATA,
};

pub mod events;
pub mod filters;
pub mod focus;
pub mod states;
pub mod transform;

mod undo;
pub use undo::*;

mod capture;
pub use capture::*;

mod mouse;
pub use mouse::*;

mod lazy_;
pub use lazy_::{lazy, LazyMode};

pub use crate::widgets::{
    menu::context::{context_menu, context_menu_anchor, context_menu_fn, disabled_context_menu, disabled_context_menu_fn},
    tip::{
        disabled_tooltip, disabled_tooltip_fn, tooltip, tooltip_anchor, tooltip_context_capture, tooltip_delay, tooltip_duration,
        tooltip_fn, tooltip_interval,
    },
};

pub use crate::core::widget_base::{enabled, hit_test_mode, interactive};
