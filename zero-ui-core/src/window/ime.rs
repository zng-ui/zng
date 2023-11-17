use crate::{event::*, widget_info::InteractionPath, text::Txt};

event_args! {
    /// Arguments for [`IME_EVENT`].
    pub struct ImeArgs {
        /// Text input widget.
        pub target: InteractionPath,

        /// The text, preview or actual insert.
        pub txt: Txt,

        /// Caret/selection within the `txt`.
        /// 
        /// The indexes are in byte offsets and indicate where the caret or selection must be placed on
        /// the inserted or preview `txt`, if not set the position is at the end of the insert.
        pub caret: Option<(usize, usize)>,

        /// If the text must be actually inserted.
        /// 
        /// If `true` the [`txt`] must be actually inserted at the position the caret is in, the caret then can be moved to
        /// after the inserted text or to [`caret`] if it set.
        /// 
        /// If `false` the widget must visually adjust the text and caret to look as if the input has committed, but the
        /// actual text must not be altered, and if the [`txt`] is empty the previous caret/selection must be restored.
        /// Usually the preview text is rendered with an underline effect, otherwise it has the same appearance as the
        /// committed text.
        /// 
        /// [`txt`]: Self::txt
        /// [`caret`]: Self::caret
        pub commit: bool,

        ..

        /// Target.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
        }
    }
}

event! {
    /// IME event targeting a text input widget.
    pub static IME_EVENT: ImeArgs;
}