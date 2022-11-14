* Refactor variables to use global lock.
    - Using the lock AND RefCell at the same time has no noticeable performance impact.
        - Need merge-var to truly test.
    - Remove RefCell of more vars.

* Replace `Var::with` with `Var::read(&self) -> VarReadGuard<T>`?
    - Need something new in ContextVar, an extra lock?
    - `Var::with` is not so bad.
```rust
/// Represents a read locked variable value.
pub struct VarReadGuard<'a, T: VarValue> {
    value: &'a T,
    _guard: Option<RwLockReadGuard<'a, ()>>,
}
impl<'a, T: VarValue> VarReadGuard<'a, T> {
    pub(super) fn new_mutable(value: &'a T, guard: RwLockReadGuard<'a, ()>) -> Self {
        Self {
            value,
            _guard: Some(guard),
        }
    }
    pub(super) fn new_imutable(value: &'a T) -> Self {
        Self { value, _guard: None }
    }
}
impl<'a, T: VarValue> ops::Deref for VarReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
```

* Make Var<T> and VarValue Send+Sync.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?