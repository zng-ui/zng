use rayon::{
    iter::plumbing::*,
    prelude::{IndexedParallelIterator, ParallelIterator},
};

use crate::context::ThreadContext;

/// Extends [`ParallelIterator`] with thread context.
pub trait ParallelIteratorExt: ParallelIterator {
    /// Captures the current [`ThreadContext`] and propagates it to all rayon tasks
    /// generated running this parallel iterator.
    ///
    /// Without this adapter all closures in the iterator chain that use [`context_local!`] and
    /// [`app_local!`] will probably not work correctly.
    ///
    /// [`context_local!`]: crate::context::context_local
    /// [`app_local!`]: crate::context::app_local
    fn with_ctx(self) -> ParallelIteratorWithCtx<Self> {
        ParallelIteratorWithCtx {
            base: self,
            ctx: ThreadContext::capture(),
        }
    }
}

impl<I: ParallelIterator> ParallelIteratorExt for I {}

/// Parallel iterator adapter the propagates the thread context.
///
/// See [`ParallelIteratorCtx`] for more details.
pub struct ParallelIteratorWithCtx<I> {
    base: I,
    ctx: ThreadContext,
}
impl<T, I> ParallelIterator for ParallelIteratorWithCtx<I>
where
    T: Send,
    I: ParallelIterator<Item = T>,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let consumer = ParallelCtxConsumer {
            base: consumer,
            ctx: self.ctx.clone(),
        };
        self.ctx.with_context(move || self.base.drive_unindexed(consumer))
    }

    fn opt_len(&self) -> Option<usize> {
        self.ctx.with_context(move || self.base.opt_len())
    }
}
impl<I: IndexedParallelIterator> IndexedParallelIterator for ParallelIteratorWithCtx<I> {
    fn len(&self) -> usize {
        self.ctx.with_context(move || self.base.len())
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let consumer = ParallelCtxConsumer {
            base: consumer,
            ctx: self.ctx.clone(),
        };
        self.ctx.with_context(move || self.base.drive(consumer))
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        let callback = ParallelCtxProducerCallback {
            base: callback,
            ctx: self.ctx.clone(),
        };
        self.ctx.with_context(move || self.base.with_producer(callback))
    }
}

struct ParallelCtxConsumer<C> {
    base: C,
    ctx: ThreadContext,
}
impl<T, C> Consumer<T> for ParallelCtxConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = ParallelCtxFolder<C::Folder>;
    type Reducer = ParallelCtxReducer<C::Reducer>;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.ctx.with_context(|| self.base.split_at(index));
        let reducer = ParallelCtxReducer {
            base: reducer,
            ctx: self.ctx.clone(),
        };
        let left = Self {
            base: left,
            ctx: self.ctx.clone(),
        };
        let right = Self {
            base: right,
            ctx: self.ctx,
        };
        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        let base = self.ctx.with_context(|| self.base.into_folder());
        ParallelCtxFolder { base, ctx: self.ctx }
    }

    fn full(&self) -> bool {
        self.ctx.with_context(|| self.base.full())
    }
}

impl<T, C> UnindexedConsumer<T> for ParallelCtxConsumer<C>
where
    C: UnindexedConsumer<T>,
    T: Send,
{
    fn split_off_left(&self) -> Self {
        let base = self.ctx.with_context(|| self.base.split_off_left());
        Self {
            base,
            ctx: self.ctx.clone(),
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        let base = self.ctx.with_context(|| self.base.to_reducer());
        ParallelCtxReducer {
            base,
            ctx: self.ctx.clone(),
        }
    }
}

struct ParallelCtxFolder<F> {
    base: F,
    ctx: ThreadContext,
}
impl<Item, F> Folder<Item> for ParallelCtxFolder<F>
where
    F: Folder<Item>,
{
    type Result = F::Result;

    fn consume(self, item: Item) -> Self {
        let base = self.ctx.with_context(move || self.base.consume(item));
        Self { base, ctx: self.ctx }
    }

    fn complete(self) -> Self::Result {
        self.ctx.with_context(|| self.base.complete())
    }

    fn full(&self) -> bool {
        self.ctx.with_context(|| self.base.full())
    }
}

struct ParallelCtxReducer<R> {
    base: R,
    ctx: ThreadContext,
}
impl<Result, R> Reducer<Result> for ParallelCtxReducer<R>
where
    R: Reducer<Result>,
{
    fn reduce(self, left: Result, right: Result) -> Result {
        self.ctx.with_context(move || self.base.reduce(left, right))
    }
}

struct ParallelCtxProducerCallback<C> {
    base: C,
    ctx: ThreadContext,
}
impl<T, C: ProducerCallback<T>> ProducerCallback<T> for ParallelCtxProducerCallback<C> {
    type Output = C::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        let producer = ParallelCtxProducer {
            base: producer,
            ctx: self.ctx.clone(),
        };
        self.ctx.with_context(move || self.base.callback(producer))
    }
}

struct ParallelCtxProducer<P> {
    base: P,
    ctx: ThreadContext,
}
impl<P: Producer> Producer for ParallelCtxProducer<P> {
    type Item = P::Item;

    type IntoIter = P::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.ctx.with_context(|| self.base.into_iter())
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.ctx.with_context(|| self.base.split_at(index));
        (
            Self {
                base: left,
                ctx: self.ctx.clone(),
            },
            Self {
                base: right,
                ctx: self.ctx,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    };

    use super::*;
    use rayon::prelude::*;

    use crate::{app::App, context::*};

    context_local! {
        static VALUE: u32 = 0u32;
    }

    #[test]
    fn map_and_sum_with_context() {
        let _app = App::minimal().run_headless(false);
        let thread_id = std::thread::current().id();
        let used_other_thread = Arc::new(AtomicBool::new(false));

        let sum: u32 = VALUE.with_context(&mut Some(1), || {
            (0..1000)
                .into_par_iter()
                .with_ctx()
                .map(|_| {
                    if thread_id != std::thread::current().id() {
                        used_other_thread.store(true, Ordering::Relaxed);
                    }
                    VALUE.get()
                })
                .sum()
        });

        assert_eq!(sum, 1000);
        assert!(used_other_thread.load(Ordering::Relaxed));
    }

    #[test]
    fn for_each_with_context() {
        let _app = App::minimal().run_headless(false);
        let thread_id = std::thread::current().id();
        let used_other_thread = Arc::new(AtomicBool::new(false));

        let sum: u32 = VALUE.with_context(&mut Some(1), || {
            let sum = Arc::new(AtomicU32::new(0));
            (0..1000).into_par_iter().with_ctx().for_each(|_| {
                if thread_id != std::thread::current().id() {
                    used_other_thread.store(true, Ordering::Relaxed);
                }
                sum.fetch_add(VALUE.get(), Ordering::Relaxed);
            });
            sum.load(Ordering::Relaxed)
        });

        assert_eq!(sum, 1000);
        assert!(used_other_thread.load(Ordering::Relaxed));
    }

    #[test]
    fn chain_for_each_with_context() {
        let _app = App::minimal().run_headless(false);
        let thread_id = std::thread::current().id();
        let used_other_thread = Arc::new(AtomicBool::new(false));

        let sum: u32 = VALUE.with_context(&mut Some(1), || {
            let sum = Arc::new(AtomicU32::new(0));

            let a = (0..500).into_par_iter();
            let b = (0..500).into_par_iter();

            a.chain(b).with_ctx().for_each(|_| {
                if thread_id != std::thread::current().id() {
                    used_other_thread.store(true, Ordering::Relaxed);
                }
                sum.fetch_add(VALUE.get(), Ordering::Relaxed);
            });
            sum.load(Ordering::Relaxed)
        });

        assert_eq!(sum, 1000);
        assert!(used_other_thread.load(Ordering::Relaxed));
    }

    #[test]
    fn chain_for_each_with_context_inverted() {
        let _app = App::minimal().run_headless(false);
        let thread_id = std::thread::current().id();
        let used_other_thread = Arc::new(AtomicBool::new(false));

        let sum: u32 = VALUE.with_context(&mut Some(1), || {
            let sum = Arc::new(AtomicU32::new(0));

            let a = (0..500).into_par_iter().with_ctx();
            let b = (0..500).into_par_iter().with_ctx();

            a.chain(b).for_each(|_| {
                if thread_id != std::thread::current().id() {
                    used_other_thread.store(true, Ordering::Relaxed);
                }
                sum.fetch_add(VALUE.get(), Ordering::Relaxed);
            });
            sum.load(Ordering::Relaxed)
        });

        assert_eq!(sum, 1000);
        assert!(used_other_thread.load(Ordering::Relaxed));
    }
}
