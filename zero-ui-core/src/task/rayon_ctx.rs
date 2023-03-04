use rayon::{iter::plumbing::{UnindexedConsumer, Consumer}, prelude::ParallelIterator};

use crate::context::ThreadContext;

pub trait ParallelIteratorCtx: ParallelIterator {
    fn ctx(self) -> ParallelCtx<Self> {
        ParallelCtx {
            base: self,
            ctx: ThreadContext::capture(),
        }
    }
}

impl<I: ParallelIterator> ParallelIteratorCtx for I {}

pub struct ParallelCtx<I> {
    base: I,
    ctx: ThreadContext,
}
impl<T, I> ParallelIterator for ParallelCtx<I>
where
    T: Send,
    I: ParallelIterator<Item = T>,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        std::thread::current().id()
        let consumer = ParallelCtxConsumer {
            base: consumer,
            ctx: self.ctx,
        };
        self.base.drive_unindexed(consumer)
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
    type Folder;

    type Reducer;

    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        todo!()
    }

    fn into_folder(self) -> Self::Folder {
        todo!()
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<T, C> UnindexedConsumer<T> for ParallelCtxConsumer<C>
where
    C: UnindexedConsumer<T>,
    T: Send,
{
    fn split_off_left(&self) -> Self {
        todo!()
    }

    fn to_reducer(&self) -> Self::Reducer {
        todo!()
    }
}
