use zng::{widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        zng::layout:: = 0;
    };
}
