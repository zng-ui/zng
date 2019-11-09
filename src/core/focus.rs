use super::{ChildValueKey, ChildValueKeyRef, LayoutPoint, LayoutRect, LayoutSize};

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

pub enum FocusState {
    NotFocused,
    NotActive,
    Active,
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

struct FocusScopeData {
    menu: bool,
    tab: Option<TabNav>,
    directional: Option<DirectionalNav>,
    len: usize,
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
    parent_scope: usize,
    scope: Option<Box<FocusScopeData>>,
}

const NO_PARENT_SCOPE: usize = usize::max_value();

#[derive(new)]
pub(crate) struct FocusMap {
    #[new(default)]
    current_scopes: Vec<usize>,
    #[new(default)]
    offset: LayoutPoint,
    #[new(default)]
    entries: Vec<FocusEntry>,
}
impl FocusMap {
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
        let parent_scope = *self.current_scopes.last().unwrap_or(&NO_PARENT_SCOPE);

        self.current_scopes.push(self.entries.len());
        self.entries.push(FocusEntry {
            key,
            origin: rect.center() + self.offset.to_vector(),
            parent_scope,
            scope: Some(Box::new(FocusScopeData {
                menu,
                tab,
                directional,
                len: 0,
                size: rect.size,
            })),
        });
    }

    pub fn pop_focus_scope(&mut self) {
        let i = self.current_scopes.pop().expect("Popped with no pushed FocusScope");
        self.entries[i].scope.as_mut().unwrap().len = self.entries.len() - i;
    }

    pub fn push_focusable(&mut self, key: FocusKey, origin: LayoutPoint) {
        self.entries.push(FocusEntry {
            key,
            origin: origin + self.offset.to_vector(),
            parent_scope: *self.current_scopes.last().expect("Pushed Focusable without FocusScope"),
            scope: None,
        });
    }

    fn position(&self, focus_key: FocusKey) -> Option<usize> {
        self.entries.iter().position(|o| o.key == focus_key)
    }

    ///Iterator over all entries that are not a menu inside the scope denoted by `scope_i`
    fn skip_menu(&self, scope_i: usize) -> impl DoubleEndedIterator<Item = &FocusEntry> {
        let mut skip_end = None;
        let mut scope_end = None;
        self.entries
            .iter()
            .enumerate()
            .filter(move |&(i, e)| {
                if let Some(value) = scope_end {
                    if i == value {
                        scope_end = None;
                        return true;
                    }
                    if let Some(value) = skip_end {
                        if i < value {
                            return false;
                        }
                        skip_end = None;
                    }
                    if let Some(scope) = &e.scope {
                        if scope.menu {
                            skip_end = Some(i + scope.len);
                            return false;
                        }
                    }
                } else if scope_i == i {
                    scope_end = Some(i + e.scope.as_ref().unwrap().len);
                    return true;
                }
                true
            })
            .map(|(_, e)| e)
    }

    fn starting_point(&self) -> Option<FocusKey> {
        self.skip_menu(0).next().map(|e| e.key)
    }

    fn is_inside(&self, parent_scope: usize, scope: usize) -> bool {
        if parent_scope == NO_PARENT_SCOPE {
            false
        } else if parent_scope == scope {
            true
        } else {
            self.is_inside(self.entries[parent_scope].parent_scope, scope)
        }
    }

    fn find_parent(&self, parent_scope: usize, predicate: impl Fn(&FocusScopeData) -> bool) -> Option<usize> {
        if parent_scope == NO_PARENT_SCOPE {
            None
        } else {
            let scope = self.entries[parent_scope].scope.as_ref().unwrap();
            if predicate(&scope) {
                Some(parent_scope)
            } else {
                self.find_parent(self.entries[parent_scope].parent_scope, predicate)
            }
        }
    }

    /// Returns vector of distance from `origin`, item, item parent scope.
    fn candidates_towards(&self, direction: FocusRequest, origin: LayoutPoint) -> Vec<(f32, FocusKey, usize)> {
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(move |c| is_in_direction(direction, origin, c.origin))
            .map(|c| {
                let o = c.origin;
                let a = (o.x - origin.x).powf(2.);
                let b = (o.y - origin.y).powf(2.);
                (a + b, c.key, c.parent_scope)
            })
            .collect();

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        candidates
    }

    fn next_towards(&self, direction: FocusRequest, key: FocusKey) -> FocusKey {
        // get candidates in direction from current focus.
        let current = self.entries.iter().find(|o| o.key == key).unwrap();
        let origin = current.origin;
        let candidates = self.candidates_towards(direction, origin);

        // if current focus is inside a scope that retains directional.
        if let Some(scope_i) = self.find_parent(current.parent_scope, |s| s.retains_directional()) {
            if let Some(c) = candidates.iter().find(|c| self.is_inside(c.2, scope_i)) {
                // if any candidate is inside same focus.
                return c.1;
            } else {
                // all candidates outside retaining scope, need to do retention.

                let scope = &self.entries[scope_i];
                let scope_data = scope.scope.as_ref().unwrap();
                match scope_data.directional {
                    // contained retention does not change focus, already is last in direction inside scope.
                    Some(DirectionalNav::Contained) => return key,
                    // cycling retention, finds closest to new origin that is
                    // in the same line or column of current focus but on the other side
                    // of the parent scope rectangle.
                    Some(DirectionalNav::Cycle) => {
                        let mut origin = origin;
                        match direction {
                            FocusRequest::Left => {
                                origin.x = scope.origin.x + scope_data.size.width / 2.;
                            }
                            FocusRequest::Right => {
                                origin.x = scope.origin.x - scope_data.size.width / 2.;
                            }
                            FocusRequest::Up => {
                                origin.y = scope.origin.y + scope_data.size.height / 2.;
                            }
                            FocusRequest::Down => {
                                origin.y = scope.origin.y - scope_data.size.height / 2.;
                            }
                            _ => unreachable!(),
                        }

                        let candidates = self.candidates_towards(direction, origin);
                        if let Some(c) = candidates.iter().find(|c| self.is_inside(c.2, scope_i)) {
                            // if can find candidate on other side.
                            return c.1;
                        } else {
                            // else do the same as contained.
                            // probably a bug, should have found the current focus again at least.
                            return key;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        if let Some(c) = candidates.first() {
            c.1
        } else {
            key
        }
    }

    fn next(&self, current_focus: FocusKey) -> FocusKey {
        // current focused index
        let curr_i = self.entries.iter().position(|o| o.key == current_focus).unwrap();

        // if current is focus scope
        if let Some(scope) = &self.entries[curr_i].scope {
            let first_inside = self.skip_menu(curr_i).find(|e| e.parent_scope == curr_i);

            if let Some(c) = first_inside {
                return c.key;
            } else if scope.retains_tab() {
                // capture scope that is empty, holds the focus.
                return current_focus;
            }
        }

        let curr_scope = self.entries[curr_i].parent_scope;
        match self.entries[curr_scope].scope.as_ref().unwrap().tab {
            Some(mode) => {
                match self.entries.get(curr_i + 1) {
                    // try to get the next item in the same scope.
                    Some(next) if next.parent_scope == curr_scope => next.key,
                    // did not find next, returns the..
                    _ => match mode {
                        //.. first item in scope.
                        TabNav::Cycle => {
                            self.skip_menu(curr_scope)
                                .find(|e| e.parent_scope == curr_scope)
                                .unwrap()
                                .key
                        }
                        //.. last item in scope.
                        TabNav::Contained => current_focus,
                        _ => unimplemented!(),
                    },
                }
            }
            None => {
                // try to get the next item.
                if let Some(next) = self.entries.get(curr_i + 1) {
                    if next.parent_scope == curr_scope {
                        next.key
                    }
                    // we need to check if next is valid in the context of
                    // the scope's parent scope.

                    // next is inside parent scope that captures.
                    else if let Some(capture_scope) = self.find_parent(curr_scope, |s| s.retains_tab()) {
                        if self.is_inside(next.parent_scope, capture_scope) {
                            next.key
                        } else {
                            // next was not inside parent scope that captures, returns
                            match self.entries[capture_scope].scope.as_ref().unwrap().tab.unwrap() {
                                // first item in scope that captures.
                                TabNav::Cycle => {
                                    self.entries
                                        .iter()
                                        .find(|e| e.parent_scope == capture_scope)
                                        .unwrap()
                                        .key
                                }
                                // last item in scope that captures.
                                TabNav::Contained => current_focus,
                                _ => unimplemented!(),
                            }
                        }
                    } else {
                        // next is outside current scope, but not inside any capturing scope
                        next.key
                    }
                } else if let Some(capture_scope) = self.find_parent(curr_scope, |s| s.retains_tab()) {
                    // we are the last entry and have parent capturing scope.
                    // return
                    match self.entries[capture_scope].scope.as_ref().unwrap().tab.unwrap() {
                        // first entry in scope that captures.
                        TabNav::Cycle => {
                            self.entries
                                .iter()
                                .find(|e| e.parent_scope == capture_scope)
                                .unwrap()
                                .key
                        }
                        // last entry in scope that captures.
                        TabNav::Contained => current_focus,
                        _ => unimplemented!(),
                    }
                } else {
                    // we are the last entry and have no parent capturing scope.
                    self.entries[0].key
                }
            }
        }
    }

    fn prev(&self, current_focus: FocusKey) -> FocusKey {
        // current focused index
        let curr_i = self.entries.iter().position(|o| o.key == current_focus).unwrap();

        // if current is focus scope
        if let Some(scope) = &self.entries[curr_i].scope {
            let last_inside = self.entries.iter().rev().find(|e| e.parent_scope == curr_i);

            if let Some(c) = last_inside {
                return c.key;
            } else if scope.retains_tab() {
                // capture scope that is empty, holds the focus.
                return current_focus;
            }
        }

        let curr_scope = self.entries[curr_i].parent_scope;
        match self.entries[curr_scope].scope.as_ref().unwrap().tab {
            Some(mode) => {
                // if has prev entry.
                if curr_i > 0 {
                    // if prev entry is current scope.
                    if curr_i - 1 == curr_scope {
                        match mode {
                            //.. last item in scope.
                            TabNav::Cycle => {
                                self.skip_menu(curr_scope)
                                    .rev()
                                    .find(|e| e.parent_scope == curr_scope)
                                    .unwrap()
                                    .key
                            }
                            //.. first item in scope.
                            TabNav::Contained => current_focus,
                            TabNav::Continue => self.entries[curr_i - 1].key,
                            TabNav::Once => unimplemented!()
                        }
                    } else {
                        self.entries[curr_i - 1].key
                    }
                } else {
                    unimplemented!()
                }
            }
            None => {
                // try to get the previous item.
                if curr_i > 0 {
                    let prev = &self.entries[curr_i - 1];

                    if prev.parent_scope == curr_scope {
                        prev.key
                    }
                    // we need to check if previous is valid in the context of
                    // the scope's parent scope.

                    // previous is inside parent scope that captures.
                    else if let Some(capture_scope) = self.find_parent(curr_scope, |s| s.retains_tab()) {
                        if self.is_inside(prev.parent_scope, capture_scope) {
                            prev.key
                        } else {
                            // previous was not inside parent scope that captures. returns
                            match self.entries[capture_scope].scope.as_ref().unwrap().tab.unwrap() {
                                // last item in scope that captures.
                                TabNav::Cycle => {
                                    self.entries
                                        .iter()
                                        .rev()
                                        .find(|e| e.parent_scope == capture_scope)
                                        .unwrap()
                                        .key
                                }
                                // first item in scope that captures.
                                TabNav::Contained => current_focus,
                                _ => unimplemented!(),
                            }
                        }
                    } else {
                        // prev is outside current scope, but not inside any capturing scope
                        prev.key
                    }
                } else if let Some(capture_scope) = self.find_parent(curr_scope, |s| s.retains_tab()) {
                    // we are the first entry and have parent capturing scope.
                    // return
                    match self.entries[capture_scope].scope.as_ref().unwrap().tab.unwrap() {
                        // last entry in scope that captures.
                        TabNav::Cycle => {
                            self.entries
                                .iter()
                                .rev()
                                .find(|e| e.parent_scope == capture_scope)
                                .unwrap()
                                .key
                        }
                        // first entry in scope that captures.
                        TabNav::Contained => current_focus,
                        _ => unimplemented!(),
                    }
                } else {
                    // we are the first entry and have no parent capturing scope.
                    self.entries[self.entries.len() - 1].key
                }
            }
        }
    }

    /// Gets next focus key  from a current `focused` and a change `request`.
    pub fn focus(&self, focused: Option<FocusKey>, request: FocusRequest) -> Option<FocusKey> {
        match (request, focused) {
            (FocusRequest::Direct(direct_key), _) => self.position(direct_key).map(|_| direct_key),
            (_, None) => self.starting_point(),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(key)) => Some(self.next(key)),
            (FocusRequest::Prev, Some(key)) => Some(self.prev(key)),
            (FocusRequest::EnterAlt, Some(_key)) => unimplemented!(),
            (FocusRequest::EscapeAlt, Some(_key)) => unimplemented!(),
            //Arrow Keys
            (direction, Some(key)) => Some(self.next_towards(direction, key)),
        }
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

#[derive(Debug, PartialEq, Eq)]
pub enum FocusStatus {
    Focused,
    FocusWithin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::point2;

    fn is_left(origin: LayoutPoint, candidate: LayoutPoint) -> bool {
        is_in_direction(FocusRequest::Left, origin, candidate)
    }

    fn is_right(origin: LayoutPoint, candidate: LayoutPoint) -> bool {
        is_in_direction(FocusRequest::Right, origin, candidate)
    }

    fn is_up(origin: LayoutPoint, candidate: LayoutPoint) -> bool {
        is_in_direction(FocusRequest::Up, origin, candidate)
    }

    fn is_down(origin: LayoutPoint, candidate: LayoutPoint) -> bool {
        is_in_direction(FocusRequest::Down, origin, candidate)
    }

    #[test]
    fn candidate_culling_left() {
        assert!(!is_left(point2(10., 10.), point2(11., 10.)));
        assert!(is_left(point2(10., 10.), point2(9., 10.)));

        assert!(is_left(point2(10., 10.), point2(9., 11.)));
        assert!(!is_left(point2(10., 10.), point2(9., 12.)));
        assert!(is_left(point2(10., 10.), point2(5., 12.)));

        assert!(is_left(point2(10., 10.), point2(9., 9.)));
        assert!(!is_left(point2(10., 10.), point2(9., 8.)));
        assert!(is_left(point2(10., 10.), point2(5., 8.)));

        assert!(!is_left(point2(10., 10.), point2(10., 10.)));
    }

    #[test]
    fn candidate_culling_right() {
        assert!(!is_right(point2(10., 10.), point2(9., 10.)));
        assert!(is_right(point2(10., 10.), point2(11., 10.)));

        assert!(is_right(point2(10., 10.), point2(11., 11.)));
        assert!(!is_right(point2(10., 10.), point2(11., 12.)));
        assert!(is_right(point2(10., 10.), point2(15., 12.)));

        assert!(is_right(point2(10., 10.), point2(11., 9.)));
        assert!(!is_right(point2(10., 10.), point2(11., 8.)));
        assert!(is_right(point2(10., 10.), point2(15., 8.)));

        assert!(!is_right(point2(10., 10.), point2(10., 10.)));
    }

    #[test]
    fn candidate_culling_up() {
        assert!(!is_up(point2(10., 10.), point2(10., 11.)));
        assert!(is_up(point2(10., 10.), point2(10., 9.)));

        assert!(is_up(point2(10., 10.), point2(11., 9.)));
        assert!(!is_up(point2(10., 10.), point2(12., 9.)));
        assert!(is_up(point2(10., 10.), point2(12., 5.)));

        assert!(is_up(point2(10., 10.), point2(9., 9.)));
        assert!(!is_up(point2(10., 10.), point2(8., 9.)));
        assert!(is_up(point2(10., 10.), point2(8., 5.)));

        assert!(!is_up(point2(10., 10.), point2(10., 10.)));
    }

    #[test]
    fn candidate_culling_down() {
        assert!(!is_down(point2(10., 10.), point2(10., 9.)));
        assert!(is_down(point2(10., 10.), point2(10., 11.)));

        assert!(is_down(point2(10., 10.), point2(11., 11.)));
        assert!(!is_down(point2(10., 10.), point2(12., 11.)));
        assert!(is_down(point2(10., 10.), point2(12., 15.)));

        assert!(is_down(point2(10., 10.), point2(9., 11.)));
        assert!(!is_down(point2(10., 10.), point2(8., 11.)));
        assert!(is_down(point2(10., 10.), point2(8., 15.)));

        assert!(!is_down(point2(10., 10.), point2(10., 10.)));
    }
}
