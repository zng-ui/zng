use zero_ui::core::widget;

#[widget($crate::base_wgt)]
pub mod base_wgt {
    use zero_ui::core::{widget_base::implicit_base, UiNode, Widget, WidgetId};
    properties! {
        remove { id }

        #[allowed_in_when = false]
        root_id: WidgetId = WidgetId::new_unique();
    }

    fn new(child: impl UiNode, root_id: WidgetId) -> impl Widget {
        implicit_base::new(child, root_id)
    }
}

#[widget($crate::test_wgt)]
pub mod test_wgt {
    inherit!(super::base_wgt);
}

fn main() {
    use zero_ui::core::WidgetId;

    // ok
    let _ = test_wgt! {
        root_id = WidgetId::new_unique();
    };

    // expect id not found
    let _ = test_wgt! {
        id = WidgetId::new_unique();
    };
}
