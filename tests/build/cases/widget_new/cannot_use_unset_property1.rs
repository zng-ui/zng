use zero_ui::core::widget;

#[widget($crate::foo)]
pub mod foo {
    use zero_ui::properties::margin;

    properties! {
        margin = 10;
    }
}

fn main() {
    let _ = foo! {
        margin = unset!;
        when *#margin == 0 { }
    };
}
