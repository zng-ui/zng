use crate::core::{
    context::{LayoutContext, RenderContext, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameInfoBuilder, FrameBuilder, FrameUpdate},
    units::*,
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

            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.event(ctx, args),)+
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.measure(ctx, available_size),)+
                }
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.arrange(ctx, final_size),)+
                }
            }

            fn frame_info(&self, ctx: &mut RenderContext, info: &mut FrameInfoBuilder) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.frame_info(ctx, info),)+
                }
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.render(ctx, frame),)+
                }
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.render_update(ctx, update),)+
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
