use std::{fmt, sync::Arc};

use parking_lot::Mutex;

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    let cap = 10000;
    let page = Arc::new(Mutex::new(Vec::with_capacity(cap)));
    (
        Sender { page: page.clone() },
        Receiver {
            page,
            read: Vec::with_capacity(cap),
        },
    )
}

pub struct Sender<T> {
    page: Arc<Mutex<Vec<T>>>,
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self { page: self.page.clone() }
    }
}
impl<T: fmt::Debug> Sender<T> {
    pub fn send(&self, value: T) {
        let mut page = self.page.lock();
        page.push(value);
    }
}

pub struct Receiver<T> {
    page: Arc<Mutex<Vec<T>>>,
    read: Vec<T>,
}
impl<T: fmt::Debug> Receiver<T> {
    pub fn try_recv(&mut self) -> Option<T> {
        match self.read.pop() {
            Some(val) => Some(val),
            None => {
                {
                    let mut page = self.page.lock();
                    std::mem::swap(&mut *page, &mut self.read);
                }
                self.read.reverse();
                self.read.pop()
            }
        }
    }
}
