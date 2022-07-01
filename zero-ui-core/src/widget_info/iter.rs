//! Widget info tree iterators.

use ego_tree::NodeRef;

use super::*;

/// Widget tree filter result.
///
/// This `enum` is used by the [`Descendants::filter`] method.
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

/// Iterator over all items in a branch of the widget tree.
///
/// This `struct` is created by the [`descendants`] and [`self_and_descendants`] methods on [`WidgetInfo`].
///
/// [`descendants`]: WidgetInfo::descendants
/// [`self_and_descendants`]: WidgetInfo::self_and_descendants
pub struct Descendants<'a> {
    tree: &'a WidgetInfoTree,
    root: NodeRef<'a, WidgetInfoData>,

    front: NodeRef<'a, WidgetInfoData>,
    front_state: DescendantsState,

    back: NodeRef<'a, WidgetInfoData>,
    back_state: DescendantsState,

    next_is_prev: bool,
}
impl<'a> fmt::Debug for Descendants<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Descendants")
            .field("root", &self.root.value().widget_id.to_string())
            .field("front", &self.front.value().widget_id.to_string())
            .field("front_state", &self.front_state)
            .field("back", &self.back.value().widget_id.to_string())
            .field("back_state", &self.back_state)
            .field("next_is_prev", &self.next_is_prev)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy)]
enum DescendantsState {
    Enter,
    Exit,
}
impl<'a> Descendants<'a> {
    pub(super) fn new(tree: &'a WidgetInfoTree, root: NodeRef<'a, WidgetInfoData>) -> Self {
        Self {
            tree,
            root,
            front: root,
            front_state: DescendantsState::Enter,
            back: root,
            back_state: DescendantsState::Enter,
            next_is_prev: false,
        }
    }

    pub(super) fn new_in(
        tree: &'a WidgetInfoTree,
        root: NodeRef<'a, WidgetInfoData>,
        item: NodeRef<'a, WidgetInfoData>,
        is_prev: bool,
    ) -> Self {
        if is_prev {
            Self {
                tree,
                root,
                front: root,
                front_state: DescendantsState::Enter,
                back: item,
                back_state: DescendantsState::Enter,
                next_is_prev: true,
            }
        } else {
            Self {
                tree,
                root,
                front: item,
                front_state: DescendantsState::Enter,
                back: root,
                back_state: DescendantsState::Enter,
                next_is_prev: false,
            }
        }
    }

    pub(super) fn new_in_after(
        tree: &'a WidgetInfoTree,
        root: NodeRef<'a, WidgetInfoData>,
        item: NodeRef<'a, WidgetInfoData>,
        is_prev: bool,
    ) -> Self {
        let mut r = Self::new_in(tree, root, item, is_prev);
        if let Some(n) = r.front.next_sibling() {
            r.front = n;
        } else {
            r.front_state = DescendantsState::Exit;
        }
        if let Some(n) = r.back.prev_sibling() {
            r.back = n;
        } else {
            r.back_state = DescendantsState::Exit;
        }
        r
    }

    /// Filter out entire branches of descendants at a time.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    pub fn filter<F>(self, filter: F) -> FilterDescendants<'a, F>
    where
        F: FnMut(WidgetInfo<'a>) -> TreeFilter,
    {
        FilterDescendants { filter, iter: self }
    }

    /// Returns the first widget included by `filter`.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    pub fn find<F>(self, filter: F) -> Option<WidgetInfo<'a>>
    where
        F: FnMut(WidgetInfo<'a>) -> TreeFilter,
    {
        #[allow(clippy::filter_next)]
        self.filter(filter).next()
    }

    /// Returns if the `filter` allows any widget.
    ///
    /// Note that you can convert `bool` into [`TreeFilter`] to use this method just like the iterator default.
    pub fn any<F>(self, filter: F) -> bool
    where
        F: FnMut(WidgetInfo<'a>) -> TreeFilter,
    {
        self.find(filter).is_some()
    }

    fn actual_next(&mut self) -> Option<WidgetInfo<'a>> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let DescendantsState::Exit = self.front_state {
                    return None;
                }
                if let DescendantsState::Exit = self.back_state {
                    return None;
                }
            }

            match self.front_state {
                DescendantsState::Enter => {
                    let next = Some(WidgetInfo::new(self.tree, self.front.id()));

                    if let Some(child) = self.front.first_child() {
                        self.front = child;
                        self.front_state = DescendantsState::Enter;
                    } else {
                        self.front_state = DescendantsState::Exit;
                    }

                    return next;
                }
                DescendantsState::Exit => {
                    if self.front == self.root {
                        return None;
                    } else if let Some(s) = self.front.next_sibling() {
                        self.front = s;
                        self.front_state = DescendantsState::Enter;
                        continue;
                    } else if let Some(p) = self.front.parent() {
                        self.front = p;
                        self.front_state = DescendantsState::Exit;
                        continue;
                    } else {
                        self.front = self.root;
                        return None;
                    }
                }
            }
        }
    }

    fn actual_next_back(&mut self) -> Option<WidgetInfo<'a>> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let DescendantsState::Exit = self.front_state {
                    return None;
                }
                if let DescendantsState::Exit = self.back_state {
                    return None;
                }
            }

            match self.back_state {
                DescendantsState::Enter => {
                    let next = Some(WidgetInfo::new(self.tree, self.back.id()));

                    if let Some(child) = self.back.last_child() {
                        self.back = child;
                        self.back_state = DescendantsState::Enter;
                    } else {
                        self.back_state = DescendantsState::Exit;
                    }

                    return next;
                }
                DescendantsState::Exit => {
                    if self.back == self.root {
                        return None;
                    } else if let Some(s) = self.back.prev_sibling() {
                        self.back = s;
                        self.back_state = DescendantsState::Enter;
                        continue;
                    } else if let Some(p) = self.back.parent() {
                        self.back = p;
                        self.back_state = DescendantsState::Exit;
                        continue;
                    } else {
                        self.back = self.root;
                        return None;
                    }
                }
            }
        }
    }
}
impl<'a> Iterator for Descendants<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_is_prev {
            self.actual_next_back()
        } else {
            self.actual_next()
        }
    }
}
impl<'a> DoubleEndedIterator for Descendants<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.next_is_prev {
            self.actual_next()
        } else {
            self.actual_next_back()
        }
    }
}

/// An iterator that filters a widget tree.
///
/// This `struct` is created by the [`Descendants::filter`] method.
pub struct FilterDescendants<'a, F: FnMut(WidgetInfo<'a>) -> TreeFilter> {
    filter: F,
    iter: Descendants<'a>,
}
impl<'a, F> fmt::Debug for FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterDescendants")
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}
impl<'a, F> FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
    fn advance(&mut self, pull: impl Fn(&mut Descendants<'a>) -> Option<WidgetInfo<'a>>, is_front: bool) -> Option<WidgetInfo<'a>> {
        loop {
            if let Some(wgt) = pull(&mut self.iter) {
                let mut skip = || {
                    if is_front {
                        self.iter.front = wgt.node();
                        self.iter.front_state = DescendantsState::Exit;
                    } else {
                        self.iter.back = wgt.node();
                        self.iter.back_state = DescendantsState::Exit;
                    }
                };

                match (self.filter)(wgt) {
                    TreeFilter::Include => return Some(wgt),
                    TreeFilter::Skip => continue,
                    TreeFilter::SkipAll => {
                        skip();
                        continue;
                    }
                    TreeFilter::SkipDescendants => {
                        skip();
                        return Some(wgt);
                    }
                }
            } else {
                return None;
            }
        }
    }
}
impl<'a, F> Iterator for FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance(|d| d.next(), !self.iter.next_is_prev)
    }
}
impl<'a, F> DoubleEndedIterator for FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> TreeFilter,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.advance(|d| d.next_back(), self.iter.next_is_prev)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        widget_info::{TreeFilter, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder, WidgetInfoTree},
        window::WindowId,
        WidgetId,
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
            .filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["c-0", "c-1", "c-2"]);
    }

    #[test]
    fn descendants_rev() {
        let tree = data();

        let result: Vec<_> = tree.root().descendants().rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-2", "c-1", "c-0"]);
    }

    #[test]
    fn descendants_filter_noop_rev() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .descendants()
            .filter(|_| TreeFilter::Include)
            .rev()
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
            .filter(|_| TreeFilter::Include)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["w", "c-0", "c-1", "c-2"]);
    }

    #[test]
    fn self_and_descendants_rev() {
        let tree = data();

        let result: Vec<_> = tree.root().self_and_descendants().rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["w", "c-2", "c-1", "c-0",]);
    }

    #[test]
    fn self_and_descendants_filter_noop_rev() {
        let tree = data();

        let result: Vec<_> = tree
            .root()
            .self_and_descendants()
            .filter(|_| TreeFilter::Include)
            .rev()
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, vec!["w", "c-2", "c-1", "c-0",]);
    }

    #[test]
    fn descendants_double() {
        let tree = data();
        let mut iter = tree.root().descendants();

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));

        let result: Vec<_> = iter.rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-2", "c-1"]);
    }

    #[test]
    fn descendants_double_filter_noop() {
        let tree = data();
        let mut iter = tree.root().descendants().filter(|_| TreeFilter::Include);

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));

        let result: Vec<_> = iter.rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["c-2", "c-1"]);
    }

    fn data_nested() -> WidgetInfoTree {
        let mut builder = WidgetInfoBuilder::new(
            WindowId::named("w"),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
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

        let result: Vec<_> = tree.root().descendants().rev().map(|w| w.test_name()).collect();

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

        let result: Vec<_> = tree.root().self_and_descendants().rev().map(|w| w.test_name()).collect();
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

        let result: Vec<_> = iter.rev().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec![
                "c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1",
                "c-0-0",
            ]
        );
    }

    #[test]
    fn descendants_double_nested() {
        let tree = data_nested();
        let mut iter = tree.root().descendants();

        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0"));
        assert_eq!(iter.next().map(|w| w.test_name()), Some("c-0-0"));

        let result: Vec<_> = iter.rev().map(|w| w.test_name()).collect();

        assert_eq!(
            result,
            vec!["c-2", "c-2-2", "c-2-2-0", "c-2-1", "c-2-0", "c-1", "c-1-1", "c-1-1-1", "c-1-1-0", "c-1-0", "c-0", "c-0-2", "c-0-1",]
        );
    }

    fn data_deep() -> WidgetInfoTree {
        let mut builder = WidgetInfoBuilder::new(
            WindowId::named("w"),
            WidgetId::named("w"),
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
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
        let result: Vec<_> = tree.root().descendants().rev().map(|w| w.test_name()).collect();

        assert_eq!(result, vec!["d-0", "d-1", "d-2", "d-3", "d-4", "d-5"])
    }

    #[test]
    fn descendants_deep_double() {
        let tree = data_deep();

        let mut iter = tree.root().descendants().rev().map(|w| w.test_name());
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
            .filter(|_| TreeFilter::Include)
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
            .filter(|w| {
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
            .filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::Skip
                } else {
                    TreeFilter::Include
                }
            })
            .rev()
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
            .filter(|w| {
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
            .filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipAll
                } else {
                    TreeFilter::Include
                }
            })
            .rev()
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
            .filter(|w| {
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
            .filter(|w| {
                if w.widget_id() == WidgetId::named("c-1") {
                    TreeFilter::SkipDescendants
                } else {
                    TreeFilter::Include
                }
            })
            .rev()
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

        let root = tree.find("c-1").unwrap();
        let item = tree.find("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_next_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_prev_siblings_in() {
        let tree = data_nested();

        let root = tree.find("c-1").unwrap();
        let item = tree.find("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .rev()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn self_and_next_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.find("c-1-1").unwrap();

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
        let item = tree.find("c-1-1").unwrap();

        let result: Vec<_> = item.self_and_prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .rev()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1"))
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn next_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.find("c-1-1-0").unwrap();

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
    fn and_prev_siblings_in_root() {
        let tree = data_nested();

        let root = tree.root();
        let item = tree.find("c-1-1-0").unwrap();

        let result: Vec<_> = item.prev_siblings_in(root).map(|w| w.test_name()).collect();
        let expected: Vec<_> = root
            .descendants()
            .rev()
            .skip_while(|w| w.widget_id() != WidgetId::named("c-1-1-0"))
            .skip(1)
            .map(|w| w.test_name())
            .collect();

        assert_eq!(result, expected);
    }
}
