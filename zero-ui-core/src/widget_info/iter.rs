//! Widget info tree iterators.

use std::iter::FusedIterator;

use super::*;

/// Widget tree filter result.
///
/// This `enum` is used by the [`TreeIterator::tree_filter`] method.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum TreeFilter {
    /// Include the descendant and continue filtering its descendants.
    Include,
    /// Skip the descendant but continue filtering its descendants.
    Skip,
    /// Skip the descendant and its descendants.
    SkipAll,
    /// Include the descendant but skips its descendants.
    SkipDescendants,
}
impl_from_and_into_var! {
    /// Returns [`Include`] for `true` and [`Skip`] for `false`.
    ///
    /// [`Include`]: TreeFilter::Include
    /// [`Skip`]: TreeFilter::Skip
    fn from(include: bool) -> TreeFilter {
        if include {
            TreeFilter::Include
        } else {
            TreeFilter::Skip
        }
    }
}

/// Iterator over all children of a widget.
///
/// This `struct` is created by the [`children`] and [`self_and_children`] methods in [`WidgetInfo`].
///
/// [`children`]: WidgetInfo::children
/// [`self_and_children`]: WidgetInfo::self_and_children
#[derive(Debug)]
pub struct Children<'a> {
    front_enter: bool,
    front: Option<WidgetInfo<'a>>,

    back_enter: bool,
    back: Option<WidgetInfo<'a>>,
}
impl<'a> Children<'a> {
    pub(super) fn new(parent: WidgetInfo<'a>) -> Self {
        Children {
            front_enter: true,
            front: Some(parent),

            back_enter: true,
            back: Some(parent),
        }
    }
}
impl<'a> Iterator for Children<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if mem::take(&mut self.front_enter) {
            let next = self.front.take();
            self.front = next.unwrap().first_child();
            next
        } else if self.front == self.back {
            let next = self.front.take();
            self.back = None;
            next
        } else if let Some(next) = self.front {
            self.front = next.next_sibling();
            Some(next)
        } else {
            None
        }
    }
}
impl<'a> DoubleEndedIterator for Children<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if mem::take(&mut self.back_enter) {
            let next = self.back.take();
            self.back = next.unwrap().last_child();
            next
        } else if self.front == self.back {
            let next = self.back.take();
            self.front = None;
            next
        } else if let Some(next) = self.back {
            self.back = next.prev_sibling();
            Some(next)
        } else {
            None
        }
    }
}

/// Iterator over all next siblings of a widget.
///
/// This `struct` is created by the [`prev_siblings`] and [`self_and_prev_siblings`] methods in [`WidgetInfo`].
///
/// [`prev_siblings`]: WidgetInfo::prev_siblings
/// [`self_and_prev_siblings`]: WidgetInfo::self_and_prev_siblings
pub struct PrevSiblings<'a> {
    node: Option<WidgetInfo<'a>>,
}
impl<'a> PrevSiblings<'a> {
    pub(super) fn new(node: WidgetInfo<'a>) -> Self {
        Self { node: Some(node) }
    }
}
impl<'a> Iterator for PrevSiblings<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node {
            self.node = n.prev_sibling();
            Some(n)
        } else {
            None
        }
    }
}

/// Iterator over all next siblings of a widget.
///
/// This `struct` is created by the [`next_siblings`] and [`self_and_next_siblings`] methods in [`WidgetInfo`].
///
/// [`next_siblings`]: WidgetInfo::next_siblings
/// [`self_and_next_siblings`]: WidgetInfo::self_and_next_siblings
pub struct NextSiblings<'a> {
    node: Option<WidgetInfo<'a>>,
}
impl<'a> NextSiblings<'a> {
    pub(super) fn new(node: WidgetInfo<'a>) -> Self {
        Self { node: Some(node) }
    }
}
impl<'a> Iterator for NextSiblings<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node {
            self.node = n.next_sibling();
            Some(n)
        } else {
            None
        }
    }
}

/// Iterator over all ancestors of a widget.
///
/// This `struct` is created by the [`ancestors`] and [`self_and_ancestors`] methods in [`WidgetInfo`].
///
/// [`ancestors`]: WidgetInfo::ancestors
/// [`self_and_ancestors`]: WidgetInfo::self_and_ancestors
pub struct Ancestors<'a> {
    node: Option<WidgetInfo<'a>>,
}
impl<'a> Ancestors<'a> {
    pub(super) fn new(node: WidgetInfo<'a>) -> Self {
        Ancestors { node: Some(node) }
    }
}
impl<'a> Iterator for Ancestors<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node {
            self.node = n.parent();
            Some(n)
        } else {
            None
        }
    }
}

mod internal {
    pub trait InternalTreeIterator {
        fn skip_all(&mut self, widget: super::WidgetInfo);
    }
}

/// Iterator that traverses the branches of a widget tree.
pub trait TreeIterator<'a>: internal::InternalTreeIterator + Iterator<Item = WidgetInfo<'a>> + FusedIterator {
    /// Creates an iterator which uses a closure to filter items or branches at a time.
    ///
    /// See [`TreeFilter`] for details.
    fn tree_filter<F: FnMut(WidgetInfo<'a>) -> TreeFilter>(self, filter: F) -> TreeFilterIter<'a, Self, F>
    where
        Self: Sized,
    {
        TreeFilterIter {
            _lt: PhantomData,
            iter: self,
            filter,
        }
    }

    /// Gets the first item not filtered out by a [`TreeFilter`] closure.
    fn tree_find<F: FnMut(WidgetInfo<'a>) -> TreeFilter>(self, filter: F) -> Option<WidgetInfo<'a>>
    where
        Self: Sized,
    {
        self.tree_filter(filter).next()
    }

    /// Check if any item is not filtered out by a [`TreeFilter`] closure.
    fn tree_any<F: FnMut(WidgetInfo<'a>) -> TreeFilter>(self, filter: F) -> bool
    where
        Self: Sized,
    {
        self.tree_find(filter).is_some()
    }
}

/// Primary implementer of [`TreeIterator`].
pub struct TreeIter<'a> {
    tree: &'a WidgetInfoTree,
    iter: tree::TreeIter,
}
impl<'a> TreeIter<'a> {
    pub(super) fn self_and_descendants(wgt: WidgetInfo<'a>) -> Self {
        Self {
            tree: wgt.tree(),
            iter: wgt.node().self_and_descendants(),
        }
    }

    pub(super) fn self_and_prev_siblings_in(wgt: WidgetInfo<'a>, ancestor: WidgetInfo<'a>) -> RevTreeIter<'a> {
        let tree = &wgt.tree.0.tree;
        let mut iter = ancestor.node().self_and_descendants().rev(tree);
        iter.skip_to(tree, wgt.node_id);

        RevTreeIter { tree: wgt.tree, iter }
    }
    pub(super) fn prev_siblings_in(wgt: WidgetInfo<'a>, ancestor: WidgetInfo<'a>) -> RevTreeIter<'a> {
        if let Some(wgt) = wgt.prev_sibling() {
            Self::self_and_prev_siblings_in(wgt, ancestor)
        } else if let Some(parent) = wgt.parent() {
            if parent != ancestor {
                Self::prev_siblings_in(parent, ancestor)
            } else {
                RevTreeIter {
                    tree: wgt.tree,
                    iter: tree::RevTreeIter::empty(),
                }
            }
        } else {
            RevTreeIter {
                tree: wgt.tree,
                iter: tree::RevTreeIter::empty(),
            }
        }
    }

    pub(super) fn self_and_next_siblings_in(wgt: WidgetInfo<'a>, ancestor: WidgetInfo<'a>) -> Self {
        let mut iter = ancestor.node().self_and_descendants();
        iter.skip_to(wgt.node_id);
        Self { tree: wgt.tree(), iter }
    }
    pub(super) fn next_siblings_in(wgt: WidgetInfo<'a>, ancestor: WidgetInfo<'a>) -> Self {
        if let Some(wgt) = wgt.next_sibling() {
            Self::self_and_next_siblings_in(wgt, ancestor)
        } else if let Some(parent) = wgt.parent() {
            if parent != ancestor {
                Self::next_siblings_in(parent, ancestor)
            } else {
                TreeIter {
                    tree: wgt.tree,
                    iter: tree::TreeIter::empty(),
                }
            }
        } else {
            TreeIter {
                tree: wgt.tree,
                iter: tree::TreeIter::empty(),
            }
        }
    }

    /// Creates a reverse tree iterator.
    ///
    /// Yields widgets in the `parent -> last_child -> prev_sibling` order. The reverse iterator is pre-advanced by the same count
    /// of widgets already yielded by this iterator. In practice this is best used immediately after getting the iterator from
    /// [`self_and_descendants`] or [`descendants`], with the intention of skipping to the last child from the starting widget.
    ///
    /// [`self_and_descendants`]: WidgetInfo::self_and_descendants
    /// [`descendants`]: WidgetInfo::descendants
    pub fn tree_rev(self) -> RevTreeIter<'a>
    where
        Self: Sized,
    {
        RevTreeIter {
            tree: self.tree,
            iter: self.iter.rev(&self.tree.0.tree),
        }
    }
}
impl<'a> internal::InternalTreeIterator for TreeIter<'a> {
    fn skip_all(&mut self, widget: WidgetInfo) {
        self.iter.close(&self.tree.0.tree, widget.node_id)
    }
}
impl<'a> Iterator for TreeIter<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| WidgetInfo::new(self.tree, id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }
}
impl<'a> ExactSizeIterator for TreeIter<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a> FusedIterator for TreeIter<'a> {}
impl<'a> TreeIterator<'a> for TreeIter<'a> {}

/// Reversing tree iterator.
///
/// This struct is created by the [`TreeIter::tree_rev`] method.
pub struct RevTreeIter<'a> {
    tree: &'a WidgetInfoTree,
    iter: tree::RevTreeIter,
}
impl<'a> internal::InternalTreeIterator for RevTreeIter<'a> {
    fn skip_all(&mut self, widget: WidgetInfo) {
        self.iter.close(&self.tree.0.tree, widget.node_id);
    }
}
impl<'a> Iterator for RevTreeIter<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next(&self.tree.0.tree).map(|id| WidgetInfo::new(self.tree, id))
    }
}
impl<'a> FusedIterator for RevTreeIter<'a> {}
impl<'a> TreeIterator<'a> for RevTreeIter<'a> {}

/// Filtering tree iterator.
///
/// This struct is created by the [`TreeIterator::tree_filter`] method.
pub struct TreeFilterIter<'a, I: TreeIterator<'a>, F: FnMut(WidgetInfo<'a>) -> TreeFilter> {
    _lt: PhantomData<&'a WidgetInfoTree>,
    iter: I,
    filter: F,
}
impl<'a, I: TreeIterator<'a>, F: FnMut(WidgetInfo<'a>) -> TreeFilter> internal::InternalTreeIterator for TreeFilterIter<'a, I, F> {
    fn skip_all(&mut self, widget: WidgetInfo) {
        self.iter.skip_all(widget)
    }
}
impl<'a, I, F> Iterator for TreeFilterIter<'a, I, F>
where
    I: TreeIterator<'a>,
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(wgt) => match (self.filter)(wgt) {
                    TreeFilter::Include => return Some(wgt),
                    TreeFilter::Skip => continue,
                    TreeFilter::SkipAll => {
                        self.iter.skip_all(wgt);
                        continue;
                    }
                    TreeFilter::SkipDescendants => {
                        self.iter.skip_all(wgt);
                        return Some(wgt);
                    }
                },
                None => return None,
            }
        }
    }
}
impl<'a, I, F> FusedIterator for TreeFilterIter<'a, I, F>
where
    I: TreeIterator<'a>,
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
}
impl<'a, I, F> TreeIterator<'a> for TreeFilterIter<'a, I, F>
where
    I: TreeIterator<'a>,
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
}

#[cfg(test)]
mod tests {
    use crate::{
        units::FactorUnits,
        widget_info::{iter::TreeIterator, TreeFilter, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder, WidgetInfoTree},
        window::WindowId,
        widget_instance::WidgetId,
    };

    use pretty_assertions::assert_eq;

    trait WidgetInfoBuilderExt {
        fn push_test_widget<F>(&mut self, name: &'static str, inner: F)
        where
            F: FnMut(&mut Self);
    }
    impl WidgetInfoBuilderExt for WidgetInfoBuilder {
        fn push_test_widget<F>(&mut self, name: &'static str, inner: F)
        where
            F: FnMut(&mut Self),
        {
            self.push_widget(WidgetId::named(name), WidgetBoundsInfo::new(), WidgetBorderInfo::new(), inner)
        }
    }

    trait WidgetInfoExt {
        fn test_name(self) -> &'static str;
    }
    impl<'a> WidgetInfoExt for WidgetInfo<'a> {
        fn test_name(self) -> &'static str {
            self.widget_id().name().as_static_str().expect("use with `push_test_widget` only")
        }
    }

    fn data() -> WidgetInfoTree {
        let mut builder = WidgetInfoBuilder::new(
            WindowId::named("w"),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
            None,
        );
        builder.push_test_widget("c-0", |_| {});
        builder.push_test_widget("c-1", |_| {});
        builder.push_test_widget("c-2", |_| {});
        builder.finalize().0
    }

    #[test]
    fn descendants() {
        let tree = data();

        let result: Vec<_> = tree.root().descendants().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-0", "c-1", "c-2"]);
    }

    #[test]
    fn descendants_filter_noop() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["c-0", "c-1", "c-2"]);
    }

    #[test]
    fn descendants_rev() {
        let tree = data();

        let result: Vec<_> = tree.root().descendants().tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-2", "c-1", "c-0"]);
    }

    #[test]
    fn descendants_filter_noop_rev() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_rev()
            .tree_filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["c-2", "c-1", "c-0"]);
    }

    #[test]
    fn self_and_descendants() {
        let tree = data();

        let result: Vec<_> = tree.root().self_and_descendants().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["w", "c-0", "c-1", "c-2"]);
    }

    #[test]
    fn self_and_descendants_filter_noop() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .self_and_descendants()
            .tree_filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["w", "c-0", "c-1", "c-2"]);
    }

    #[test]
    fn self_and_descendants_rev() {
        let tree = data();

        let result: Vec<_> = tree.root().self_and_descendants().tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["w", "c-2", "c-1", "c-0",]);
    }

    #[test]
    fn self_and_descendants_filter_noop_rev() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .self_and_descendants()
            .tree_rev()
            .tree_filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["w", "c-2", "c-1", "c-0",]);
    }

    #[test]
    fn descendants_double() {
        let tree = data();
        let mut iter = tree.root().descendants();

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));

        let result: Vec<_> = iter.tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-1", "c-0"]);
    }

    #[test]
    fn descendants_double_filter_noop() {
        let tree = data();
        let mut iter = tree.root().descendants().tree_rev().tree_filter(|_| TreeFilter::Include);

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-2"));

        let result: Vec<_> = iter.map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-1", "c-0"]);
    }

    fn data_nested() -> WidgetInfoTree {
        let mut builder = WidgetInfoBuilder::new(
            WindowId::named("w"),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
            None,
        );
        builder.push_test_widget("c-0", |builder| {
            builder.push_test_widget("c-0-0", |_| {});
            builder.push_test_widget("c-0-1", |_| {});
            builder.push_test_widget("c-0-2", |_| {});
        });
        builder.push_test_widget("c-1", |builder| {
            builder.push_test_widget("c-1-0", |_| {});
            builder.push_test_widget("c-1-1", |builder| {
                builder.push_test_widget("c-1-1-0", |_| {});
                builder.push_test_widget("c-1-1-1", |_| {});
            });
        });
        builder.push_test_widget("c-2", |builder| {
            builder.push_test_widget("c-2-0", |_| {});
            builder.push_test_widget("c-2-1", |_| {});
            builder.push_test_widget("c-2-2", |builder| {
                builder.push_test_widget("c-2-2-0", |_| {});
            });
        });
        builder.finalize().0
    }

    #[test]
    fn descendants_nested() {
        let tree = data_nested();

        let result: Vec<_> = tree.root().descendants().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec![
                "c-0", "c-0-0", "c-0-1", "c-0-2", "c-1", "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", "c-2", "c-2-0", "c-2-1", "c-2-2",
                "c-2-2-0",
            ]
        );
    }

    #[test]
    fn descendants_nested_rev() {
        let tree = data_nested();

        let result: Vec<_> = tree.root().descendants().tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec![
                "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1",
                "c-0-0",
            ]
        );
    }

    #[test]
    fn self_and_descendants_nested() {
        let tree = data_nested();

        let result: Vec<_> = tree.root().self_and_descendants().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec![
                "w", "c-0", "c-0-0", "c-0-1", "c-0-2", "c-1", "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", "c-2", "c-2-0", "c-2-1", "c-2-2",
                "c-2-2-0",
            ]
        );
    }

    #[test]
    fn self_and_descendants_nested_rev() {
        let tree = data_nested();

        let result: Vec<_> = tree.root().self_and_descendants().tree_rev().map(|w| w.test_name()).collect();
        assert_eq!(
            result,
            vec![
                "w", "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1",
                "c-0-0",
            ]
        );
    }

    #[test]
    fn descendants_double_nested_entering_ok() {
        let tree = data_nested();
        let mut iter = tree.root().descendants();

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));

        let result: Vec<_> = iter.tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec!["c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1", "c-0-0",]
        );
    }

    #[test]
    fn descendants_double_nested() {
        let tree = data_nested();
        let mut iter = tree.root().descendants();

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));
        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0-0"));

        let result: Vec<_> = iter.tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec!["c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1", "c-0-0"]
        );
    }

    fn data_deep() -> WidgetInfoTree {
        let mut builder = WidgetInfoBuilder::new(
            WindowId::named("w"),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
            None,
        );
        builder.push_test_widget("d-0", |builder| {
            builder.push_test_widget("d-1", |builder| {
                builder.push_test_widget("d-2", |builder| {
                    builder.push_test_widget("d-3", |builder| {
                        builder.push_test_widget("d-4", |builder| {
                            builder.push_test_widget("d-5", |_| {});
                        });
                    });
                });
            });
        });
        builder.finalize().0
    }

    #[test]
    fn descendants_deep() {
        let tree = data_deep();
        let result: Vec<_> = tree.root().descendants().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["d-0", "d-1", "d-2", "d-3", "d-4", "d-5"])
    }

    #[test]
    fn descendants_deep_rev() {
        let tree = data_deep();
        let result: Vec<_> = tree.root().descendants().tree_rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["d-0", "d-1", "d-2", "d-3", "d-4", "d-5"])
    }

    #[test]
    fn descendants_deep_double() {
        let tree = data_deep();

        let mut iter = tree.root().descendants().tree_rev().map(|w| w.test_name());
        iter.next();

        let result: Vec<_> = iter.collect();

        assert_eq!(result, vec!["d-1", "d-2", "d-3", "d-4", "d-5"])
    }

    #[test]
    fn descendants_filter_include() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-0", "c-0-0", "c-0-1", "c-0-2", "c-1", "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", "c-2", "c-2-0", "c-2-1", "c-2-2",
                "c-2-2-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::Skip
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-0", "c-0-0", "c-0-1", "c-0-2", /* "c-1", */
                "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", "c-2", "c-2-0", "c-2-1", "c-2-2", "c-2-2-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip_rev() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_rev()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::Skip
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", /* "c-1", */
                "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1", "c-0-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip_all() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipAll
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-0", "c-0-0", "c-0-1", "c-0-2", /* "c-1", "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", */
                "c-2", "c-2-0", "c-2-1", "c-2-2", "c-2-2-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip_all_rev() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_rev()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipAll
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", /* "c-1, c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", */ "c-0", "c-0-2",
                "c-0-1", "c-0-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip_desc() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipDescendants
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-0", "c-0-0", "c-0-1", "c-0-2", "c-1", /* "c-1-0", "c-1-1", "c-1-1-0", "c-1-1-1", */
                "c-2", "c-2-0", "c-2-1", "c-2-2", "c-2-2-0",
            ]
        );
    }

    #[test]
    fn descendants_filter_skip_desc_rev() {
        let tree = data_nested();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .tree_rev()
            .tree_filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipDescendants
                } else {
                    TreeFilter::Include
                }
            })
            .map(|w| w.test_name())
            .collect();

        assert_eq!(
            result,
            vec![
                "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", /* c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", */ "c-0", "c-0-2",
                "c-0-1", "c-0-0",
            ]
        );
    }

    #[test]
    fn self_and_next_siblings_in() {
        let tree = data_nested();

        let root = tree.get("c-1").unwrap();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_next_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_prev_siblings_in_problem_case() {
        let tree = data_nested();

        let root = tree.get("c-1").unwrap();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .tree_rev()
            // .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_next_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_next_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_prev_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .tree_rev()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn next_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1-0").unwrap();

        let result: Vec<_> = item.next_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1-0"))
            .skip(1)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn prev_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1-0").unwrap();

        let result: Vec<_> = item.prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .tree_rev()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1-0"))
            .skip(1)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }
}
