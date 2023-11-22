use crate::{event::*, text::Txt, widget_info::WidgetPath};

event_args! {
    /// Arguments for [`IME_EVENT`].
    pub struct ImeArgs {
        /// The enabled text input widget.
        pub target: WidgetPath,

        /// The text, preview or actual insert.
        pub txt: Txt,

        /// Caret/selection within the `txt` when it is preview.
        ///
        /// The indexes are in byte offsets and indicate where the caret or selection must be placed on
        /// the inserted or preview `txt`, if not set the position is at the end of the insert.
        ///
        /// If this is `None` the text must [`commit`].
        ///
        /// [`commit`]: Self::commit
        pub preview_caret: Option<(usize, usize)>,

        ..

        /// Target.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
        }
    }
}
impl ImeArgs {
    /// If the text must be actually inserted.
    ///
    /// If `true` the [`txt`] must be actually inserted at the last non-preview caret/selection, the caret then must be moved to
    /// after the inserted text.
    ///
    /// If `false` the widget must visually adjust the text and caret to look as if the input has committed, but the
    /// actual text must not be altered, and if the [`txt`] is empty the previous caret/selection must be restored.
    /// Usually the preview text is rendered with an underline effect, otherwise it has the same appearance as the
    /// committed text.
    ///
    /// [`txt`]: Self::txt
    /// [`caret`]: Self::caret
    pub fn commit(&self) -> bool {
        self.preview_caret.is_none()
    }
}

event! {
    /// IME event targeting a text input widget.
    pub static IME_EVENT: ImeArgs;
}
