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
    /// See the [`Focus::focus_disabled_widgets`] config for more on the parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    fn focusable(self, focus_disabled_widgets: bool) -> IterFocusuable<'a, I>;
}
impl<'a, I> IterFocusableExt<'a, I> for I
where
    I: Iterator<Item = WidgetInfo<'a>>,
{
    fn focusable(self, focus_disabled_widgets: bool) -> IterFocusuable<'a, I> {
        IterFocusuable {
            iter: self,
            focus_disabled_widgets,
        }
    }
}

/// Filter a widget info iterator to only focusable items.
///
/// Use [`IterFocusableExt::focusable`] to create.
pub struct IterFocusuable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    iter: I,
    focus_disabled_widgets: bool,
}
impl<'a, I> Iterator for IterFocusuable<'a, I>
where
    I: Iterator<Item = WidgetInfo<'a>>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.as_focusable(self.focus_disabled_widgets) {
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
            if let Some(next) = next.as_focusable(self.focus_disabled_widgets) {
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
pub struct FocusableDescendants<'a, I>
where
    I: TreeIterator<'a>,
{
    _lt: PhantomData<&'a WidgetInfoTree>,
    iter: I,
    focus_disabled_widgets: bool,
}
impl<'a, I> FocusableDescendants<'a, I>
where
    I: TreeIterator<'a>,
{
    pub(super) fn new(iter: I, focus_disabled_widgets: bool) -> Self {
        Self {
            _lt: PhantomData,
            iter,
            focus_disabled_widgets,
        }
    }

    /// Filter out entire branches of descendants at a time.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    ///
    /// [`TreeFilter`]: w_iter::TreeFilter
    pub fn tree_filter<F>(self, mut filter: F) -> FocusableFilterDescendants<'a, I, impl FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter>
    where
        F: FnMut(WidgetFocusInfo<'a>) -> w_iter::TreeFilter,
    {
        FocusableFilterDescendants {
            iter: self.iter.tree_filter(move |w| {
                if let Some(f) = w.as_focusable(self.focus_disabled_widgets) {
                    filter(f)
                } else {
                    w_iter::TreeFilter::Skip
                }
            }),
            focus_disabled_widgets: self.focus_disabled_widgets,
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
impl<'a, I> Iterator for FocusableDescendants<'a, I>
where
    I: TreeIterator<'a>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for next in self.iter.by_ref() {
            if let Some(next) = next.as_focusable(self.focus_disabled_widgets) {
                return Some(next);
            }
        }
        None
    }
}
impl<'a, I> DoubleEndedIterator for FocusableDescendants<'a, I>
where
    I: TreeIterator<'a>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(next) = self.iter.next_back() {
            if let Some(next) = next.as_focusable(self.focus_disabled_widgets) {
                return Some(next);
            }
        }
        None
    }
}

/// An iterator that filters a focusable widget tree.
///
/// This `struct` is created by the [`FocusableDescendants::tree_filter`] method. See its documentation for more.
pub struct FocusableFilterDescendants<'a, I, F>
where
    I: TreeIterator<'a>,
    F: FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter,
{
    iter: w_iter::TreeFilterIter<'a, I, F>,
    focus_disabled_widgets: bool,
}
impl<'a, I, F> Iterator for FocusableFilterDescendants<'a, I, F>
where
    F: FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter,
    I: TreeIterator<'a>,
{
    type Item = WidgetFocusInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|w| w.as_focus_info(self.focus_disabled_widgets))
    }
}
impl<'a, I, F> DoubleEndedIterator for FocusableFilterDescendants<'a, I, F>
where
    F: FnMut(WidgetInfo<'a>) -> w_iter::TreeFilter,
    I: TreeIterator<'a>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|w| w.as_focus_info(self.focus_disabled_widgets))
    }
}
