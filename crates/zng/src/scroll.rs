#![cfg(feature = "scroll")]

//! Scroll widgets, commands and properties.
//!
//! The [`Scroll!`](struct@Scroll) widget accepts a single child of any size, overflow is clipped and can be brought
//! into view by scrolling, the widget also supports content zooming and panning. The
//! [`mode`](struct@Scroll#method.mode) property can be used to dynamically change the [`ScrollMode`].
//!
//! ```
//! # fn main() { }
//! use zng::prelude::*;
//!
//! # fn demo() { let _ =
//! Scroll! {
//!     // ZOOM includes PAN that includes VERTICAL and HORIZONTAL
//!     mode = zng::scroll::ScrollMode::ZOOM;
//!     // mouse press and drag scrolls
//!     mouse_pan = true;
//!
//!     child = Image! {
//!         // center_viewport uses the SCROLL service
//!         img_loading_fn = wgt_fn!(|_| center_viewport(Text!("loading..")));
//!
//!         // content is a large image
//!         source = "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg";
//!         img_limits = zng::image::ImageLimits::none();
//!         img_downscale = zng::image::ImageDownscale::from(layout::Px(8000));
//!     };
//! }
//! # ; }
//!
//! fn center_viewport(msg: impl IntoUiNode) -> UiNode {
//!     use zng::scroll::SCROLL;
//!     Container! {
//!         // center the message on the scroll viewport:
//!         //
//!         // the large images can take a moment to decode in debug builds, but the size
//!         // is already known after read, so the "loading.." message ends-up off-screen
//!         // because it is centered on the image.
//!         layout::x = merge_var!(SCROLL.horizontal_offset(), SCROLL.zoom_scale(), |&h, &s| h.0.fct_l()
//!             - 1.vw() / s * h);
//!         layout::y = merge_var!(SCROLL.vertical_offset(), SCROLL.zoom_scale(), |&v, &s| v.0.fct_l() - 1.vh() / s * v);
//!         layout::scale = SCROLL.zoom_scale().map(|&fct| 1.fct() / fct);
//!         layout::transform_origin = 0;
//!         widget::auto_hide = false;
//!         layout::max_size = (1.vw(), 1.vh());
//!
//!         child_align = Align::CENTER;
//!         child = msg;
//!     }
//! }
//! ```
//!
//! The example above declares a scroll with zoom and mouse pan features enabled, is also makes use of the [`SCROLL`] service
//! to implement the `center_viewport` widget that is place in the content, but transforms to always be in the viewport.
//!
//! The `SCROLL` service can be used to interact with the parent `Scroll!`, you can also use commands in [`cmd`] to
//! control any `Scroll!` widget.
//!
//! # Full API
//!
//! See [`zng_wgt_scroll`] for the full widget API.

pub use zng_wgt_scroll::{
    LazyMode, SCROLL, Scroll, ScrollBarArgs, ScrollFrom, ScrollInfo, ScrollMode, ScrollUnitsMix, Scrollbar, ScrollbarFnMix,
    SmoothScrolling, Thumb, WidgetInfoExt, alt_factor, auto_hide_extra, clip_to_viewport, define_viewport_unit, h_line_unit, h_page_unit,
    h_scrollbar_fn, h_wheel_unit, lazy, line_units, max_zoom, min_zoom, mode, mouse_pan, overscroll_color, page_units,
    scroll_to_focused_mode, scrollbar_fn, scrollbar_joiner_fn, smooth_scrolling, v_line_unit, v_page_unit, v_scrollbar_fn, v_wheel_unit,
    wheel_units, zoom_origin, zoom_size_only, zoom_touch_origin, zoom_wheel_origin, zoom_wheel_unit,
};

/// Scrollbar thumb widget.
pub mod thumb {
    pub use zng_wgt_scroll::thumb::{Thumb, cross_length, offset, viewport_ratio};
}

/// Scroll widget.
pub mod scrollbar {
    pub use zng_wgt_scroll::scrollbar::{Orientation, SCROLLBAR, Scrollbar, orientation};
}

/// Scroll commands.
pub mod cmd {
    pub use zng_wgt_scroll::cmd::{
        PAGE_DOWN_CMD, PAGE_LEFT_CMD, PAGE_RIGHT_CMD, PAGE_UP_CMD, SCROLL_DOWN_CMD, SCROLL_LEFT_CMD, SCROLL_RIGHT_CMD,
        SCROLL_TO_BOTTOM_CMD, SCROLL_TO_CMD, SCROLL_TO_LEFTMOST_CMD, SCROLL_TO_RIGHTMOST_CMD, SCROLL_TO_TOP_CMD, SCROLL_UP_CMD,
        ScrollRequest, ScrollToMode, ScrollToRequest, ScrollToTarget, ZOOM_IN_CMD, ZOOM_OUT_CMD, ZOOM_RESET_CMD, ZOOM_TO_FIT_CMD,
        scroll_to, scroll_to_zoom, ZoomToFitRequest,
    };
}
