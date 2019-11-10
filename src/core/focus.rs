#![allow(dead_code)]

use super::{ChildValueKey, ChildValueKeyRef, LayoutPoint, LayoutRect, LayoutSize};
use ego_tree::{NodeId, Tree, NodeRef};

uid! {
    /// Focusable unique identifier.
    pub struct FocusKey(_);
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

    /// Move focus into the menu scope. TODO
    EnterAlt,
    /// Move focus to parent focus scope.
    EscapeAlt,

    /// Move focus to the left of current.
    Left,
    /// Move focus to the right of current.
    Right,
    /// Move focus above current.
    Up,
    /// Move focus bellow current.
    Down,
}

#[derive(new)]
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

struct FocusScopeData {
    menu: bool,
    tab: Option<TabNav>,
    directional: Option<DirectionalNav>,
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

struct FocusEntry {
    key: FocusKey,
    origin: LayoutPoint,
    scope: Option<Box<FocusScopeData>>,
}

pub(crate) struct FocusMap {
    offset: LayoutPoint,
    current_scope: NodeId,
    entries: Tree<FocusEntry>,
}

impl FocusMap {
    pub fn new(
        window_scope_key: FocusKey,
        window_scope_rect: &LayoutRect,
        window_scope_tab: Option<TabNav>,
        window_scope_directional: Option<DirectionalNav>,
    ) -> Self {
        let entries = Tree::new(FocusEntry {
            key: window_scope_key,
            origin: window_scope_rect.center(),
            scope: Some(Box::new(FocusScopeData {
                menu: false,
                tab: window_scope_tab,
                directional: window_scope_directional,
                size: window_scope_rect.size,
            })),
        });

        FocusMap {
            offset: window_scope_rect.origin,
            current_scope: entries.root().id(),
            entries,
        }
    }

    pub fn push_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset += final_rect.origin.to_vector();
    }

    pub fn pop_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset -= final_rect.origin.to_vector();
    }

    pub fn push_focus_scope(
        &mut self,
        key: FocusKey,
        rect: &LayoutRect,
        menu: bool,
        tab: Option<TabNav>,
        directional: Option<DirectionalNav>,
    ) {
        self.current_scope = self
            .entries
            .get_mut(self.current_scope)
            .unwrap()
            .append(FocusEntry {
                key,
                origin: rect.center() + self.offset.to_vector(),
                scope: Some(Box::new(FocusScopeData {
                    menu,
                    tab,
                    directional,
                    size: rect.size,
                })),
            })
            .id();

        self.push_reference_frame(rect);
    }

    pub fn pop_focus_scope(&mut self, rect: &LayoutRect) {
        self.current_scope = self.entries.get(self.current_scope).unwrap().parent().unwrap().id();
        self.pop_reference_frame(rect);
    }

    pub fn push_focusable(&mut self, key: FocusKey, origin: LayoutPoint) {
        self.entries.get_mut(self.current_scope).unwrap().append(FocusEntry {
            key,
            origin: origin + self.offset.to_vector(),
            scope: None,
        });
    }

    /// Gets next focus key  from a current `focused` and a change `request`.
    pub fn focus(&self, focused: Option<FocusKey>, request: FocusRequest) -> Option<FocusKey> {
        match (request, focused) {
            (FocusRequest::Direct(direct_key), current) => {
                if self.contains(direct_key) {
                    Some(direct_key)
                } else {
                    current
                }
            },
            (_, None) => Some(self.entries.root().value().key),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(key)) => Some(self.next(key)),
            (FocusRequest::Prev, Some(key)) => Some(self.prev(key)),
            // Alt - Esc
            (FocusRequest::EnterAlt, Some(_key)) => unimplemented!(),
            (FocusRequest::EscapeAlt, Some(_key)) => unimplemented!(),
            //Arrow Keys
            (direction, Some(key)) => Some(self.next_towards(direction, key)),
        }
    }

    fn find_node(&self, key: FocusKey) -> Option<NodeRef<FocusEntry>> {
        self.entries.root().descendants().find(|n|n.value().key == key)
    }

    fn id_from_key(&self, key: FocusKey) -> Option<NodeId> {
        self.find_node(key).map(|n|n.id())
    }

    fn contains(&self, key: FocusKey) -> bool {
        self.entries.root().descendants().any(|n|n.value().key == key)
    }

    fn next(&self, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();

        if let Some(scope) = &node.value().scope {
            if let Some(first_child) = node.first_child() {
                return first_child.value().key;
            } else if scope.retains_tab() {
                return current;
            }
        }

        if let Some(next_same_scope) = node.next_sibling() {
            return next_same_scope.value().key;
        }

        self.next_scoped(current, node)
    }

    fn next_scoped(&self, current: FocusKey, node: NodeRef<FocusEntry>) -> FocusKey {
        if let Some(parent_scope) = node.parent() {
            match parent_scope.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Cycle) => return parent_scope.first_child().unwrap().value().key,
                Some(TabNav::Contained) => return current,
                Some(TabNav::Continue) => if let Some(next) = parent_scope.next_sibling() {
                    return next.value().key;
                } else {
                   return self.next_scoped(current, parent_scope);
                },
                Some(TabNav::Once) => unimplemented!(),
                None => unimplemented!()
            }
        }

        current
    }

    fn prev(&self, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();

        if let Some(scope) = &node.value().scope {
            if let Some(first_child) = node.last_child() {
                return first_child.value().key;
            } else if scope.retains_tab() {
                return current;
            }
        }

        if let Some(prev_same_scope) = node.prev_sibling() {
            return prev_same_scope.value().key;
        }

       self.prev_scoped(current, node)
    }

    fn prev_scoped(&self, current: FocusKey, node: NodeRef<FocusEntry>) -> FocusKey {
        if let Some(parent_scope) = node.parent() {
            match parent_scope.value().scope.as_ref().unwrap().tab {
                Some(TabNav::Cycle) => return parent_scope.last_child().unwrap().value().key,
                Some(TabNav::Contained) => return current,
                Some(TabNav::Continue) => if let Some(prev) = parent_scope.prev_sibling() {
                    return prev.value().key;
                } else {
                   return self.prev_scoped(current, parent_scope);
                },
                Some(TabNav::Once) => unimplemented!(),
                None => unimplemented!()
            }
        }

        current
    }

    /// Returns vector of distance from `origin`, item, item parent scope.
    fn candidates_towards(&self, direction: FocusRequest, origin: LayoutPoint) -> Vec<(f32, FocusKey, NodeId)> {
        let mut candidates: Vec<_> = self
            .entries
            .root()
            .descendants()
            .filter(move |c| is_in_direction(direction, origin, c.value().origin))
            .map(|c| {
                let o = c.value().origin;
                let a = (o.x - origin.x).powf(2.);
                let b = (o.y - origin.y).powf(2.);
                (a + b, c.value().key, c.parent().unwrap().id())
            })
            .collect();

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        candidates
    }

    fn next_towards(&self, direction: FocusRequest, current: FocusKey) -> FocusKey {
        let node = self.find_node(current).unwrap();
        let candidates = self.candidates_towards(direction, node.value().origin);
        let parent = node.parent().unwrap();

        if let Some(candidate) = candidates.first(){
            if candidate.2 == parent.id(){
                return candidate.1;
            }
        }
        match parent.value().scope.as_ref().unwrap().directional {
            // contained retention does not change focus, already is last in direction inside scope.
            Some(DirectionalNav::Contained) => return current,
            // cycling retention, finds closest to new origin that is
            // in the same line or column of current focus but on the other side
            // of the parent scope rectangle.
            Some(DirectionalNav::Cycle) => {
                let mut origin = node.value().origin;
                let scope_origin = parent.value().origin;
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

                let candidates = self.candidates_towards(direction, origin);    
                if let Some(c) = candidates.iter().find(|c| c.2 == parent.id()) {
                    // if can find candidate on other side.
                    return c.1;
                } else {
                    // else do the same as contained.
                    // probably a bug, should have found the current focus again at least.
                    return current;
                }
            }
            Some(DirectionalNav::Continue) =>{
                if let Some(candidate) = candidates.first(){
                    return candidate.1;
                }
            }
            _ => unimplemented!()

        }

        current
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
