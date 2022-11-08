use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    #[cfg(any())]
    pub use margin as disabled_margin;

    inherit!(zero_ui::core::widget_base::base);

    properties! {
        #[cfg(not(any()))]
        pub zero_ui::properties::margin as allowed_cfg;

        #[cfg(any())]
        pub disabled_margin as disabled_cfg;

        #[allow(unused_imports)]
        pub zero_ui::properties::margin as allowed_lints = {
            use std::vec;
            0
        };

        /// doc
        pub zero_ui::properties::margin as allowed_doc;
    }
}

fn main() {
    let _ = test_widget! {
        allowed_cfg = 0;
        allowed_lints = 0;
        allowed_doc = 0;
        // #[cfg(any())]
        disabled_cfg = 0;
    };
}
