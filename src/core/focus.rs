#![allow(dead_code)]

use super::{ChildValueKey, ChildValueKeyRef, LayoutPoint, LayoutRect, LayoutSize};
use ego_tree::{NodeId, NodeRef, Tree};

uid! {
    /// Focusable unique identifier.
    pub struct FocusKey(_) { new_lazy() -> pub struct FocusKeyRef };
}

/// Custom focus navigation implementation must return this to stop
/// the default implementation on `keyboard_input`.
pub static FOCUS_HANDLED: ChildValueKeyRef<()> = ChildValueKey::new_lazy();

/// Focus change request.
#[derive(Clone, Copy, Debug)]
pub enum FocusRequest {
    /// Move focus to key.
    Direct(FocusKey),

    /// Move focus to next from current in screen, or to starting key.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,

    /// Move focus to the left of current.
    Left,
    /// Move focus to the right of current.
    Right,
    /// Move focus above current.
    Up,
    /// Move focus bellow current.
    Down,
}

#[derive(new, Debug)]
pub struct FocusChange {
    pub old_focus: Option<FocusKey>,
    pub new_focus: Option<FocusKey>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabNav {
    Continue,
    Contained,
    Cycle,
    Once,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DirectionalNav {
    Continue,
    Contained,
    Cycle,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FocusStatus {
    Focused,
    FocusWithin,
}

#[derive(Clone, new)]
pub struct FocusScopeData {
    pub skip: bool,
    pub tab: Option<TabNav>,
    pub directional: Option<DirectionalNav>,
    #[new(default)]
    size: LayoutSize,
}

impl FocusScopeData {
    fn retains_tab(&self) -> bool {
        match self.tab {
            Some(TabNav::Cycle) | Some(TabNav::Contained) => true,
            _ => false,
        }
    }

    fn retains_directional(&self) -> bool {
        match self.directional {
            Some(DirectionalNav::Cycle) | Some(DirectionalNav::Contained) => true,
            _ => false,
        }
    }
}

#[derive(Clone, new)]
pub struct FocusableData {
    pub tab_index: u32,
    pub key: FocusKey,
    #[new(default)]
    origin: LayoutPoint,
}

struct FocusEntry {
    f: FocusableData,
    scope: Option<Box<FocusScopeData>>,
}

pub(crate) struct FocusMap {
    offset: LayoutPoint,
    current_scope: NodeId,
    entries: Tree<FocusEntry>,
    len: usize,
}

impl FocusMap {
    pub fn new() -> Self {
        FocusMap::with_capacity(1)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        static EMPTY_KEY: FocusKeyRef = FocusKey::new_lazy();

        let entries = Tree::with_capacity(
            FocusEntry {
                f: FocusableData {
                    tab_index: 0,
                    key: *EMPTY_KEY,
                    origin: LayoutPoint::zero(),
                },
                scope: None,
            },
            capacity,
        );

        FocusMap {
            offset: LayoutPoint::zero(),
            current_scope: entries.root().id(),
            entries,
            len: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.root().value().scope.is_none()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset += final_rect.origin.to_vector();
    }

    pub fn pop_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset -= final_rect.origin.to_vector();
    }

    pub fn push_focus_scope(
        &mut self,
        rect: &LayoutRect,
        mut focusable_data: FocusableData,
        mut scope_data: FocusScopeData,
    ) {
        focusable_data.origin = rect.center() + self.offset.to_vector();
        scope_data.size = rect.size;

        let focus_entry = FocusEntry {
            f: focusable_data,
            scope: Some(Box::new(scope_data)),
        };

        if self.is_empty() {
            *self.entries.root_mut().value() = focus_entry;
        } else {
            self.current_scope = self.push_focus_entry(focus_entry);
            self.len += 1;
        }

        self.push_reference_frame(rect);
    }

    pub fn pop_focus_scope(&mut self, rect: &LayoutRect) {
        // if not root
        if let Some(parent) = self.entries.get(self.current_scope).unwrap().parent() {
            self.current_scope = parent.id();
            self.pop_reference_frame(rect);
        }
    }

    pub fn push_focusable(&mut self, rect: &LayoutRect, mut focusable_data: FocusableData) {
        focusable_data.origin = rect.center() + self.offset.to_vector();

        let focus_entry = FocusEntry {
            f: focusable_data,
            scope: None,
        };

        self.push_focus_entry(focus_entry);
        self.len += 1;
    }

    pub fn closest_existing(&self, current: FocusKey, new_map: &FocusMap) -> Option<FocusKey> {
        if new_map.contains(current) {
            Some(current)
        } else {
            let node = self
                .find_node(current)
                .expect("closest_existing must be called on the old map");

            node.next_siblings()
                .chain(node.prev_siblings())
                .chain(node.ancestors())
                .find(|n| new_map.contains(n.value().f.key))
                .map(|n| n.value().f.key)
        }
    }

    /// Gets next focus key  from a current `focused` and a change `request`.
    pub fn focus(&self, current: Option<FocusKey>, request: FocusRequest) -> Option<FocusKey> {
        if self.is_empty() {
            return None;
        }

        match (request, current) {
            (FocusRequest::Direct(direct_key), current) => {
                if self.contains(direct_key) {
                    Some(direct_key)
                } else {
                    current
                }
            }
            (_, None) => Some(self.entries.root().value().f.key),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(current)) => Some(self.next(current)),
            (FocusRequest::Prev, Some(current)) => Some(self.prev(current)),
            //Arrow Keys
            (direction, Some(current)) => Some(self.next_towards(direction, current)),
        }
    }

    fn push_focus_entry(&mut self, focus_entry: FocusEntry) -> NodeId {
        self.entries
            .get_mut(self.current_scope)
            .unwrap()
            .append(focus_entry)
            .id()
    }

    pub fn contains(&self, key: FocusKey) -> bool {
        self.entries.root().descendants().any(|n| n.value().f.key == key)
    }

    fn find_node(&self, key: FocusKey) -> Option<NodeRef<FocusEntry>> {
        self.entries.root().descendants().find(|n| n.value().f.key == key)
    }

    fn id_from_key(&self, key: FocusKey) -> Option<NodeId> {
        self.find_node(key).map(|n| n.id())
    }

    fn next(&self, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();
        self.node_next(current, node, false)
    }

    fn node_next(&self, current: FocusKey, node: NodeRef<FocusEntry>, from_scope: bool) -> FocusKey {
        if let (false, Some(scope)) = (from_scope, &node.value().scope) {
            if let Some(first_child) = node.first_tab_child() {
                return first_child.value().f.key;
            } else if scope.retains_tab() {
                return current;
            }
        } else if let Some(parent_node) = node.parent() {
            match parent_node.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Once) => return self.node_next(current, parent_node, true),
                None => return current,
                _ => {}
            }
        }

        if let Some(next_same_scope) = node.next_tab_sibling() {
            return next_same_scope.value().f.key;
        }

        self.scope_next(current, node)
    }

    ///Next, given the parent scope tab navigation, when the current is the last item in the scope
    fn scope_next(&self, current: FocusKey, node: NodeRef<FocusEntry>) -> FocusKey {
        if let Some(parent_node) = node.parent() {
            match parent_node.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Cycle) => parent_node.first_tab_child().unwrap().value().f.key,
                Some(TabNav::Contained) => current,
                Some(TabNav::Continue) => {
                    if let Some(next) = parent_node.next_tab_sibling() {
                        next.value().f.key
                    } else {
                        self.scope_next(current, parent_node)
                    }
                }
                Some(TabNav::Once) => self.node_next(current, parent_node, true),
                None => current,
            }
        } else {
            current
        }
    }

    fn prev(&self, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();
        self.node_prev(current, node, false)
    }

    fn node_prev(&self, current: FocusKey, node: NodeRef<FocusEntry>, from_scope: bool) -> FocusKey {
        if let (false, Some(scope)) = (from_scope, &node.value().scope) {
            if let Some(first_child) = node.last_tab_child() {
                return first_child.value().f.key;
            } else if scope.retains_tab() {
                return current;
            }
        } else if let Some(parent_node) = node.parent() {
            match parent_node.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Once) => return self.node_prev(current, parent_node, true),
                None => return current,
                _ => {}
            }
        }

        if let Some(prev_same_scope) = node.prev_tab_sibling() {
            return prev_same_scope.value().f.key;
        }

        self.scope_prev(current, node)
    }

    ///Previous, given the parent scope tab navigation, when the current is the first item in the scope
    fn scope_prev(&self, current: FocusKey, node: NodeRef<FocusEntry>) -> FocusKey {
        if let Some(parent_node) = node.parent() {
            match parent_node.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Cycle) => parent_node.last_tab_child().unwrap().value().f.key,
                Some(TabNav::Contained) => current,
                Some(TabNav::Continue) => {
                    if let Some(prev) = parent_node.prev_tab_sibling() {
                        prev.value().f.key
                    } else {
                        self.scope_prev(current, parent_node)
                    }
                }
                Some(TabNav::Once) => self.node_prev(current, parent_node, true),
                None => current,
            }
        } else {
            current
        }
    }

    fn next_towards(&self, direction: FocusRequest, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();
        let parent = node.parent().unwrap();
        let parent_nav = parent.value().scope.as_ref().unwrap().directional;

        if parent_nav.is_none() {
            return current;
        }

        let candidates = self.nodes_towards(parent, node.value().f.origin, direction);

        if let Some((_, closest_in_scope)) = candidates.first() {
            return closest_in_scope.value().f.key;
        }

        match parent_nav {
            // contained retention does not change focus, already is last in direction inside scope.
            Some(DirectionalNav::Contained) => return current,
            // cycling retention, finds closest to new origin that is
            // in the same line or column of current focus but on the other side
            // of the parent scope rectangle.
            Some(DirectionalNav::Cycle) => {
                let mut origin = node.value().f.origin;
                let scope_origin = parent.value().f.origin;
                let scope_size = parent.value().scope.as_ref().unwrap().size;
                match direction {
                    FocusRequest::Left => {
                        origin.x = scope_origin.x + scope_size.width / 2.;
                    }
                    FocusRequest::Right => {
                        origin.x = scope_origin.x - scope_size.width / 2.;
                    }
                    FocusRequest::Up => {
                        origin.y = scope_origin.y + scope_size.height / 2.;
                    }
                    FocusRequest::Down => {
                        origin.y = scope_origin.y - scope_size.height / 2.;
                    }
                    _ => unreachable!(),
                }

                let candidates = self.nodes_towards(parent, origin, direction);
                if let Some((_, c)) = candidates.first() {
                    // if can find candidate on other side.
                    return c.value().f.key;
                } else {
                    // else do the same as contained.
                    // probably a bug, should have found the current focus again at least.
                    return current;
                }
            }
            Some(DirectionalNav::Continue) => {
                if let Some(parent) = parent.parent() {
                    let candidates = self.nodes_towards(parent, node.value().f.origin, direction);
                    if let Some((_, c)) = candidates.first() {
                        return c.value().f.key;
                    }
                }
            }
            None => unreachable!(),
        }
        current
    }

    /// All content nodes in direction from origin inside a scope.
    fn nodes_towards<'a>(
        &self,
        scope: NodeRef<'a, FocusEntry>,
        origin: LayoutPoint,
        direction: FocusRequest,
    ) -> Vec<(f32, NodeRef<'a, FocusEntry>)> {
        let mut nodes: Vec<(f32, NodeRef<'a, FocusEntry>)> = scope
            .children()
            .filter(move |c| {
                if let Some(scope) = &c.value().scope {
                    if scope.skip {
                        return false;
                    }
                }
                is_in_direction(direction, origin, c.value().f.origin)
            })
            .map(|c| {
                let o = c.value().f.origin;
                let a = (o.x - origin.x).powf(2.);
                let b = (o.y - origin.y).powf(2.);
                (a + b, c)
            })
            .collect();

        nodes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        nodes
    }
}

trait NodeExt<'a> {
    fn next_tab_sibling(&self) -> Option<NodeRef<'a, FocusEntry>>;
    fn prev_tab_sibling(&self) -> Option<NodeRef<'a, FocusEntry>>;
    fn first_tab_child(&self) -> Option<NodeRef<'a, FocusEntry>>;
    fn last_tab_child(&self) -> Option<NodeRef<'a, FocusEntry>>;
    fn parent_scope(&self) -> Option<&FocusScopeData>;
}

impl<'a> NodeExt<'a> for NodeRef<'a, FocusEntry> {
    fn next_tab_sibling(&self) -> Option<Self> {
        if let Some(parent) = self.parent() {
            let self_tab_index = self.value().f.tab_index;

            // check for common case, next in render order with the same tab_index.
            if let Some(next) = self.next_sibling() {
                if next.value().f.tab_index == self_tab_index {
                    return Some(next);
                }
            }

            // did not find common case, search smallest tab_index greater then current.

            let mut found_self = false;
            let mut smallest_index = u32::max_value();
            let mut first_after = None;

            for c in parent.children() {
                let value = &c.value();

                if !found_self && c.id() == self.id() {
                    found_self = true;
                    continue;
                }

                // skips..
                if let Some(scope) = &value.scope {
                    if scope.skip {
                        // ..when marked to skip
                        continue;
                    }
                }
                if value.f.tab_index < self_tab_index || (!found_self && value.f.tab_index == self_tab_index) {
                    // ..when `c` is before current tab_index or is same tab_index, but before current
                    // in render position.
                    continue;
                }

                if value.f.tab_index == self_tab_index {
                    // found same tab_index after found_self.
                    return Some(c);
                }

                if value.f.tab_index < smallest_index {
                    smallest_index = value.f.tab_index;
                    first_after = Some(c);
                }
            }

            first_after
        } else {
            None
        }
    }

    fn prev_tab_sibling(&self) -> Option<Self> {
        let mut prev = self.prev_sibling();
        while let Some(n) = prev {
            if let Some(scope) = &n.value().scope {
                if scope.skip {
                    prev = n.prev_sibling();
                    continue;
                }
            }

            return prev;
        }
        None
    }

    fn first_tab_child(&self) -> Option<Self> {
        let mut smallest_index = u32::max_value();
        let mut first = None;

        for c in self.children() {
            let value = &c.value();

            if let Some(scope) = &value.scope {
                if scope.skip {
                    continue;
                }
            }

            if value.f.tab_index < smallest_index {
                smallest_index = value.f.tab_index;
                first = Some(c);
            } else if first.is_none() {
                first = Some(c);
            }
        }

        first
    }

    fn last_tab_child(&self) -> Option<Self> {
        let mut largest_index = 0;
        let mut last = None;

        for c in self.children().rev() {
            let value = &c.value();

            if let Some(scope) = &value.scope {
                if scope.skip {
                    continue;
                }
            }

            if value.f.tab_index > largest_index {
                largest_index = value.f.tab_index;
                last = Some(c);
            } else if last.is_none() {
                last = Some(c);
            }
        }

        last
    }

    fn parent_scope(&self) -> Option<&FocusScopeData> {
        self.parent().map(|node| node.value().scope.as_ref().unwrap().as_ref())
    }
}

fn is_in_direction(direction: FocusRequest, origin: LayoutPoint, candidate: LayoutPoint) -> bool {
    let (a, b, c, d) = match direction {
        FocusRequest::Left => (candidate.x, origin.x, candidate.y, origin.y),
        FocusRequest::Right => (origin.x, candidate.x, candidate.y, origin.y),
        FocusRequest::Up => (candidate.y, origin.y, candidate.x, origin.x),
        FocusRequest::Down => (origin.y, candidate.y, candidate.x, origin.x),
        _ => unreachable!(),
    };

    // checks if the candidate point is in between two imaginary perpendicular lines parting from the
    // origin point in the focus direction
    if a < b {
        if c >= d {
            return c <= d + (b - a);
        } else {
            return c >= d - (b - a);
        }
    }

    false
}
