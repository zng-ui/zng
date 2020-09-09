use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameBuilder,
    units::LayoutSize,
    UiNode,
};

macro_rules! ui_n {
    ($UiEnum: ident { $($UiNode: ident),+ }) => {
        #[doc(hidden)]
        pub enum $UiEnum<$($UiNode: UiNode),+> {
            $($UiNode($UiNode)),+
        }

        impl<$($UiNode: UiNode),+> UiNode for $UiEnum<$($UiNode),+> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.init(ctx),)+
                }
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.deinit(ctx),)+
                }
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.update(ctx),)+
                }
            }

            fn update_hp(&mut self, ctx: &mut WidgetContext) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.update_hp(ctx),)+
                }
            }

            fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.measure(available_size, ctx),)+
                }
            }

            fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.arrange(final_size, ctx),)+
                }
            }

            fn render(&self, f: &mut FrameBuilder) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.render(f),)+
                }
            }
        }
    };
}

ui_n!(Ui2 { A, B });
ui_n!(Ui3 { A, B, C });
ui_n!(Ui4 { A, B, C, D });
ui_n!(Ui5 { A, B, C, D, E });
ui_n!(Ui6 { A, B, C, D, E, F });
ui_n!(Ui7 { A, B, C, D, E, F, G });
ui_n!(Ui8 { A, B, C, D, E, F, G, H });
