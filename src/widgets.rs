//! Common widgets.

pub mod layouts;
pub mod mixins;
pub mod text;

mod button_;
mod container_;
mod window_;

mod fill_color;
mod gradient;
mod line_;
mod ui_n;
mod view_;

pub use button_::*;
pub use container_::*;
pub use fill_color::*;
pub use gradient::*;
pub use line_::*;
pub use ui_n::*;
pub use view_::*;
pub use window_::*;

/// My widget docs.
#[crate::core::widget2($crate::widgets::docs_sample_wgt)]
pub mod docs_sample_wgt {
    use crate::core::color::colors;
    use crate::properties::background::background_color;
    use crate::properties::states::is_hovered;

    properties! {
        /// My property docs.
        background_color = colors::RED;

        /// My when docs.
        when self.is_hovered {
            background_color = colors::BLUE;
        }
    }
}
