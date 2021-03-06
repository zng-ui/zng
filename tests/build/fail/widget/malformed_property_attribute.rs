use zero_ui::core::widget2;

#[widget2($crate::widget)]
pub mod widget {
    use zero_ui::properties::margin;

    properties! {
        #![allow(inner_attribute)]
        #[!foo]
        /// valid
        margin = 10;
    }
}

fn main() {}
