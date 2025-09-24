#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Exact size constraints and exact positioning properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod offset_impl;
pub use offset_impl::*;
mod min;
pub use min::*;
mod max;
pub use max::*;
mod size_impl;
pub use size_impl::*;
mod force;
pub use force::*;
mod actual;
pub use actual::*;
mod sticky;
pub use sticky::*;

use zng_wgt::prelude::*;

/// Set or overwrite the baseline of the widget.
///
/// The `baseline` is a vertical offset from the bottom edge of the widget's inner bounds up, it defines the
/// line where the widget naturally *sits*, some widgets like [Text!` have a non-zero default baseline, most others leave it at zero.
///
/// Relative values are computed from the widget's height.
#[property(BORDER, default(Length::Default))]
pub fn baseline(child: impl IntoUiNode, baseline: impl IntoVar<Length>) -> UiNode {
    let baseline = baseline.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&baseline);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let bounds = WIDGET.bounds();
            let inner_size = bounds.inner_size();
            let default = bounds.baseline();

            let baseline = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(inner_size).with_fill(true, true), || {
                baseline.layout_dft_y(default)
            });
            wl.set_baseline(baseline);

            *final_size = size;
        }
        _ => {}
    })
}

/// Exact size property info.
///
/// Properties like [`size`], [`width`] and [`height`] set this metadata in the widget state.
/// Panels can use this info to implement [`Length::Leftover`] support.
///
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
#[expect(non_camel_case_types)]
pub struct WIDGET_SIZE;
impl WIDGET_SIZE {
    /// Set the width state.
    pub fn set_width(&self, width: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let width = width.into();
            match state.entry(*WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().width = width,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(width, WidgetLength::Default));
                }
            }
        });
    }

    /// Set the height state.
    pub fn set_height(&self, height: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let height = height.into();
            match state.entry(*WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().height = height,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(WidgetLength::Default, height));
                }
            }
        })
    }

    /// Set the size state.
    pub fn set(&self, size: &Size) {
        WIDGET.set_state(*WIDGET_SIZE_ID, euclid::size2((&size.width).into(), (&size.height).into()));
    }

    /// Get the size set in the state.
    pub fn get(&self) -> euclid::Size2D<WidgetLength, ()> {
        WIDGET.get_state(*WIDGET_SIZE_ID).unwrap_or_default()
    }

    /// Get the size set in the widget state.
    pub fn get_wgt(&self, wgt: &mut UiNode) -> euclid::Size2D<WidgetLength, ()> {
        match wgt.as_widget() {
            Some(mut wgt) => wgt.with_context(WidgetUpdateMode::Ignore, || self.get()),
            None => Default::default(),
        }
    }
}

static_id! {
    static ref WIDGET_SIZE_ID: StateId<euclid::Size2D<WidgetLength, ()>>;
}

/// Represents the width or height property value set on a widget.
///
/// Properties like [`size`], [`width`] and [`height`] set the [`WIDGET_SIZE`]
/// metadata in the widget state. Panels can use this info to implement [`Length::Leftover`] support.
///  
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum WidgetLength {
    /// Evaluates to [`PxConstraints2d::fill_size`] when measured, can serve as a request for *size-to-fit*.
    ///
    /// The `Grid!` widget uses this to fit the column and row widgets to *their* cells, as they don't
    /// logically own the cells, this fit needs to be computed by the parent panel.
    ///
    /// [`PxConstraints2d::fill_size`]: zng_wgt::prelude::PxConstraints2d::fill_size
    #[default]
    Default,
    /// The [`Length::Leftover`] value. Evaluates to the [`LayoutMetrics::leftover`] value when measured, if
    /// a leftover value is not provided evaluates like a [`Length::Factor`].
    ///
    /// The *leftover* length needs to be computed by the parent panel, as it depends on the length of the sibling widgets,
    /// not just the panel constraints. Panels that support this, compute the value for each widget and measure/layout each using
    /// [`LAYOUT.with_leftover`] to inject the computed value.
    ///
    /// [`LAYOUT.with_leftover`]: zng_wgt::prelude::LAYOUT::with_leftover
    /// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
    /// [`Length::Factor`]: zng_wgt::prelude::Length::Factor
    /// [`LayoutMetrics::leftover`]: zng_wgt::prelude::LayoutMetrics::leftover
    Leftover(Factor),
    /// Any of the other [`Length`] kinds. All contextual metrics needed to compute these values is already available
    /// in the [`LayoutMetrics`], panels that support [`Length::Leftover`] can layout this widget first to compute the
    /// leftover length.
    ///
    /// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
    /// [`LayoutMetrics`]: zng_wgt::prelude::LayoutMetrics
    /// [`Length`]: zng_wgt::prelude::Length
    Exact,
}

impl From<&Length> for WidgetLength {
    fn from(value: &Length) -> Self {
        match value {
            Length::Default => WidgetLength::Default,
            Length::Leftover(f) => WidgetLength::Leftover(*f),
            _ => WidgetLength::Exact,
        }
    }
}
