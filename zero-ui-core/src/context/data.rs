use std::{cell::RefCell, fmt, mem, rc::Rc};

use linear_map::LinearMap;

use crate::crate_util::RunOnDrop;

unique_id_64! {
    /// Unique ID of a [`DataContext`].
    pub struct DataContextId;
}

struct DataContextData {
    id: DataContextId,
    parent: Option<DataContext>,
    drop_handlers: RefCell<Vec<Box<dyn FnOnce(DataContextId)>>>,
}
impl Drop for DataContextData {
    fn drop(&mut self) {
        for c in self.drop_handlers.get_mut().drain(..) {
            c(self.id);
        }
    }
}

/// Identifies the current data context.
#[derive(Clone)]
pub struct DataContext(Rc<DataContextData>);
impl DataContext {
    /// New base context, does not inherit current.
    pub fn new_base() -> Self {
        DataContext(Rc::new(DataContextData {
            id: DataContextId::new_unique(),
            parent: None,
            drop_handlers: RefCell::new(vec![]),
        }))
    }

    /// New context that inherit from current.
    pub fn new_inherit() -> Self {
        DataContext(Rc::new(DataContextData {
            id: DataContextId::new_unique(),
            parent: Some(Self::current()),
            drop_handlers: RefCell::new(vec![]),
        }))
    }

    /// Clone a reference of the current context.
    pub fn current() -> Self {
        CURRENT.with(|ctx| ctx.borrow().clone())
    }

    /// Unique ID of the context.
    pub fn id(&self) -> DataContextId {
        self.0.id
    }

    /// Iterate over context ancestors, parent, grand-parent, etc.
    pub fn ancestors(&self) -> impl Iterator<Item = DataContextId> + '_ {
        let mut parent = &self.0.parent;
        std::iter::from_fn(move || {
            if let Some(p) = &parent {
                let id = p.id();
                parent = &p.0.parent;
                Some(id)
            } else {
                None
            }
        })
    }

    /// Calls `f` in the data context.
    pub fn with_context<R>(&self, f: impl FnOnce() -> R) -> R {
        CURRENT.with(|ctx| {
            let old = mem::replace(&mut *ctx.borrow_mut(), self.clone());
            let _restore = RunOnDrop::new(|| *ctx.borrow_mut() = old);
            f()
        })
    }

    /// Register a `callback` to run when all clones of this context are dropped.
    pub fn on_drop(&self, callback: Box<dyn FnOnce(DataContextId)>) {
        self.0.drop_handlers.borrow_mut().push(callback)
    }
}

thread_local! {
    static CURRENT: RefCell<DataContext> = RefCell::new(DataContext::new_base());
}

struct ContextualizedDataData<T> {
    versions: RefCell<LinearMap<DataContextId, T>>,
}
