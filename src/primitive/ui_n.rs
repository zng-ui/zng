use crate::core::*;

macro_rules! ui_n {
    ($UiEnum: ident { $($Ui: ident),+ }) => {
        /// Helper type for returning more then one type of [Ui].
        ///
        /// There is a helper enum type for 2 to 8 alternative Uis
        /// named `Ui2-8`, if you need more then 8 return a `Box<dyn Ui>` using
        /// [into_box]([Ui::into_box).
        ///
        /// # Example
        /// ```
        /// # use zero_ui::{core::*, primitive::*, *};
        /// # fn restart_btn() -> impl Ui { text("Restart") }
        /// #
        /// fn countdown(n: usize) -> impl Ui {
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
        pub enum $UiEnum<$($Ui: Ui),+> {
            $($Ui($Ui)),+
        }

        impl<$($Ui: Ui),+> Ui for $UiEnum<$($Ui),+> {
            fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.init(values, update),)+
                }
            }

            fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
                match self {
                    $($UiEnum::$Ui(ui) => ui.measure(available_size),)+
                }
            }

            fn arrange(&mut self, final_size: LayoutSize) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.arrange(final_size),)+
                }
            }

            fn render(&self, f: &mut NextFrame) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.render(f),)+
                }
            }

            fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.keyboard_input(input, values, update),)+
                }
            }

            fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.window_focused(focused, values, update),)+
                }
            }

            fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.focus_changed(change, values, update),)+
                }
            }

            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.mouse_input(input, hits, values, update),)+
                }
            }

            fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.mouse_move(input, hits, values, update),)+
                }
            }

            fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.mouse_entered(values, update),)+
                }
            }

            fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.mouse_left(values, update),)+
                }
            }

            fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.close_request(values, update),)+
                }
            }

            fn focus_status(&self) -> Option<FocusStatus> {
                match self {
                    $($UiEnum::$Ui(ui) => ui.focus_status(),)+
                }
            }

            fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
                match self {
                    $($UiEnum::$Ui(ui) => ui.point_over(hits),)+
                }
            }

            fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.value_changed(values, update),)+
                }
            }

            fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                match self {
                    $($UiEnum::$Ui(ui) => ui.parent_value_changed(values, update),)+
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
