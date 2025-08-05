//! Observable vec var value.

use std::ops;

use crate::{VARS, VarUpdateId, VarValue};

/// Represents a [`Vec<T>`] that tracks changes when used inside a variable.
///
/// The changes made in the last update are available in [`ObservableVec::changes`].
///
/// This struct is designed to be a data source for list presenters, because it tracks
/// exact changes it enables the implementation of transition animations such as a new
/// element expanding into place, it also allows the retention of widget state for elements
/// that did not change.
///
/// Changes are logged using the [`VecChange`] enum, note that the enum only tracks indexes at the
/// moment the change happens, that means that you cannot get the removed items and [`VecChange::Insert`]
/// must be the last change in an update cycle. If any change is made that invalidates an `Insert` all
/// changes for the cycle are collapsed to [`VecChange::Clear`], to avoid this try to only `remove`
/// before `insert`.
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
        self.changes.inserted(index, 1);
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
        self.changes.inserted(self.list.len(), other.len());
        self.list.append(other);
    }

    /// Remove the `index` element.
    ///
    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        let r = self.list.remove(index);
        self.changes.removed(index, 1);
        r
    }

    /// Remove the last element from the vec.
    ///
    /// See [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() { None } else { Some(self.remove(self.len() - 1)) }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// See [`Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            let count = self.len() - len;
            self.changes.removed(len, count);
        }
        self.list.truncate(len);
    }

    /// Removes an element from the vector and returns it.
    ///
    /// See [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        let r = self.list.swap_remove(index);

        self.changes.removed(index, 1);
        self.changes.moved(self.list.len() - 1, index);

        r
    }

    /// Removes all elements.
    ///
    /// See [`Vec::clear`].
    pub fn clear(&mut self) {
        if !self.is_empty() {
            self.clear();
            self.changes.cleared();
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
                self.changes.removed(i, 1);
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
            self.changes.removed(range.start, range.len());
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
            self.changes.inserted(self.len(), count);
            self.list.resize(new_len, value);
        }
    }

    /// Clones and appends all elements in a slice to the Vec.
    ///
    /// See [`Vec::extend_from_slice`].
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if !other.is_empty() {
            self.changes.inserted(self.len(), other.len());
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
            self.changes.inserted(index, src.len());
        }
    }

    /// Move the element `from` index `to` index.
    ///
    /// If `from < to` this is the same as `self.insert(to - 1, self.remove(from))`, otherwise
    /// is the same as `self.insert(to, self.remove(from))`, but the change is tracked
    /// as a single [`VecChange::Move`].
    pub fn reinsert(&mut self, from: usize, mut to: usize) {
        if from != to {
            if from < to {
                to -= 1;
            }
            let el = self.list.remove(from);
            self.list.insert(to, el);
            self.changes.moved(from, to);
        } else {
            // assert contained
            let _ = &self.list[to];
        }
    }

    /// Mutate the `index`.
    ///
    /// This logs a [`VecChange::Remove`] and [`VecChange::Insert`] for the `index`, if it is valid.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let r = self.list.get_mut(index);
        if r.is_some() {
            self.changes.removed(index, 1);
            self.changes.inserted(index, 1);
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
            self.changes.removed(range.start, count);
            self.changes.inserted(range.start, count);
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
            self.changes.inserted(index, count);
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
        ///
        /// In [`ObservableVec::changes`] the index and count can be used to select
        /// the new elements in the vec because if any (re)move is made after insert the
        /// changes are collapsed to a `Clear`.
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

    pub fn inserted(&mut self, i: usize, n: usize) {
        let update_id = VARS.update_id();
        if self.update_id != update_id {
            self.changes.clear();
            self.changes.push(VecChange::Insert { index: i, count: n });
            self.update_id = update_id;
        } else if self.changes != [VecChange::Clear] {
            if let Some(VecChange::Insert { index, count }) = self.changes.last_mut() {
                if i >= *index && i <= *index + *count {
                    // new insert inside previous
                    *count += n;
                    return;
                } else {
                    // insert indexes need to be patched.
                    self.changes.clear();
                    self.changes.push(VecChange::Clear);
                    return;
                }
            }
            self.changes.push(VecChange::Insert { index: i, count: n });
        }
    }

    pub fn moved(&mut self, f: usize, t: usize) {
        let update_id = VARS.update_id();
        if self.update_id != update_id {
            self.changes.clear();
            self.changes.push(VecChange::Move {
                from_index: f,
                to_index: t,
            });
            self.update_id = update_id;
        } else if self.changes != [VecChange::Clear] {
            self.changes.push(VecChange::Move {
                from_index: f,
                to_index: t,
            });
        }
    }

    pub fn removed(&mut self, i: usize, n: usize) {
        let update_id = VARS.update_id();
        if self.update_id != update_id {
            self.changes.clear();
            self.changes.push(VecChange::Remove { index: i, count: n });
            self.update_id = update_id;
        } else if self.changes != [VecChange::Clear] {
            if let Some(last) = self.changes.last_mut() {
                match last {
                    VecChange::Remove { index, count } => {
                        let s = i;
                        let e = i + n;

                        if s <= *index && e > *index {
                            // new remove contains previous remove.
                            *index = s;
                            *count += n;
                            return;
                        }
                    }
                    VecChange::Insert { .. } => {
                        // insert indexes need to be patched.
                        self.changes.clear();
                        self.changes.push(VecChange::Clear);
                        return;
                    }
                    _ => {}
                }
            }

            self.changes.push(VecChange::Remove { index: i, count: n });
        }
    }

    pub fn cleared(&mut self) {
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
