use super::{LayoutPoint, LayoutRect};

uid! {
    /// Focusable unique identifier.
    pub struct FocusKey(_);
}

#[derive(Clone, Copy)]
pub enum FocusRequest {
    /// Move focus to key.
    Direct(FocusKey),
    /// Move focus to next from current in screen, or to starting key.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,
    Left,
    Right,
    Up,
    Down,
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

struct FocusScopeData {
    navigation: KeyNavigation,
    capture: bool,
    len: usize,
}

struct FocusEntry {
    key: FocusKey,
    origin: LayoutPoint,
    parent_scope: usize,
    scope: Option<Box<FocusScopeData>>,
}

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

    pub fn push_focus_scope(&mut self, key: FocusKey, origin: LayoutPoint, navigation: KeyNavigation, capture: bool) {
        let parent_scope = *self.current_scopes.last().unwrap_or(&0);

        self.current_scopes.push(self.entries.len());
        self.entries.push(FocusEntry {
            key,
            origin: origin + self.offset.to_vector(),
            parent_scope,
            scope: Some(Box::new(FocusScopeData {
                navigation,
                capture,
                len: 0,
            })),
        });
    }

    pub fn pop_fucus_scope(&mut self) {
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
        unimplemented!()
    }

    fn next_towards(&self, direction: FocusRequest, key: FocusKey) -> FocusKey {
        let origin = self.entries.iter().filter(|o| o.key == key).next().unwrap().origin;

        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(move |c| is_in_direction(direction, origin, c.origin))
            .map(|c| {
                let o = c.origin;
                let a = (o.x - origin.x).powf(2.);
                let b = (o.y - origin.y).powf(2.);
                (a + b, c.key)
            })
            .collect();

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        candidates.first().map(|c| c.1).unwrap_or(key)
    }

    pub fn focus(&self, focused: Option<FocusKey>, r: FocusRequest) -> Option<FocusKey> {
        match (r, focused) {
            (FocusRequest::Direct(direct_key), _) => self.position(direct_key).map(|_| direct_key),
            (_, None) => self.starting_point(),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(key)) => unimplemented!(),
            (FocusRequest::Prev, Some(key)) => unimplemented!(),
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
