use crate::core::*;

macro_rules! ui_n {
    ($UiEnum: ident { $($UiNode: ident),+ }) => {
        /// Helper type for returning more then one type of [UiNode].
        ///
        /// There is a helper enum type for 2 to 8 alternative Uis
        /// named `Ui2-8`, if you need more then 8 return a `Box<dyn UiNode>` using
        /// [boxed]([UiNode::boxed).
        ///
        /// # Example
        /// ```
        /// # use zero_ui::{core::*, properties::*, *};
        /// # fn restart_btn() -> impl UiNode { text("Restart") }
        /// #
        /// fn countdown(n: usize) -> impl UiNode {
        ///     if n > 0 {
        ///         Ui2::A(text(format!("{}!", n)))
        ///     } else {
        ///         Ui2::B(v_stack((
        ///             ui! {
        ///                 text_color: rgb(0, 0, 128);
        ///                 => text("Congratulations!!")
        ///             },
        ///             restart_btn()
        ///         ).into()))
        ///     }
        /// }
        /// ```
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

            fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.measure(available_size),)+
                }
            }

            fn arrange(&mut self, final_size: LayoutSize) {
                match self {
                    $($UiEnum::$UiNode(ui) => ui.arrange(final_size),)+
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
