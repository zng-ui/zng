use zng::{APP, layout::margin, mouse::cursor, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        cursor = #[allow(unused_imports)]
        margin = {
            use zng::layout::PxPoint;
            0
        }
    };
}
