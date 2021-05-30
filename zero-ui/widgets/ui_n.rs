use crate::core::{
    context::{LayoutContext, RenderContext, WidgetContext},
    event::{AnyEventArgs, AnyEventUpdate, EventUpdate},
    render::{FrameBuilder, FrameUpdate},
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

            fn event_boxed(&mut self, ctx: &mut WidgetContext, update: AnyEventUpdate, args: &AnyEventArgs) {
                self.event(ctx, update, args);
            }

            fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.event(ctx, update, args),)+
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.measure(ctx, available_size),)+
                }
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.arrange(ctx, final_size),)+
                }
            }

            fn render(&self, ctx: &mut RenderContext, f: &mut FrameBuilder) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.render(ctx, f),)+
                }
            }

            fn render_update(&self, ctx: &mut RenderContext, u: &mut FrameUpdate) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.render_update(ctx, u),)+
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
