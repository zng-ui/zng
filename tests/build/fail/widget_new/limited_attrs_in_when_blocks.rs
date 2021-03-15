use zero_ui::properties::margin;
use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        #[some_attr]
        when self.is_pressed { }

        #[inline]
        when self.is_pressed { }

        /// docs
        when self.is_pressed { }

        #[allow(unused)]// only this one is not an error
        when self.is_pressed { }
    };
}
