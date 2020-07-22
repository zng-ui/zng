use zero_ui::{core::focus::TabIndex, prelude::*};

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        let size = var((800., 600.));
        let title = size.map(|s: &LayoutSize| formatx!("Button Example - {}x{}", s.width.ceil(), s.height.ceil()));
        window! {
            size: size;
            title: title;
            content: v_stack! {
                spacing: 5.0;
                items: ui_vec![
                    example("Button 5", TabIndex(5)), 
                    example("Button 4", TabIndex(3)),
                    example("Button 3", TabIndex(2)),
                    example("Button 1", TabIndex(0)),
                    example("Button 2", TabIndex(0)),
                ];
            };
        }
    })
}

fn example(content: impl Into<Text>, tab_index: TabIndex) -> impl Widget {
    button! {
        align: Alignment::CENTER;
        content: text(content.into());
        tab_index;
    }
}
