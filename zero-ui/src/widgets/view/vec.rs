use std::ops;

use crate::core::var::{VarUpdateId, VarValue, VARS};

/// Represents a [`Vec<T>`] that tracks changes when used inside a variable.
///
/// The changes made in the last update are available in [`ObservableVec::changes`].
///
/// This struct is designed to be a data source for [`list_presenter`], because it tracks
/// exact changes it enables the implementation of transition animations such as a new
/// element *expanding* into place, it also allows the retention of widget state for elements
/// that did not change.
///
/// [`list_presenter`]: crate::widgets::list_presenter
#[derive(Debug, Clone, PartialEq)]
pub struct ObservableVec<T: VarValue> {
    list: Vec<T>,
    changes: VecChanges,
}
impl<T: VarValue> Default for ObservableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: VarValue> ops::Deref for ObservableVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.list.deref()
    }
}
impl<T: VarValue> ObservableVec<T> {
    /// New empty vec.
    pub const fn new() -> Self {
        Self {
            list: vec![],
            changes: VecChanges::new(),
        }
    }

    /// New empty vec with pre-allocated capacity.
    ///
    /// See [`Vec::with_capacity`].
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            list: Vec::with_capacity(capacity),
            changes: VecChanges::new(),
        }
    }

    /// Reserves capacity for at least additional more elements.
    ///
    /// See [`Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.list.reserve(additional);
    }

    /// Insert the `element` at the `index`.
    ///
    /// See [`Vec::insert`].
    pub fn insert(&mut self, index: usize, element: T) {
        self.list.insert(index, element);
        self.changes.push(VecChange::Insert { index, count: 1 });
    }

    /// Insert the `element` at the end of the vec.
    ///
    /// See [`Vec::push`].
    pub fn push(&mut self, element: T) {
        self.insert(self.len(), element);
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// See [`Vec::append`].
    pub fn append(&mut self, other: &mut Vec<T>) {
        self.changes.push(VecChange::Insert {
            index: self.list.len(),
            count: other.len(),
        });
        self.list.append(other);
    }

    /// Remove the `index` element.
    ///
    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        let r = self.list.remove(index);
        self.changes.push(VecChange::Remove { index, count: 1 });
        r
    }

    /// Remove the last element from the vec.
    ///
    /// See [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(self.len() - 1))
        }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// See [`Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            let count = self.len() - len;
            self.changes.push(VecChange::Remove { index: len, count });
        }
        self.list.truncate(len);
    }

    /// Removes an element from the vector and returns it.
    ///
    /// See [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        let r = self.list.swap_remove(index);

        self.changes.push(VecChange::Remove { index, count: 1 });
        self.changes.push(VecChange::Move {
            from_index: self.list.len() - 1,
            to_index: index,
        });

        r
    }

    /// Removes all elements.
    ///
    /// See [`Vec::clear`].
    pub fn clear(&mut self) {
        if !self.is_empty() {
            self.clear();
            self.changes.clear();
        }
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// See [`Vec::retain_mut`] for more details.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut i = 0;

        self.list.retain_mut(|it| {
            let retain = f(it);
            if retain {
                i += 1;
            } else {
                self.changes.push(VecChange::Remove { index: i, count: 1 });
            }
            retain
        })
    }

    /// Removes the specified range from the vector in bulk, returning all removed elements as an iterator.
    ///
    /// See [`Vec::drain`].
    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, T>
    where
        R: ops::RangeBounds<usize>,
    {
        let range = std_slice_range(range, ..self.len());
        let r = self.list.drain(range.clone());

        if !range.is_empty() {
            self.changes.push(VecChange::Remove {
                index: range.start,
                count: range.len(),
            });
        }

        r
    }

    /// Resizes the Vec in-place so that len is equal to `new_len`.
    ///
    /// See [`Vec::resize`].
    pub fn resize(&mut self, new_len: usize, value: T) {
        if new_len <= self.len() {
            self.truncate(new_len);
        } else {
            let count = new_len - self.len();
            self.changes.push(VecChange::Insert { index: self.len(), count });
            self.list.resize(new_len, value);
        }
    }

    /// Clones and appends all elements in a slice to the Vec.
    ///
    /// See [`Vec::extend_from_slice`].
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if !other.is_empty() {
            self.changes.push(VecChange::Insert {
                index: self.len(),
                count: other.len(),
            });
        }
        self.list.extend_from_slice(other);
    }

    /// Copies elements from `src` range to the end of the vector.
    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: ops::RangeBounds<usize>,
    {
        let src = std_slice_range(src, ..self.len());

        let index = self.len();

        self.list.extend_from_within(src.clone());

        if !src.is_empty() {
            self.changes.push(VecChange::Insert { index, count: src.len() });
        }
    }

    /// Mutate the `index`.
    ///
    /// This logs a [`VecChange::Remove`] and [`VecChange::Insert`] for the `index`, if it is valid.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let r = self.list.get_mut(index);
        if r.is_some() {
            self.changes.push(VecChange::Remove { index, count: 1 });
            self.changes.push(VecChange::Insert { index, count: 1 });
        }
        r
    }

    /// Mutate the `range`.
    ///
    /// This logs a [`VecChange::Remove`] and [`VecChange::Insert`] for the `range`, if it is valid.
    pub fn slice_mut<R>(&mut self, range: R) -> &mut [T]
    where
        R: ops::RangeBounds<usize>,
    {
        let range = std_slice_range(range, ..self.len());
        let r = &mut self.list[range.clone()];

        let count = range.len();
        if count > 0 {
            self.changes.push(VecChange::Remove { index: range.start, count });
            self.changes.push(VecChange::Insert { index: range.start, count });
        }

        r
    }

    /// Changes applied in the last var update.
    ///
    /// If the variable is new and this is empty assume the entire vector was replaced (same as [`VecChange::Clear`]).
    pub fn changes(&self) -> &[VecChange] {
        if self.changes.update_id == VARS.update_id() {
            &self.changes.changes
        } else {
            &[]
        }
    }
}

impl<T: VarValue> Extend<T> for ObservableVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let index = self.len();
        self.list.extend(iter);
        let count = self.len() - index;
        if count > 0 {
            self.changes.push(VecChange::Insert { index, count });
        }
    }
}
impl<T: VarValue> From<Vec<T>> for ObservableVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            list: value,
            changes: VecChanges::new(),
        }
    }
}
impl<T: VarValue> From<ObservableVec<T>> for Vec<T> {
    fn from(value: ObservableVec<T>) -> Self {
        value.list
    }
}

/// Represents a change in a [`ObservableVec`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VecChange {
    /// Elements removed.
    Remove {
        /// Index of the first element removed, at the time of removal.
        index: usize,
        /// Number of elements removed.
        count: usize,
    },
    /// Elements inserted.
    Insert {
        /// Index of the first element inserted, at the time of insertion.
        index: usize,
        /// Number of elements inserted.
        count: usize,
    },
    /// Element removed an reinserted.
    Move {
        /// Index the element was first at.
        from_index: usize,
        /// Index the element was reinserted after removal.
        to_index: usize,
    },
    /// All elements removed/replaced.
    Clear,
}

#[derive(Debug, PartialEq)]
struct VecChanges {
    changes: Vec<VecChange>,
    update_id: VarUpdateId,
}
impl Clone for VecChanges {
    fn clone(&self) -> Self {
        let update_id = VARS.update_id();
        if self.update_id == update_id {
            Self {
                changes: self.changes.clone(),
                update_id,
            }
        } else {
            Self {
                changes: vec![],
                update_id,
            }
        }
    }
}
impl VecChanges {
    const fn new() -> Self {
        Self {
            changes: vec![],
            update_id: VarUpdateId::never(),
        }
    }

    pub fn push(&mut self, change: VecChange) {
        let update_id = VARS.update_id();
        if self.update_id != update_id {
            self.changes.clear();
            self.changes.push(change);
            self.update_id = update_id;
        } else if self.changes != [VecChange::Clear] {
            if let Some(last) = self.changes.last_mut() {
                match last {
                    VecChange::Remove { index, count } => {
                        if let VecChange::Remove { index: i2, count: c2 } = change {
                            let i1 = *index;
                            if i1 == i2 {
                                *count += c2;
                            }
                        }
                    }
                    VecChange::Insert { index, count } => {
                        if let VecChange::Insert { index: i2, count: c2 } = change {
                            let i1 = *index;
                            let c1 = *count;
                            if i2 == i1 + c1 {
                                *count += c2;
                            }
                        }
                    }
                    VecChange::Move { .. } => {}
                    VecChange::Clear => unreachable!(),
                }
            } else {
                self.changes.push(change);
            }
        }
    }

    pub fn clear(&mut self) {
        self.changes.clear();
        self.changes.push(VecChange::Clear);
        self.update_id = VARS.update_id();
    }
}

// See <https://github.com/rust-lang/rust/issues/76393>
#[track_caller]
#[must_use]
fn std_slice_range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
where
    R: ops::RangeBounds<usize>,
{
    let len = bounds.end;

    let start: ops::Bound<&usize> = range.start_bound();
    let start = match start {
        ops::Bound::Included(&start) => start,
        ops::Bound::Excluded(start) => start.checked_add(1).unwrap(),
        ops::Bound::Unbounded => 0,
    };

    let end: ops::Bound<&usize> = range.end_bound();
    let end = match end {
        ops::Bound::Included(end) => end.checked_add(1).unwrap(),
        ops::Bound::Excluded(&end) => end,
        ops::Bound::Unbounded => len,
    };

    if start > end {
        panic!("invalid range {start}..{end}");
    }
    if end > len {
        panic!("invalid range {start}..{end}");
    }

    ops::Range { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{app::App, var::*};

    #[test]
    fn basic_usage() {
        let mut app = App::minimal().run_headless(false);

        let list = var(ObservableVec::<u32>::new());

        list.modify(|a| {
            a.to_mut().push(32);
        });
        app.update_observe(
            || {
                assert!(list.is_new());

                list.with_new(|l| {
                    assert_eq!(&[32], &l[..]);
                    assert_eq!(&[VecChange::Insert { index: 0, count: 1 }], l.changes());
                });
            },
            false,
        )
        .assert_wait();

        list.modify(|a| {
            a.to_mut().push(33);
        });
        app.update_observe(
            || {
                assert!(list.is_new());

                list.with_new(|l| {
                    assert_eq!(&[32, 33], &l[..]);
                    assert_eq!(&[VecChange::Insert { index: 1, count: 1 }], l.changes());
                });
            },
            false,
        )
        .assert_wait();
    }
}
