//! Widget info tree iterators.

use ego_tree::NodeRef;

use super::*;

/// Widget tree filter result.
///
/// This `enum` is used by the [`filter_descendants`] and [`filter_self_and_descendants`] methods on [`WidgetInfo`]. See its documentation for more.
///
/// [`filter_descendants`]: WidgetInfo::filter_descendants
/// [`filter_self_and_descendants`]: WidgetInfo::filter_self_and_descendants
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum DescendantFilter {
    /// Include the descendant and continue filtering its descendants.
    Include,
    /// Skip the descendant but continue filtering its descendants.
    Skip,
    /// Skip the descendant and its descendants.
    SkipAll,
    /// Include the descendant but skips its descendants.
    SkipDescendants,
}

/// Iterator over all items in a branch of the widget tree.
///
/// This `struct` is created by the [`descendants`] and [`self_and_descendants`] methods on [`WidgetInfo`]. See its documentation for more.
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
}
#[derive(Clone, Copy)]
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
        }
    }

    pub(super) fn new_in(tree: &'a WidgetInfoTree, root: NodeRef<'a, WidgetInfoData>, item: NodeRef<'a, WidgetInfoData>) -> Self {
        Self {
            tree,
            root,
            front: item,
            front_state: DescendantsState::Enter,
            back: item,
            back_state: DescendantsState::Enter,
        }
    }

    /// Filter out entire branches of descendants at a time.
    pub fn filter<F>(self, filter: F) -> FilterDescendants<'a, F>
    where
        F: FnMut(WidgetInfo<'a>) -> DescendantFilter,
    {
        FilterDescendants {
            filter,
            tree: self.tree,
            root: self.root,

            front: self.front,
            front_state: match self.front_state {
                DescendantsState::Enter => FilterDescendantsState::Filter,
                DescendantsState::Exit => FilterDescendantsState::Exit,
            },
            back: self.back,
            back_state: match self.back_state {
                DescendantsState::Enter => FilterDescendantsState::Filter,
                DescendantsState::Exit => FilterDescendantsState::Exit,
            },
        }
    }
}
impl<'a> Iterator for Descendants<'a> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let (DescendantsState::Exit, DescendantsState::Exit) = (self.front_state, self.back_state) {
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
}
impl<'a> DoubleEndedIterator for Descendants<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let (DescendantsState::Exit, DescendantsState::Exit) = (self.front_state, self.back_state) {
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

/// An iterator that filters a widget tree.
///
/// This `struct` is created by the [`Descendants::filter`] method. See its documentation for more.
pub struct FilterDescendants<'a, F: FnMut(WidgetInfo<'a>) -> DescendantFilter> {
    filter: F,
    tree: &'a WidgetInfoTree,

    root: NodeRef<'a, WidgetInfoData>,

    front: NodeRef<'a, WidgetInfoData>,
    front_state: FilterDescendantsState,

    back: NodeRef<'a, WidgetInfoData>,
    back_state: FilterDescendantsState,
}
#[derive(Clone, Copy)]
enum FilterDescendantsState {
    Filter,
    Enter,
    Exit,
}

impl<'a, F> Iterator for FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> DescendantFilter,
{
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let (FilterDescendantsState::Exit, FilterDescendantsState::Exit) = (self.front_state, self.back_state) {
                    return None;
                }
            }

            match self.front_state {
                FilterDescendantsState::Filter => {
                    let wgt = WidgetInfo::new(self.tree, self.front.id());

                    match (self.filter)(wgt) {
                        DescendantFilter::Include => {
                            self.front_state = FilterDescendantsState::Enter;
                            return Some(wgt);
                        }
                        DescendantFilter::Skip => {
                            self.front_state = FilterDescendantsState::Enter;
                            continue;
                        }
                        DescendantFilter::SkipAll => {
                            self.front_state = FilterDescendantsState::Exit;
                            continue;
                        }
                        DescendantFilter::SkipDescendants => {
                            self.front_state = FilterDescendantsState::Exit;
                            return Some(wgt);
                        }
                    }
                }
                FilterDescendantsState::Enter => {
                    if let Some(child) = self.front.first_child() {
                        self.front = child;
                        self.front_state = FilterDescendantsState::Filter;
                        continue;
                    } else {
                        self.front_state = FilterDescendantsState::Exit;
                        continue;
                    }
                }
                FilterDescendantsState::Exit => {
                    if self.front == self.root {
                        return None;
                    } else if let Some(s) = self.front.next_sibling() {
                        self.front = s;
                        self.front_state = FilterDescendantsState::Filter;
                        continue;
                    } else if let Some(p) = self.front.parent() {
                        self.front = p;
                        self.front_state = FilterDescendantsState::Exit;
                        continue;
                    } else {
                        // did not find our root, but found a tree root?
                        unreachable!()
                    }
                }
            }
        }
    }
}
impl<'a, F> DoubleEndedIterator for FilterDescendants<'a, F>
where
    F: FnMut(WidgetInfo<'a>) -> DescendantFilter,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            // DoubleEndedIterator contract
            if self.front == self.back {
                if let (FilterDescendantsState::Exit, FilterDescendantsState::Exit) = (self.front_state, self.back_state) {
                    return None;
                }
            }

            match self.front_state {
                FilterDescendantsState::Filter => {
                    let wgt = WidgetInfo::new(self.tree, self.front.id());

                    match (self.filter)(wgt) {
                        DescendantFilter::Include => {
                            self.front_state = FilterDescendantsState::Enter;
                            return Some(wgt);
                        }
                        DescendantFilter::Skip => {
                            self.front_state = FilterDescendantsState::Enter;
                            continue;
                        }
                        DescendantFilter::SkipAll => {
                            self.front_state = FilterDescendantsState::Exit;
                            continue;
                        }
                        DescendantFilter::SkipDescendants => {
                            self.front_state = FilterDescendantsState::Exit;
                            return Some(wgt);
                        }
                    }
                }
                FilterDescendantsState::Enter => {
                    if let Some(child) = self.front.last_child() {
                        self.front = child;
                        self.front_state = FilterDescendantsState::Filter;
                        continue;
                    } else {
                        self.front_state = FilterDescendantsState::Exit;
                        continue;
                    }
                }
                FilterDescendantsState::Exit => {
                    if self.front == self.root {
                        return None;
                    } else if let Some(s) = self.front.prev_sibling() {
                        self.front = s;
                        self.front_state = FilterDescendantsState::Filter;
                        continue;
                    } else if let Some(p) = self.front.parent() {
                        self.front = p;
                        self.front_state = FilterDescendantsState::Exit;
                        continue;
                    } else {
                        // did not find our root, but found a tree root?
                        unreachable!()
                    }
                }
            }
        }
    }
}
