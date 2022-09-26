use std::mem;

use crate::{
    app::AppEventSender,
    context::{AppContext, Updates},
};

use super::*;

/// Represents the last time a variable was mutated or the current update cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarUpdateId(u32);
impl VarUpdateId {
    /// ID that is never new.
    pub const fn never() -> Self {
        VarUpdateId(0)
    }

    fn next(&mut self) {
        if self.0 == u32::MAX {
            self.0 = 1;
        } else {
            self.0 += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct VarApplyUpdateId(u32);
impl VarApplyUpdateId {
    /// ID that is never returned in `Vars`.
    pub(super) const fn initial() -> Self {
        VarApplyUpdateId(0)
    }

    fn next(&mut self) {
        if self.0 == u32::MAX {
            self.0 = 1;
        } else {
            self.0 += 1;
        }
    }
}

pub(super) type VarUpdateFn = Box<dyn FnOnce(&Vars, &mut Updates)>;

thread_singleton!(SingletonVars);

/// Enables write access for [`Var<T>`].
pub struct Vars {
    _singleton: SingletonVars,
    app_event_sender: AppEventSender,

    update_id: VarUpdateId,
    apply_update_id: VarApplyUpdateId,

    updates: RefCell<Vec<VarUpdateFn>>,
    spare_updates: Vec<VarUpdateFn>,

    modify_receivers: RefCell<Vec<Box<dyn Fn(&Vars) -> bool>>>,
}
impl Vars {
    /// Id of the current vars update in the app scope.
    ///
    /// Variable with [`AnyVar::update_id`] equal to this are *new*.
    pub fn update_id(&self) -> VarUpdateId {
        self.update_id
    }

    pub(crate) fn instance(app_event_sender: AppEventSender) -> Vars {
        Vars {
            _singleton: SingletonVars::assert_new("Vars"),
            app_event_sender,
            update_id: VarUpdateId(1),
            apply_update_id: VarApplyUpdateId(1),
            updates: RefCell::new(Vec::with_capacity(128)),
            spare_updates: Vec::with_capacity(128),
            modify_receivers: RefCell::new(vec![]),
        }
    }

    pub(super) fn schedule_update(&self, update: VarUpdateFn) {
        self.updates.borrow_mut().push(update);
    }

    /// Id of each `schedule_update` cycle during `apply_updates`
    pub(super) fn apply_update_id(&self) -> VarApplyUpdateId {
        self.apply_update_id
    }

    pub(crate) fn apply_updates(&mut self, updates: &mut Updates) {
        debug_assert!(self.spare_updates.is_empty());

        self.update_id.next();

        while !self.updates.get_mut().is_empty() {
            let mut var_updates = mem::replace(self.updates.get_mut(), mem::take(&mut self.spare_updates));
            for update in var_updates.drain(..) {
                update(self, updates);
            }
            self.spare_updates = var_updates;

            self.apply_update_id.next();
        }
    }

    pub(crate) fn on_pre_vars(ctx: &mut AppContext) {
        todo!()
    }
    pub(crate) fn on_vars(ctx: &mut AppContext) {
        todo!()
    }

    pub(crate) fn register_channel_recv(&self, recv_modify: Box<dyn Fn(&Vars) -> bool>) {
        self.modify_receivers.borrow_mut().push(recv_modify);
    }

    pub(crate) fn app_event_sender(&self) -> AppEventSender {
        self.app_event_sender.clone()
    }

    pub(crate) fn receive_sended_modify(&self) {
        let mut rcvs = mem::take(&mut *self.modify_receivers.borrow_mut());
        rcvs.retain(|rcv| rcv(self));

        let mut rcvs_mut = self.modify_receivers.borrow_mut();
        rcvs.extend(rcvs_mut.drain(..));
        *rcvs_mut = rcvs;
    }
}

/// Represents temporary access to [`Vars`].
///
/// All contexts that provide [`Vars`] implement this trait to facilitate access to it.
pub trait WithVars {
    /// Visit the [`Vars`] reference.
    fn with_vars<R, F: FnOnce(&Vars) -> R>(&self, visit: F) -> R;
}
impl WithVars for Vars {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self)
    }
}

/*

impl<'a> WithVars for crate::context::AppContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WindowContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
    A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WidgetContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::LayoutContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl WithVars for crate::context::AppContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
impl WithVars for crate::context::WidgetContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithVars for crate::context::TestWidgetContext {
    fn with_vars<R, A>(&self, action: A) -> R
    where
    A: FnOnce(&Vars) -> R,
    {
        action(&self.vars)
    }
}
impl WithVars for crate::app::HeadlessApp {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars())
    }
}

        */
