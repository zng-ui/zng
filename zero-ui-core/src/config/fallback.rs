use parking_lot::Mutex;

use crate::var::*;

use super::*;

/// Represents a config source that is read and written too, when a key is not present in the source
/// the fallback variable is used, but if that variable is modified the key is inserted in the primary config.
pub struct FallbackConfig<S: Config, F: Config> {
    fallback: F,
    config: S,
}
impl<S: Config, F: Config> FallbackConfig<S, F> {
    /// New config.
    pub fn new(config: S, fallback: F) -> Self {
        Self { fallback, config }
    }
}
impl<S: Config, F: Config> AnyConfig for FallbackConfig<S, F> {
    fn status(&self) -> BoxedVar<ConfigStatus> {
        merge_var!(self.fallback.status(), self.config.status(), |fallback, over| {
            ConfigStatus::merge_status([fallback.clone(), over.clone()].into_iter())
        })
        .boxed()
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        let over = self.config.get_raw(key.clone(), default.clone(), shared);
        if self.config.contains_key(&key) {
            return over;
        }

        let fallback = self.fallback.get_raw(key, default, shared);
        let result = var(fallback.get());

        #[derive(Clone, Copy)]
        enum State {
            Fallback,
            FallbackUpdated,
            Over,
            OverUpdated,
        }
        let state = Arc::new(atomic::Atomic::new(State::Fallback));

        // hook fallback, signal `result` that an update is flowing from the fallback.
        let wk_result = result.downgrade();
        fallback
            .hook(Box::new(clmv!(state, |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Over | State::OverUpdated => {
                        // result -> over
                        return false;
                    }
                    _ => {}
                }

                // fallback -> result
                if let Some(result) = wk_result.upgrade() {
                    state.store(State::FallbackUpdated, atomic::Ordering::Relaxed);
                    result.set(value.as_any().downcast_ref::<RawConfigValue>().unwrap().clone());
                    true
                } else {
                    // weak-ref to avoid circular ref.
                    false
                }
            })))
            .perm();

        // hook over, signals `result` that an update is flowing from the override.
        let wk_result = result.downgrade();
        over.hook(Box::new(clmv!(state, |value| {
            match state.load(atomic::Ordering::Relaxed) {
                State::OverUpdated => {
                    // result -> over
                    state.store(State::Over, atomic::Ordering::Relaxed);
                }
                _ => {
                    // over -> result
                    let value = value.as_any().downcast_ref::<RawConfigValue>().unwrap();
                    state.store(State::OverUpdated, atomic::Ordering::Relaxed);
                    if let Some(result) = wk_result.upgrade() {
                        result.set(value.clone());
                    } else {
                        // weak-ref to avoid circular ref.
                        return false;
                    }
                }
            }

            true
        })))
        .perm();

        // hook result, on first callback not caused by `fallback` drops it and changes to `over`.
        let fallback = Mutex::new(Some(fallback));
        result
            .hook(Box::new(move |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Fallback => {
                        // result -> over(first)
                        state.store(State::Over, atomic::Ordering::Relaxed);
                        *fallback.lock() = None;
                        let value = value.as_any().downcast_ref::<RawConfigValue>().unwrap().clone();
                        let _ = over.set_ne(value);
                    }
                    State::FallbackUpdated => {
                        // fallback -> result
                        state.store(State::Fallback, atomic::Ordering::Relaxed);
                    }
                    State::Over => {
                        // result -> over
                        let value = value.as_any().downcast_ref::<RawConfigValue>().unwrap().clone();
                        let _ = over.set_ne(value);
                    }
                    State::OverUpdated => {
                        // over -> result
                        state.store(State::Over, atomic::Ordering::Relaxed);
                    }
                }
                true
            }))
            .perm();

        result.boxed()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.fallback.contains_key(key) || self.config.contains_key(key)
    }
}
impl<S: Config, F: Config> Config for FallbackConfig<S, F> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let key = key.into();
        let default = default();
        let fallback = self.fallback.get(key.clone(), || default.clone());
        let over = var(None::<T>); // TODO, actually provided by self.source
        if over.with(|s| s.is_some()) {
            return self.config.get(key, move || default);
        }
        let result = var(fallback.get());

        #[derive(Clone, Copy)]
        enum State {
            Fallback,
            FallbackUpdated,
            Over,
            OverUpdated,
        }
        let state = Arc::new(atomic::Atomic::new(State::Fallback));

        // hook fallback, signal `result` that an update is flowing from the fallback.
        let wk_result = result.downgrade();
        fallback
            .hook(Box::new(clmv!(state, |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Over | State::OverUpdated => {
                        // result -> over
                        return false;
                    }
                    _ => {}
                }

                // fallback -> result
                if let Some(result) = wk_result.upgrade() {
                    state.store(State::FallbackUpdated, atomic::Ordering::Relaxed);
                    result.set(value.as_any().downcast_ref::<T>().unwrap().clone());
                    true
                } else {
                    // weak-ref to avoid circular ref.
                    false
                }
            })))
            .perm();

        // hook over, signals `result` that an update is flowing from the override.
        let wk_result = result.downgrade();
        over.hook(Box::new(clmv!(state, |value| {
            match state.load(atomic::Ordering::Relaxed) {
                State::OverUpdated => {
                    // result -> over
                    state.store(State::Over, atomic::Ordering::Relaxed);
                }
                _ => {
                    // over -> result
                    if let Some(value) = value.as_any().downcast_ref::<Option<T>>().unwrap() {
                        state.store(State::OverUpdated, atomic::Ordering::Relaxed);
                        if let Some(result) = wk_result.upgrade() {
                            result.set(value.clone());
                        } else {
                            // weak-ref to avoid circular ref.
                            return false;
                        }
                    }
                }
            }

            true
        })))
        .perm();

        // hook result, on first callback not caused by `fallback` drops it and changes to `over`.
        let fallback = Mutex::new(Some(fallback));
        result
            .hook(Box::new(move |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Fallback => {
                        // result -> over(first)
                        state.store(State::Over, atomic::Ordering::Relaxed);
                        *fallback.lock() = None;
                        over.set(Some(value.as_any().downcast_ref::<T>().unwrap().clone()));
                    }
                    State::FallbackUpdated => {
                        // fallback -> result
                        state.store(State::Fallback, atomic::Ordering::Relaxed);
                    }
                    State::Over => {
                        // result -> over
                        over.set(Some(value.as_any().downcast_ref::<T>().unwrap().clone()));
                    }
                    State::OverUpdated => {
                        // over -> result
                        state.store(State::Over, atomic::Ordering::Relaxed);
                    }
                }
                true
            }))
            .perm();

        result.boxed()
    }
}
