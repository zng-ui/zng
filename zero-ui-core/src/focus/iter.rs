//! Focusable info tree iterators.
//!

use std::marker::PhantomData;

use super::*;

use crate::widget_info::{
    iter::{self as w_iter, TreeIterator},
    WidgetInfo,
};

/// Filter-maps an iterator of [`WidgetInfo`] to [`WidgetFocusInfo`].
pub trait IterFocusableExt<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    /// Returns an iterator of only the focusable widgets.
    ///
    /// See the [`Focus::focus_disabled_widgets`] and [`Focus::focus_hidden_widgets`] config for more on the parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    /// [`Focus::focus_hidden_widgets`]: crate::focus::Focus::focus_hidden_widgets
    fn focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> IterFocusuable<'a, I>;
}
impl<'a, I> IterFocusableExt<'a, I> for I
where
    I: Iterator<Item = WidgetInfo<'a>>,
{
    fn focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> IterFocusuable<'a, I> {
        IterFocusuable {
            iter: self,
            mode: FocusMode::new(focus_disabled_widgets, focus_hidden_widgets),
        }
    }
}

/// Filter a widget info iterator to only focusable items.
///
/// Use [`IterFocusableExt::focusable`] to create.
pub struct IterFocusuable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    iter: I,
    mode: FocusMode,
}
impl<'a, I> Iterator for IterFocusuable<'a, I>
where
    I: Iterator<Item = WidgetInfo<'a>>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.as_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                return Some(next);
            }
        }
        None
    }
}
impl<'a, I> DoubleEndedIterator for IterFocusuable<'a, I>
where
    I: Iterator<Item = WidgetInfo<'a>> + DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(next) = self.iter.next_back() {
            if let Some(next) = next.as_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
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
pub struct FocusTreeIter<'a, I>
where
    I: TreeIterator<'a>,
{
    _lt: PhantomData<&'a WidgetInfoTree>,
    iter: I,
    mode: FocusMode,
}
impl<'a, I> FocusTreeIter<'a, I>
where
    I: TreeIterator<'a>,
{
    pub(super) fn new(iter: I, mode: FocusMode) -> Self {
        Self {
            _lt: PhantomData,
            iter,
            mode,
        }
    }

    /// Filter out entire branches of descendants at a time.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_filter<F>(self, mut filter: F) -> FocusTreeFilterIter<'a, I, impl FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter>
    where
        F: FnMut(WidgetFocusInfo<'a>) -> w_iter::TreeFilter,
    {
        FocusTreeFilterIter {
            iter: self.iter.tree_filter(move |w| {
                if let Some(f) = w.as_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                    filter(f)
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
    pub fn tree_find<F>(self, filter: F) -> Option<WidgetFocusInfo<'a>>
    where
        F: FnMut(WidgetFocusInfo<'a>) -> w_iter::TreeFilter,
    {
        #[allow(clippy::filter_next)]
        self.tree_filter(filter).next()
    }

    /// Returns if the `filter` allows any focusable.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_any<F>(self, filter: F) -> bool
    where
        F: FnMut(WidgetFocusInfo<'a>) -> w_iter::TreeFilter,
    {
        self.tree_find(filter).is_some()
    }
}
impl<'a> FocusTreeIter<'a, w_iter::TreeIter<'a>> {
    /// Creates a reverse tree iterator.
    pub fn tree_rev(self) -> FocusTreeIter<'a, w_iter::RevTreeIter<'a>> {
        FocusTreeIter::new(self.iter.tree_rev(), self.mode)
    }
}

impl<'a, I> Iterator for FocusTreeIter<'a, I>
where
    I: TreeIterator<'a>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.as_focusable(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)) {
                return Some(next);
            }
        }
        None
    }
}

/// An iterator that filters a focusable widget tree.
///
/// This `struct` is created by the [`FocusTreeIter::tree_filter`] method. See its documentation for more.
pub struct FocusTreeFilterIter<'a, I, F>
where
    I: TreeIterator<'a>,
    F: FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter,
{
    iter: w_iter::TreeFilterIter<'a, I, F>,
    mode: FocusMode,
}
impl<'a, I, F> Iterator for FocusTreeFilterIter<'a, I, F>
where
    F: FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter,
    I: TreeIterator<'a>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|w| w.as_focus_info(self.mode.contains(FocusMode::DISABLED), self.mode.contains(FocusMode::HIDDEN)))
    }
}
