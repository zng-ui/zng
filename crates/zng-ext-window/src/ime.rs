use zng_app::{
    event::{event, event_args},
    widget::info::{WidgetInfo, WidgetInfoBuilder, WidgetPath},
};
use zng_state_map::{StateId, static_id};
use zng_txt::Txt;

event_args! {
    /// Arguments for [`IME_EVENT`].
    pub struct ImeArgs {
        /// The enabled text input widget.
        pub target: WidgetPath,

        /// The text, preview or actual insert.
        pub txt: Txt,

        /// Caret/selection within the `txt` when it is preview.
        ///
        /// The indexes are in char byte offsets and indicate where the caret or selection must be placed on
        /// the inserted or preview `txt`, if not set the position is at the end of the insert.
        ///
        /// If this is `None` the text must [`commit`].
        ///
        /// [`commit`]: Self::commit
        pub preview_caret: Option<(usize, usize)>,

        ..

        /// Target.
        fn is_in_target(&self, id: WidgetId) -> bool {
            self.target.contains(id)
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
    pub fn commit(&self) -> bool {
        self.preview_caret.is_none()
    }
}

event! {
    /// Input Method Editor event targeting a text input widget.
    pub static IME_EVENT: ImeArgs;
}

/// IME extension methods for [`WidgetInfo`].
///
/// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
pub trait WidgetInfoImeArea {
    /// If widget defines an IME exclusion area in the window space when it is focused.
    fn is_ime_area(&self) -> bool;
}

/// IME extension methods for [`WidgetInfoBuilder`].
///
/// [`WidgetInfoBuilder`]: zng_app::widget::info::WidgetInfoBuilder
pub trait WidgetInfoBuilderImeArea {
    /// Flag this widget as an IME exclusion area in the window space when it is focused.
    fn flag_is_ime_area(&mut self);
}

static_id! {
    static ref IS_IME_AREA: StateId<()>;
}

impl WidgetInfoImeArea for WidgetInfo {
    fn is_ime_area(&self) -> bool {
        self.meta().flagged(*IS_IME_AREA)
    }
}

impl WidgetInfoBuilderImeArea for WidgetInfoBuilder {
    fn flag_is_ime_area(&mut self) {
        self.flag_meta(*IS_IME_AREA);
    }
}
