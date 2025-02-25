//! Widget info tree iterators.
use std::iter::FusedIterator;

use zng_var::impl_from_and_into_var;

use super::*;

/// Widget tree filter selected for a widget in the tree.
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
pub struct Children {
    front_enter: bool,
    front: Option<WidgetInfo>,

    back_enter: bool,
    back: Option<WidgetInfo>,
}
impl Children {
    pub(super) fn new(parent: WidgetInfo) -> Self {
        Self {
            front_enter: true,
            front: Some(parent.clone()),

            back_enter: true,
            back: Some(parent),
        }
    }

    /// New empty iterator.
    pub fn empty() -> Self {
        Self {
            front_enter: false,
            front: None,
            back_enter: false,
            back: None,
        }
    }

    /// New with a children selection.
    pub fn new_range(front: WidgetInfo, back: WidgetInfo) -> Self {
        assert_eq!(
            front.node().parent().unwrap().id(),
            back.node().parent().unwrap().id(),
            "front and back not siblings"
        );
        Self {
            front_enter: false,
            front: Some(front),
            back_enter: false,
            back: Some(back),
        }
    }
}
impl Iterator for Children {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if mem::take(&mut self.front_enter) {
            let next = self.front.take().unwrap();
            self.front = next.first_child();
            Some(next)
        } else if self.front == self.back {
            let next = self.front.take();
            self.back = None;
            next
        } else if let Some(next) = self.front.take() {
            self.front = next.next_sibling();
            Some(next)
        } else {
            None
        }
    }
}
impl DoubleEndedIterator for Children {
    fn next_back(&mut self) -> Option<Self::Item> {
        if mem::take(&mut self.back_enter) {
            let next = self.back.take().unwrap();
            self.back = next.last_child();
            Some(next)
        } else if self.front == self.back {
            let next = self.back.take();
            self.front = None;
            next
        } else if let Some(next) = self.back.take() {
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
pub struct PrevSiblings {
    node: Option<WidgetInfo>,
}
impl PrevSiblings {
    pub(super) fn new(node: WidgetInfo) -> Self {
        Self { node: Some(node) }
    }
}
impl Iterator for PrevSiblings {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
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
pub struct NextSiblings {
    node: Option<WidgetInfo>,
}
impl NextSiblings {
    pub(super) fn new(node: WidgetInfo) -> Self {
        Self { node: Some(node) }
    }
}
impl Iterator for NextSiblings {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
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
pub struct Ancestors {
    node: Option<WidgetInfo>,
}
impl Ancestors {
    pub(super) fn new(node: WidgetInfo) -> Self {
        Ancestors { node: Some(node) }
    }
}
impl Iterator for Ancestors {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
            self.node = n.parent();
            Some(n)
        } else {
            None
        }
    }
}

mod internal {
    pub trait InternalTreeIterator {
        fn skip_all(&mut self, widget: &super::WidgetInfo);
    }
}

/// Iterator that traverses the branches of a widget tree.
pub trait TreeIterator: internal::InternalTreeIterator + Iterator<Item = WidgetInfo> + FusedIterator {
    /// Creates an iterator which uses a closure to filter items or branches at a time.
    ///
    /// See [`TreeFilter`] for details.
    fn tree_filter<F>(self, filter: F) -> TreeFilterIter<Self, F>
    where
        Self: Sized,
        F: FnMut(&WidgetInfo) -> TreeFilter,
    {
        TreeFilterIter { iter: self, filter }
    }

    /// Gets the first item not filtered out by a [`TreeFilter`] closure.
    fn tree_find<F>(self, filter: F) -> Option<WidgetInfo>
    where
        Self: Sized,
        F: FnMut(&WidgetInfo) -> TreeFilter,
    {
        self.tree_filter(filter).next()
    }

    /// Check if any item is not filtered out by a [`TreeFilter`] closure.
    fn tree_any<F>(self, filter: F) -> bool
    where
        Self: Sized,
        F: FnMut(&WidgetInfo) -> TreeFilter,
    {
        self.tree_find(filter).is_some()
    }
}

/// Primary implementer of [`TreeIterator`].
pub struct TreeIter {
    tree: WidgetInfoTree,
    iter: tree::TreeIter,
}
impl TreeIter {
    pub(super) fn self_and_descendants(wgt: WidgetInfo) -> Self {
        Self {
            tree: wgt.tree().clone(),
            iter: wgt.node().self_and_descendants(),
        }
    }

    pub(super) fn self_and_prev_siblings_in(wgt: WidgetInfo, ancestor: WidgetInfo) -> RevTreeIter {
        let tree = &wgt.tree.0.tree;
        let mut iter = ancestor.node().self_and_descendants().rev(tree);
        iter.skip_to(tree, wgt.node_id);

        RevTreeIter { tree: wgt.tree, iter }
    }
    pub(super) fn prev_siblings_in(wgt: WidgetInfo, ancestor: WidgetInfo) -> RevTreeIter {
        if let Some(wgt) = wgt.prev_sibling() {
            return Self::self_and_prev_siblings_in(wgt, ancestor);
        } else if let Some(parent) = wgt.parent() {
            if parent != ancestor && wgt.tree == ancestor.tree {
                return Self::prev_siblings_in(parent, ancestor);
            }
        }
        RevTreeIter {
            tree: wgt.tree,
            iter: tree::RevTreeIter::empty(),
        }
    }

    pub(super) fn self_and_next_siblings_in(wgt: WidgetInfo, ancestor: WidgetInfo) -> Self {
        if wgt.tree != ancestor.tree {
            return TreeIter {
                tree: wgt.tree,
                iter: tree::TreeIter::empty(),
            };
        }

        let mut iter = ancestor.node().self_and_descendants();
        iter.skip_to(wgt.node_id);
        Self {
            tree: wgt.tree().clone(),
            iter,
        }
    }
    pub(super) fn next_siblings_in(wgt: WidgetInfo, ancestor: WidgetInfo) -> Self {
        if let Some(wgt) = wgt.next_sibling() {
            return Self::self_and_next_siblings_in(wgt, ancestor);
        } else if let Some(parent) = wgt.parent() {
            if parent != ancestor && wgt.tree == ancestor.tree {
                return Self::next_siblings_in(parent, ancestor);
            }
        }
        TreeIter {
            tree: wgt.tree,
            iter: tree::TreeIter::empty(),
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
    pub fn tree_rev(self) -> RevTreeIter
    where
        Self: Sized,
    {
        RevTreeIter {
            iter: self.iter.rev(&self.tree.0.tree),
            tree: self.tree,
        }
    }
}
impl internal::InternalTreeIterator for TreeIter {
    fn skip_all(&mut self, widget: &WidgetInfo) {
        self.iter.close(&self.tree.0.tree, widget.node_id)
    }
}
impl Iterator for TreeIter {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| WidgetInfo::new(self.tree.clone(), id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }
}
impl ExactSizeIterator for TreeIter {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl FusedIterator for TreeIter {}
impl TreeIterator for TreeIter {}

/// Reversing tree iterator.
///
/// This struct is created by the [`TreeIter::tree_rev`] method.
pub struct RevTreeIter {
    tree: WidgetInfoTree,
    iter: tree::RevTreeIter,
}
impl internal::InternalTreeIterator for RevTreeIter {
    fn skip_all(&mut self, widget: &WidgetInfo) {
        self.iter.close(&self.tree.0.tree, widget.node_id);
    }
}
impl Iterator for RevTreeIter {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next(&self.tree.0.tree).map(|id| WidgetInfo::new(self.tree.clone(), id))
    }
}
impl FusedIterator for RevTreeIter {}
impl TreeIterator for RevTreeIter {}

/// Filtering tree iterator.
///
/// This struct is created by the [`TreeIterator::tree_filter`] method.
pub struct TreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> TreeFilter,
{
    iter: I,
    filter: F,
}
impl<I, F> internal::InternalTreeIterator for TreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> TreeFilter,
{
    fn skip_all(&mut self, widget: &WidgetInfo) {
        self.iter.skip_all(widget)
    }
}
impl<I, F> Iterator for TreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> TreeFilter,
{
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(wgt) => match (self.filter)(&wgt) {
                    TreeFilter::Include => return Some(wgt),
                    TreeFilter::Skip => continue,
                    TreeFilter::SkipAll => {
                        self.iter.skip_all(&wgt);
                        continue;
                    }
                    TreeFilter::SkipDescendants => {
                        self.iter.skip_all(&wgt);
                        return Some(wgt);
                    }
                },
                None => return None,
            }
        }
    }
}
impl<I, F> FusedIterator for TreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> TreeFilter,
{
}
impl<I, F> TreeIterator for TreeFilterIter<I, F>
where
    I: TreeIterator,
    F: FnMut(&WidgetInfo) -> TreeFilter,
{
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use zng_layout::unit::FactorUnits;

    use crate::{
        APP,
        widget::{
            WIDGET, WidgetCtx, WidgetId, WidgetUpdateMode,
            info::{
                TreeFilter, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder, WidgetInfoTree, access::AccessEnabled,
                iter::TreeIterator,
            },
        },
        window::{WINDOW, WindowId},
    };

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
            WINDOW.with_test_context(WidgetUpdateMode::Ignore, || {
                WIDGET.with_context(&mut WidgetCtx::new(WidgetId::named(name)), WidgetUpdateMode::Ignore, || {
                    self.push_widget(inner)
                });
            });
        }
    }

    trait WidgetInfoExt {
        fn test_name(self) -> &'static str;
    }
    impl WidgetInfoExt for WidgetInfo {
        fn test_name(self) -> &'static str {
            self.id().name().as_static_str().expect("use with `push_test_widget` only")
        }
    }

    fn data() -> WidgetInfoTree {
        let _scope = APP.minimal();
        let mut builder = WidgetInfoBuilder::new(
            Arc::default(),
            WindowId::named("w"),
            AccessEnabled::empty(),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
        );
        builder.push_test_widget("c-0", |_| {});
        builder.push_test_widget("c-1", |_| {});
        builder.push_test_widget("c-2", |_| {});
        builder.finalize(None, false)
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
        let _scope = APP.minimal();
        let mut builder = WidgetInfoBuilder::new(
            Arc::default(),
            WindowId::named("w"),
            AccessEnabled::empty(),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
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
        builder.finalize(None, false)
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
            vec![
                "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1", "c-0-0",
            ]
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
            vec![
                "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1", "c-0-0"
            ]
        );
    }

    fn data_deep() -> WidgetInfoTree {
        let _scope = APP.minimal();
        let mut builder = WidgetInfoBuilder::new(
            Arc::default(),
            WindowId::named("w"),
            AccessEnabled::empty(),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
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
        builder.finalize(None, false)
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
                if w.id() == WidgetId::named("c-1") {
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
                if w.id() == WidgetId::named("c-1") {
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
                if w.id() == WidgetId::named("c-1") {
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
                if w.id() == WidgetId::named("c-1") {
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
                if w.id() == WidgetId::named("c-1") {
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
                if w.id() == WidgetId::named("c-1") {
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

        let result: Vec<_> = item.self_and_next_siblings_in(&root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_prev_siblings_in_problem_case() {
        let tree = data_nested();

        let root = tree.get("c-1").unwrap();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(&root).map(|w| w.test_name()).collect();
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

        let result: Vec<_> = item.self_and_next_siblings_in(&root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_prev_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(&root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .tree_rev()
            .skip_while(|w| w.id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn next_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.get("c-1-1-0").unwrap();

        let result: Vec<_> = item.next_siblings_in(&root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.id() != WidgetId::named("c-1-1-0"))
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

        let result: Vec<_> = item.prev_siblings_in(&root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .tree_rev()
            .skip_while(|w| w.id() != WidgetId::named("c-1-1-0"))
            .skip(1)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }
}
