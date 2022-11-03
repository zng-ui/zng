use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        #[some_attr]
        when *#is_pressed { }

        #[inline]
        when *#is_pressed { }

        /// docs
        when *#is_pressed { }

        #[allow(unused)]// only this one is not an error
        when *#is_pressed { }
    };
}
