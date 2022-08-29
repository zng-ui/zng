use zero_ui::core::*;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        #[priority_index = 10]
        margin as ok0;

        #[priority_index]
        margin as e1;
        
        #[priority_index = ]
        margin as e2;

        #[priority_index = 10.0]
        margin as e3;

        #[priority_index = "10"]
        margin as e4;
        
        #[priority_index(10)]
        margin as e5;
    }
}

fn main() {

}
