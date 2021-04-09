use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        remove {
            ;
            some::bla::bla::bla::bla::path;
            test1;
            test2,
            test3;
        }
    }
}

fn main() {}
