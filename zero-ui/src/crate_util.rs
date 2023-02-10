use std::{mem, ops};

/// Runs a cleanup action once on drop.
pub(crate) struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

pub(crate) trait Recycle: Default {
    fn recycle(&mut self);
}

#[derive(Default)]
pub(crate) struct RecycleVec<T: Recycle> {
    vec: Vec<T>,
    fresh_len: usize,
}
impl<T: Recycle> RecycleVec<T> {
    pub fn begin_reuse(&mut self) {
        self.fresh_len = 0;
    }

    pub fn commit_reuse(&mut self) {
        self.vec.truncate(self.fresh_len);
    }

    pub fn new_item(&mut self) -> T {
        if self.vec.len() > self.fresh_len {
            let mut item = self.vec.pop().unwrap();
            item.recycle();
            item
        } else {
            T::default()
        }
    }

    pub fn push(&mut self, item: T) {
        if self.fresh_len == self.vec.len() {
            self.vec.push(item);
        } else {
            let old = mem::replace(&mut self.vec[self.fresh_len], item);
            self.vec.push(old);
        }
        self.fresh_len += 1;
    }

    pub fn push_renew(&mut self, item: &mut T) {
        let new_item = self.new_item();
        let item = mem::replace(item, new_item);
        self.push(item);
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.vec[..self.fresh_len].iter_mut()
    }
}
impl<T: Recycle> ops::Deref for RecycleVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.vec[..self.fresh_len]
    }
}
impl<T: Recycle> ops::DerefMut for RecycleVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec[..self.fresh_len]
    }
}
