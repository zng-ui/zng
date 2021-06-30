use zero_ui::prelude::*;

fn main() {
    let wgt = blank! {
        margin = 10;

        when self.enabled {
            margin = 20;
        }
    };
}
