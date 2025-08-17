use zng::{APP, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        zng::layout:margin = 0;
    };
}
