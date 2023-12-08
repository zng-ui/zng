use std::ops;

/// Start and manage an app process.
///
/// # View Process
///
/// A view-process must be initialized before starting an app. Panics on `run` if there is
/// no view-process, also panics if the current process is already executing as a view-process.
pub struct APP;
impl ops::Deref for APP {
    type Target = zero_ui_app::APP;

    fn deref(&self) -> &Self::Target {
        &zero_ui_app::APP
    }
}

mod defaults {
    use zero_ui_app::{window::WINDOW, AppExtended, AppExtension};
    use zero_ui_color::COLOR_SCHEME_VAR;
    use zero_ui_ext_clipboard::ClipboardManager;
    use zero_ui_ext_config::ConfigManager;
    use zero_ui_ext_font::FontManager;
    use zero_ui_ext_fs_watcher::FsWatcherManager;
    use zero_ui_ext_image::ImageManager;
    use zero_ui_ext_input::{
        focus::FocusManager, gesture::GestureManager, keyboard::KeyboardManager, mouse::MouseManager,
        pointer_capture::PointerCaptureManager, touch::TouchManager,
    };
    use zero_ui_ext_l10n::L10nManager;
    use zero_ui_ext_undo::UndoManager;
    use zero_ui_ext_window::{WINDOW_Ext as _, WindowManager, WINDOWS};
    use zero_ui_var::Var as _;
    use zero_ui_wgt::nodes::with_context_var_init;

    impl super::APP {
        /// App with default extensions.
        ///     
        /// # Extensions
        ///
        /// Extensions included.
        ///
        /// * [`FsWatcherManager`]
        /// * [`ConfigManager`]
        /// * [`L10nManager`]
        /// * [`PointerCaptureManager`]
        /// * [`MouseManager`]
        /// * [`TouchManager`]
        /// * [`KeyboardManager`]
        /// * [`GestureManager`]
        /// * [`WindowManager`]
        /// * [`FontManager`]
        /// * [`FocusManager`]
        /// * [`ImageManager`]
        /// * [`ClipboardManager`]
        /// * [`UndoManager`]
        /// * [`MaterialFonts`] if `cfg(feature = "material_icons")`.
        ///
        /// [`MaterialFonts`]: zero_ui_wgt_material_icons::MaterialFonts
        pub fn defaults(&self) -> AppExtended<impl AppExtension> {
            let r = self
                .minimal()
                .extend(FsWatcherManager::default())
                .extend(ConfigManager::default())
                .extend(L10nManager::default())
                .extend(PointerCaptureManager::default())
                .extend(MouseManager::default())
                .extend(TouchManager::default())
                .extend(KeyboardManager::default())
                .extend(GestureManager::default())
                .extend(WindowManager::default())
                .extend(FontManager::default())
                .extend(FocusManager::default())
                .extend(ImageManager::default())
                .extend(ClipboardManager::default())
                .extend(UndoManager::default());

            #[cfg(feature = "material_icons")]
            let r = r.extend(zero_ui_wgt_material_icons::MaterialFonts);

            r.extend(DefaultsInit {})
        }
    }

    struct DefaultsInit {}
    impl AppExtension for DefaultsInit {
        fn init(&mut self) {
            WINDOWS.register_root_extender(|a| {
                // actualize LANG_VAR early and set layout direction.
                let child = zero_ui_wgt_text::lang(child, zero_ui_ext_l10n::LANG_VAR);

                // optimization, actualize mapping context-vars early, see `context_var!` docs.
                // !!: TODO, review this after Var::map specialization work.
                let child = zero_ui_wgt_text::font_palette(child, zero_ui_wgt_text::FONT_PALETTE_VAR);

                child
            });

            /*


                        * Add `WINDOWS.register_root_extender` on the default app?
                - `FONTS.system_font_aa`.
                - color scheme.
                - `font_palette = crate::widgets::text::FONT_PALETTE_VAR;` for performance? (// optimization, actualize mapping context-vars early, see `context_var!` docs.)
                - `font_size = crate::widgets::text::FONT_SIZE_VAR;`
                - `font_color = color_scheme_map(rgb(0.92, 0.92, 0.92), rgb(0.08, 0.08, 0.08));`
            ```rust
            // removed from core

            // removed from text
            /// Default selection toolbar.
            ///
            /// See [`SELECTION_TOOLBAR_FN_VAR`] for more details.
            pub fn default_selection_toolbar(args: SelectionToolbarArgs) -> impl UiNode {
                use super::commands::*;

                ContextMenu! {
                    style_fn = menu::context::TouchStyle!();
                    children = ui_vec![
                        menu::TouchCmdButton!(COPY_CMD.scoped(args.anchor_id)),
                        menu::TouchCmdButton!(SELECT_ALL_CMD.scoped(args.anchor_id)),
                    ];
                }
            }

            ```
                         */
        }
    }
}
