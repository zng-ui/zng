use super::{ChildValueKey, ChildValueKeyRef, LayoutPoint, LayoutRect};

uid! {
    /// Focusable unique identifier.
    pub struct FocusKey(_);
}

/// Custom focus navigation implementation must return this to stop
/// the default implementation on `keyboard_input`.
pub static FOCUS_HANDLED: ChildValueKeyRef<()> = ChildValueKey::new_lazy();

#[derive(Clone, Copy)]
pub enum FocusRequest {
    /// Move focus to key.
    Direct(FocusKey),
    /// Move focus to next from current in screen, or to starting key.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,
    /// Move focus to parent focus scope.
    Escape,

    Left,
    Right,
    Up,
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

#[derive(Clone, Copy)]
pub enum KeyNavigation {
    /// TAB goes to next in text reading order.
    /// Capture: TAB in last item goes back to first.
    /// Not capture: TAB in last item goes to next item after scope.
    Tab,
    /// Arrows goes to closest item in the arrow direction.
    /// Capture: Arrow press into edge of scope loops back to begining of the same line or column.
    ///    * Search next within a range to the same direction but in a parallel dimension?
    ///    * Remember dimension that entered item when going back (instead of using middle)?
    /// Not capture: Behaves like parent scope allows arrow navigation within this scope.
    Arrows,
    Both,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Capture,
    Cycle,
}

struct FocusScopeData {
    _navigation: KeyNavigation,
    capture: Option<CaptureMode>,
    len: usize,
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
        navigation: KeyNavigation,
        capture: Option<CaptureMode>,
    ) {
        let parent_scope = *self.current_scopes.last().unwrap_or(&NO_PARENT_SCOPE);

        self.current_scopes.push(self.entries.len());
        self.entries.push(FocusEntry {
            key,
            origin: rect.center() + self.offset.to_vector(),
            parent_scope,
            scope: Some(Box::new(FocusScopeData {
                _navigation: navigation,
                capture,
                len: 0,
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

    fn starting_point(&self) -> Option<FocusKey> {
        self.entries.first().map(|e| e.key)
    }

    fn query_capture_scope(&self, parent_scope: usize) -> Option<usize> {
        if parent_scope == NO_PARENT_SCOPE {
            None
        } else {
            let scope = self.entries[parent_scope].scope.as_ref().unwrap();
            if scope.capture.is_some() {
                Some(parent_scope)
            } else {
                self.query_capture_scope(self.entries[parent_scope].parent_scope)
            }
        }
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

    fn next_towards(&self, direction: FocusRequest, key: FocusKey) -> FocusKey {
        let current = self.entries.iter().find(|o| o.key == key).unwrap();
        let origin = current.origin;

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

        if let Some(scope) = self.query_capture_scope(current.parent_scope) {
            if let Some(c) = candidates.iter().find(|c| self.is_inside(c.2, scope)) {
                return c.1;
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
            let first_inside = self.entries.iter().find(|e| e.parent_scope == curr_i);

            if let Some(c) = first_inside {
                return c.key;
            } else if scope.capture.is_some() {
                // capture scope that is empty, holds the focus.
                return current_focus;
            }
        }

        let curr_scope = self.entries[curr_i].parent_scope;
        match self.entries[curr_scope].scope.as_ref().unwrap().capture {
            Some(mode) => {
                match self.entries.get(curr_i + 1) {
                    // try to get the next item in the same scope.
                    Some(next) if next.parent_scope == curr_scope => next.key,
                    // did not find next, returns the..
                    _ => match mode {
                        //.. first item in scope.
                        CaptureMode::Cycle => self.entries.iter().find(|e| e.parent_scope == curr_scope).unwrap().key,
                        //.. last item in scope.
                        CaptureMode::Capture => current_focus,
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
                    else if let Some(capture_scope) = self.query_capture_scope(curr_scope) {
                        if self.is_inside(next.parent_scope, capture_scope) {
                            next.key
                        } else {
                            // next was not inside parent scope that captures. returns
                            match self.entries[capture_scope].scope.as_ref().unwrap().capture.unwrap() {
                                // first item in scope that captures.
                                CaptureMode::Cycle => {
                                    self.entries
                                        .iter()
                                        .find(|e| e.parent_scope == capture_scope)
                                        .unwrap()
                                        .key
                                }
                                // last item in scope that captures.
                                CaptureMode::Capture => current_focus,
                            }
                        }
                    } else {
                        // next is outside current scope, but not inside any capturing scope
                        next.key
                    }
                } else if let Some(capture_scope) = self.query_capture_scope(curr_scope) {
                    // we are the last entry and have parent capturing scope.
                    // return
                    match self.entries[capture_scope].scope.as_ref().unwrap().capture.unwrap() {
                        // first entry in scope that captures.
                        CaptureMode::Cycle => {
                            self.entries
                                .iter()
                                .find(|e| e.parent_scope == capture_scope)
                                .unwrap()
                                .key
                        }
                        // last entry in scope that captures.
                        CaptureMode::Capture => current_focus,
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
            } else if scope.capture.is_some() {
                // capture scope that is empty, holds the focus.
                return current_focus;
            }
        }

        let curr_scope = self.entries[curr_i].parent_scope;
        match self.entries[curr_scope].scope.as_ref().unwrap().capture {
            Some(mode) => {
                let prev = if curr_i > 0 {
                    Some(&self.entries[curr_i - 1])
                } else {
                    None
                };
                match prev {
                    // try to get the previous item in the same scope.
                    Some(prev) if prev.parent_scope == curr_scope => prev.key,
                    // did not find previous, returns the..
                    _ => match mode {
                        //.. last item in scope.
                        CaptureMode::Cycle => {
                            self.entries
                                .iter()
                                .rev()
                                .find(|e| e.parent_scope == curr_scope)
                                .unwrap()
                                .key
                        }
                        //.. first item in scope.
                        CaptureMode::Capture => current_focus,
                    },
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
                    else if let Some(capture_scope) = self.query_capture_scope(curr_scope) {
                        if self.is_inside(prev.parent_scope, capture_scope) {
                            prev.key
                        } else {
                            // previous was not inside parent scope that captures. returns
                            match self.entries[capture_scope].scope.as_ref().unwrap().capture.unwrap() {
                                // last item in scope that captures.
                                CaptureMode::Cycle => {
                                    self.entries
                                        .iter()
                                        .rev()
                                        .find(|e| e.parent_scope == capture_scope)
                                        .unwrap()
                                        .key
                                }
                                // first item in scope that captures.
                                CaptureMode::Capture => current_focus,
                            }
                        }
                    } else {
                        // prev is outside current scope, but not inside any capturing scope
                        prev.key
                    }
                } else if let Some(capture_scope) = self.query_capture_scope(curr_scope) {
                    // we are the first entry and have parent capturing scope.
                    // return
                    match self.entries[capture_scope].scope.as_ref().unwrap().capture.unwrap() {
                        // last entry in scope that captures.
                        CaptureMode::Cycle => {
                            self.entries
                                .iter()
                                .rev()
                                .find(|e| e.parent_scope == capture_scope)
                                .unwrap()
                                .key
                        }
                        // first entry in scope that captures.
                        CaptureMode::Capture => current_focus,
                    }
                } else {
                    // we are the first entry and have no parent capturing scope.
                    self.entries[self.entries.len() - 1].key
                }
            }
        }
    }

    pub fn focus(&self, focused: Option<FocusKey>, r: FocusRequest) -> Option<FocusKey> {
        match (r, focused) {
            (FocusRequest::Direct(direct_key), _) => self.position(direct_key).map(|_| direct_key),
            (_, None) => self.starting_point(),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(key)) => Some(self.next(key)),
            (FocusRequest::Prev, Some(key)) => Some(self.prev(key)),
            (FocusRequest::Escape, Some(_key)) => unimplemented!(),
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
