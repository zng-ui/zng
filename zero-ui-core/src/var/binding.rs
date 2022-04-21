//! Var binding types and tests, implementation is in `vars.rs`

use std::cell::Cell;

use crate::crate_util::*;

/// Represents a variable binding created by one of the `bind` methods of [`Vars`] or [`Var`].
///
/// Drop all clones of this handle to drop the binding, or call [`permanent`] to drop the handle
/// but keep the binding alive for the duration of the app.
///
/// [`permanent`]: VarBindingHandle::permanent
/// [`Vars`]: crate::var::Vars
/// [`Var`]: crate::var::Var
#[derive(Clone, PartialEq, Eq, Hash)]
#[must_use = "the var binding is undone if the handle is dropped"]
pub struct VarBindingHandle(Handle<()>);
impl VarBindingHandle {
    pub(super) fn new() -> (HandleOwner<()>, VarBindingHandle) {
        let (owner, handle) = Handle::new(());
        (owner, VarBindingHandle(handle))
    }

    /// Create dummy handle that is always in the *unbound* state.
    #[inline]
    pub fn dummy() -> VarBindingHandle {
        VarBindingHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unbind.
    ///
    /// The var binding stays in memory for the duration of the app or until another handle calls [`unbind`](Self::unbind).
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unbind`](Self::unbind) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the binding to drop.
    #[inline]
    pub fn unbind(self) {
        self.0.force_drop();
    }

    /// If another handle has called [`unbind`](Self::unbind).
    ///
    /// The var binding is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_unbound(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    #[inline]
    pub fn downgrade(&self) -> WeakVarBindingHandle {
        WeakVarBindingHandle(self.0.downgrade())
    }
}

/// Weak [`VarBindingHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct WeakVarBindingHandle(WeakHandle<()>);
impl WeakVarBindingHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Get the binding handle if it is still bound.
    pub fn upgrade(&self) -> Option<VarBindingHandle> {
        self.0.upgrade().map(VarBindingHandle)
    }
}

/// Represents the variable binding in its binding closure.
///
/// See the [`Vars::bind`] method for more details.
///
/// [`Vars::bind`]: crate::var::Vars::bind
pub struct VarBinding {
    unbind: Cell<bool>,
}
impl VarBinding {
    pub(super) fn new() -> Self {
        VarBinding { unbind: Cell::new(false) }
    }

    /// Drop the binding after applying the returned update.
    #[inline]
    pub fn unbind(&self) {
        self.unbind.set(true);
    }

    /// If the binding will be dropped after applying the update.
    #[inline]
    pub fn unbind_requested(&self) -> bool {
        self.unbind.get()
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::text::ToText;
    use crate::var::{var, Var};

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_map(&app.ctx(), &b, |_, a| a.to_text()).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(13i32), a.copy_new(ctx));
                assert_eq!(Some("13".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_map_bidi(&app.ctx(), &b, |_, a| a.to_text(), |_, b| b.parse().unwrap())
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.clone_new(ctx));
                assert_eq!(Some(55i32), a.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_filter(&app.ctx(), &b, |_, a| if *a == 13 { None } else { Some(a.to_text()) })
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(13i32), a.copy_new(ctx));
                assert_eq!("20".to_text(), b.get_clone(ctx));
                assert!(!b.is_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_filter_bidi(&app.ctx(), &b, |_, a| Some(a.to_text()), |_, b| b.parse().ok())
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.clone_new(ctx));
                assert_eq!(Some(55i32), a.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "not a i32");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("not a i32".to_text()), b.clone_new(ctx));
                assert_eq!(55i32, a.copy(ctx));
                assert!(!a.is_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless(false);

        a.bind_map(&app.ctx(), &b, |_, a| *a + 1).permanent();
        b.bind_map(&app.ctx(), &c, |_, b| *b + 1).permanent();
        c.bind_map(&app.ctx(), &d, |_, c| *c + 1).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(20), a.copy_new(ctx));
                assert_eq!(Some(21), b.copy_new(ctx));
                assert_eq!(Some(22), c.copy_new(ctx));
                assert_eq!(Some(23), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(30), a.copy_new(ctx));
                assert_eq!(Some(31), b.copy_new(ctx));
                assert_eq!(Some(32), c.copy_new(ctx));
                assert_eq!(Some(33), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_bidi_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless(false);

        a.bind_bidi(&app.ctx(), &b).permanent();
        b.bind_bidi(&app.ctx(), &c).permanent();
        c.bind_bidi(&app.ctx(), &d).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(20), a.copy_new(ctx));
                assert_eq!(Some(20), b.copy_new(ctx));
                assert_eq!(Some(20), c.copy_new(ctx));
                assert_eq!(Some(20), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        d.set(app.ctx().vars, 30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(30), a.copy_new(ctx));
                assert_eq!(Some(30), b.copy_new(ctx));
                assert_eq!(Some(30), c.copy_new(ctx));
                assert_eq!(Some(30), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_inside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless(false);

        let _handle = a.bind_map(&app.ctx(), &b, |info, i| {
            info.unbind();
            *i + 1
        });

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(10), a.copy_new(ctx));
                assert_eq!(Some(11), b.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());

        a.set(app.ctx().vars, 100);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(100), a.copy_new(ctx));
                assert!(!b.is_new(ctx));
                assert_eq!(11, b.copy(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_outside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless(false);

        let handle = a.bind_map(&app.ctx(), &b, |_, i| *i + 1);

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(10), a.copy_new(ctx));
                assert_eq!(Some(11), b.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        drop(handle);

        a.set(app.ctx().vars, 100);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(100), a.copy_new(ctx));
                assert!(!b.is_new(ctx));
                assert_eq!(11, b.copy(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }
}
