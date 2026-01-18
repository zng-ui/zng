use std::sync::Arc;

use atomic::Atomic;
use zng_app::{
    event::{event, event_args},
    widget::info::{WidgetInfo, WidgetInfoBuilder, WidgetPath},
};
use zng_layout::unit::PxRect;
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
    /// IME exclusion area in the window space.
    ///
    /// Widgets are IME targets when they are focused and subscribe to [`IME_EVENT`]. This
    /// value is an area the IME window should avoid covering, by default it is the widget inner-bounds,
    /// but the widget can override it using [`set_ime_area`].
    ///
    /// This value can change after every render update.
    ///
    /// [`set_ime_area`]: WidgetInfoBuilderImeArea::set_ime_area
    fn ime_area(&self) -> PxRect;
}

/// IME extension methods for [`WidgetInfoBuilder`].
///
/// [`WidgetInfoBuilder`]: zng_app::widget::info::WidgetInfoBuilder
pub trait WidgetInfoBuilderImeArea {
    /// Set a custom [`ime_area`].
    ///
    /// The value can be updated every frame using interior mutability, without needing to rebuild the info.
    ///
    /// [`ime_area`]: WidgetInfoImeArea::ime_area
    fn set_ime_area(&mut self, area: Arc<Atomic<PxRect>>);
}

static_id! {
    static ref IME_AREA: StateId<Arc<Atomic<PxRect>>>;
}

impl WidgetInfoImeArea for WidgetInfo {
    fn ime_area(&self) -> PxRect {
        self.meta()
            .get(*IME_AREA)
            .map(|r| r.load(atomic::Ordering::Relaxed))
            .unwrap_or_else(|| self.inner_bounds())
    }
}

impl WidgetInfoBuilderImeArea for WidgetInfoBuilder {
    fn set_ime_area(&mut self, area: Arc<Atomic<PxRect>>) {
        self.set_meta(*IME_AREA, area);
    }
}
