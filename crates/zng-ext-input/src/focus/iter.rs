//! Focusable info tree iterators.
//!

use zng_app::widget::info::{
    WidgetInfo,
    iter::{self as w_iter, TreeIterator},
};

use super::*;

/// Filter-maps an iterator of [`WidgetInfo`] to [`WidgetFocusInfo`].
///
///  [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
pub trait IterFocusableExt<I: Iterator<Item = WidgetInfo>> {
    /// Returns an iterator of only the focusable widgets.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] and [`FOCUS.focus_hidden_widgets`] config for more on the parameter.
    ///
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    fn focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> IterFocusable<I>;
}
impl<I> IterFocusableExt<I> for I
where
    I: Iterator<Item = WidgetInfo>,
{
    fn focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> IterFocusable<I> {
        IterFocusable {
            iter: self,
            mode: FocusMode::new(focus_disabled_widgets, focus_hidden_widgets),
        }
    }
}

/// Filter a widget info iterator to only focusable items.
///
/// Use [`IterFocusableExt::focusable`] to create.
pub struct IterFocusable<I: Iterator<Item = WidgetInfo>> {
    iter: I,
    mode: FocusMode,
}
impl<I> Iterator for IterFocusable<I>
where
    I: Iterator<Item = WidgetInfo>,
{
    type Item = WidgetFocusInfo;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.into_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                return Some(next);
            }
        }
        None
    }
}
impl<I> DoubleEndedIterator for IterFocusable<I>
where
    I: Iterator<Item = WidgetInfo> + DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(next) = self.iter.next_back() {
            if let Some(next) = next.into_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                return Some(next);
            }
        }
        None
    }
}

/// Iterator over all focusable items in a branch of the widget tree.
///
/// This `struct` is created by the [`descendants`] and [`self_and_descendants`] methods on [`WidgetFocusInfo`].
/// See its documentation for more.
///
/// [`descendants`]: WidgetFocusInfo::descendants
/// [`self_and_descendants`]: WidgetFocusInfo::self_and_descendants
pub struct FocusTreeIter<I>
where
    I: TreeIterator,
{
    iter: I,
    mode: FocusMode,
}
impl<I> FocusTreeIter<I>
where
    I: TreeIterator,
{
    pub(super) fn new(iter: I, mode: FocusMode) -> Self {
        Self { iter, mode }
    }

    /// Filter out entire branches of descendants at a time.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_filter<F>(self, mut filter: F) -> FocusTreeFilterIter<I, impl FnMut(&WidgetInfo) -> w_iter::TreeFilter>
    where
        F: FnMut(&WidgetFocusInfo) -> w_iter::TreeFilter,
    {
        FocusTreeFilterIter {
            iter: self.iter.tree_filter(move |w| {
                if let Some(f) = w
                    .clone()
                    .into_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN))
                {
                    filter(&f)
                } else {
                    w_iter::TreeFilter::Skip
                }
            }),
            mode: self.mode,
        }
    }

    /// Returns the first focusable included by `filter`.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_find<F>(self, filter: F) -> Option<WidgetFocusInfo>
    where
        F: FnMut(&WidgetFocusInfo) -> w_iter::TreeFilter,
    {
        self.tree_filter(filter).next()
    }

    /// Returns if the `filter` allows any focusable.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_any<F>(self, filter: F) -> bool
    where
        F: FnMut(&WidgetFocusInfo) -> w_iter::TreeFilter,
    {
        self.tree_find(filter).is_some()
    }
}
impl FocusTreeIter<w_iter::TreeIter> {
    /// Creates a reverse tree iterator.
    pub fn tree_rev(self) -> FocusTreeIter<w_iter::RevTreeIter> {
        FocusTreeIter::new(self.iter.tree_rev(), self.mode)
    }
}

impl<I> Iterator for FocusTreeIter<I>
where
    I: TreeIterator,
{
    type Item = WidgetFocusInfo;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.into_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                return Some(next);
            }
        }
        None
    }
}

/// An iterator that filters a focusable widget tree.
///
/// This `struct` is created by the [`FocusTreeIter::tree_filter`] method. See its documentation for more.
pub struct FocusTreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> w_iter::TreeFilter,
{
    iter: w_iter::TreeFilterIter<I, F>,
    mode: FocusMode,
}
impl<I, F> Iterator for FocusTreeFilterIter<I, F>
where
    F: FnMut(&WidgetInfo) -> w_iter::TreeFilter,
    I: TreeIterator,
{
    type Item = WidgetFocusInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|w| w.into_focus_info(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)))
    }
}
