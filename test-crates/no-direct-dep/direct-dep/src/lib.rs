use zero_ui::core::{widget, Widget};

widget! {
    pub test_widget;
}

pub fn is_test_widget(wgt: impl Widget) -> bool {
    true
}

pub use zero_ui::crate_reference_call as zero_ui_ref_call;
