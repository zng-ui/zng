use zng::{APP, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = ;
    };
}
