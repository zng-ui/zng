//! Scroll widgets, commands and properties.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_scroll`] for the full widget API.

pub use zero_ui_wgt_scroll::{
    alt_factor, auto_hide_extra, clip_to_viewport, define_viewport_unit, h_line_unit, h_page_unit, h_scrollbar_fn, h_wheel_unit, lazy,
    line_units, max_zoom, min_zoom, mode, mouse_pan, overscroll_color, page_units, scroll_to_focused_mode, scrollbar_fn,
    scrollbar_joiner_fn, smooth_scrolling, v_line_unit, v_page_unit, v_scrollbar_fn, v_wheel_unit, wheel_units, zoom_origin,
    zoom_touch_origin, zoom_wheel_origin, zoom_wheel_unit, LazyMode, Scroll, ScrollBarArgs, ScrollFrom, ScrollInfo, ScrollMode,
    ScrollUnitsMix, Scrollbar, ScrollbarFnMix, SmoothScrolling, Thumb, WidgetInfoExt, SCROLL,
};

/// Scrollbar thumb widget.
pub mod thumb {
    pub use zero_ui_wgt_scroll::thumb::{cross_length, offset, viewport_ratio, Thumb};
}

/// Scroll widget.
pub mod scrollbar {
    pub use zero_ui_wgt_scroll::scrollbar::{orientation, Orientation, Scrollbar, SCROLLBAR};
}

/// Scroll commands.
pub mod cmd {
    pub use zero_ui_wgt_scroll::cmd::{
        scroll_to, scroll_to_zoom, ScrollRequest, ScrollToMode, ScrollToRequest, ScrollToTarget, PAGE_DOWN_CMD, PAGE_LEFT_CMD,
        PAGE_RIGHT_CMD, PAGE_UP_CMD, SCROLL_DOWN_CMD, SCROLL_LEFT_CMD, SCROLL_RIGHT_CMD, SCROLL_TO_BOTTOM_CMD, SCROLL_TO_CMD,
        SCROLL_TO_LEFTMOST_CMD, SCROLL_TO_RIGHTMOST_CMD, SCROLL_TO_TOP_CMD, SCROLL_UP_CMD, ZOOM_IN_CMD, ZOOM_OUT_CMD, ZOOM_RESET_CMD,
    };
}
