use atomic::Atomic;
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
        let cfg_var = self.config.get_raw(key.clone(), default.clone(), shared);
        if self.config.contains_key(&key) {
            // no need for fallback
            return cfg_var;
        }

        let fall_var = self.fallback.get_raw(key, default, shared);

        let res_var = var(fall_var.get());

        // based on `Var::bind_bidi` code.
        let last_bidi_update = Arc::new(Atomic::new((VarUpdateId::never(), VarUpdateId::never())));

        // bind cfg_var -> res_var, handles potential bidi binding
        let weak_res_var = res_var.downgrade();
        cfg_var
            .hook(Box::new(clmv!(last_bidi_update, |v| {
                if let Some(res_var) = weak_res_var.upgrade() {
                    let update_id = VARS.update_id();
                    let (_, res_id) = last_bidi_update.load(atomic::Ordering::Relaxed);
                    if update_id != res_id {
                        // res_var did not cause this assign, propagate.
                        last_bidi_update.store((update_id, res_id), atomic::Ordering::Relaxed);
                        res_var.set_ne(v.as_any().downcast_ref::<RawConfigValue>().unwrap().clone());
                    }

                    true
                } else {
                    false
                }
            })))
            .perm();

        // bind fallback_var -> res_var.
        let weak_res_var = res_var.downgrade();
        let fall_to_res = fall_var.hook(Box::new(clmv!(last_bidi_update, |v| {
            if let Some(res_var) = weak_res_var.upgrade() {
                let (cfg_id, res_id) = last_bidi_update.load(atomic::Ordering::Relaxed);
                let update_id = VARS.update_id();

                // if cfg_var or res_var updates in the same cycle we are about to be dropped.
                let retain = update_id != cfg_id && update_id != res_id;

                if retain {
                    // only fall_var updated
                    res_var.set_ne(v.as_any().downcast_ref::<RawConfigValue>().unwrap().clone());
                }
                retain
            } else {
                false
            }
        })));

        // map res_var -> cfg_var, manages fallback binding lifetime.
        let fall_var = Mutex::new(Some((fall_var, fall_to_res)));
        res_var
            .hook(Box::new(move |v| {
                let mut fall_var = fall_var.lock();
                let update_id = VARS.update_id();

                if let Some((fv, _)) = &*fall_var {
                    if fv.last_update() == update_id {
                        // update from fallback
                        return true;
                    } else {
                        *fall_var = None;
                    }
                }

                let (cfg_id, _) = last_bidi_update.load(atomic::Ordering::Relaxed);
                if update_id != cfg_id {
                    // cfg_var did not cause this assign.
                    last_bidi_update.store((cfg_id, update_id), atomic::Ordering::Relaxed);
                    let _ = cfg_var.set_ne(v.as_any().downcast_ref::<RawConfigValue>().unwrap().clone());
                }

                true
            }))
            .perm();

        res_var.boxed()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.fallback.contains_key(key) || self.config.contains_key(key)
    }
}
impl<S: Config, F: Config> Config for FallbackConfig<S, F> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let key = key.into();
        let default = default();

        let cfg_var = self.config.get(key.clone(), || default.clone());
        if self.config.contains_key(&key) {
            // no need for fallback
            return cfg_var;
        }

        let fall_var = self.fallback.get(key, || default);

        let res_var = var(fall_var.get());

        // based on `Var::bind_bidi` code.
        let last_bidi_update = Arc::new(Atomic::new((VarUpdateId::never(), VarUpdateId::never())));

        // bind cfg_var -> res_var, handles potential bidi binding
        let weak_res_var = res_var.downgrade();
        cfg_var
            .hook(Box::new(clmv!(last_bidi_update, |v| {
                if let Some(res_var) = weak_res_var.upgrade() {
                    let update_id = VARS.update_id();
                    let (_, ots_id) = last_bidi_update.load(atomic::Ordering::Relaxed);
                    if update_id != ots_id {
                        // other_to_self did not cause this assign, propagate.
                        last_bidi_update.store((update_id, ots_id), atomic::Ordering::Relaxed);
                        res_var.set(v.as_any().downcast_ref::<T>().unwrap().clone());
                    }

                    true
                } else {
                    false
                }
            })))
            .perm();

        // bind fallback_var -> res_var.
        let weak_res_var = res_var.downgrade();
        let fall_to_res = fall_var.hook(Box::new(clmv!(last_bidi_update, |v| {
            if let Some(res_var) = weak_res_var.upgrade() {
                let (cfg_id, res_id) = last_bidi_update.load(atomic::Ordering::Relaxed);
                let update_id = VARS.update_id();

                // if cfg_var or res_var updates in the same cycle we are about to be dropped.
                let retain = update_id != cfg_id && update_id != res_id;

                if retain {
                    // only fall_var updated
                    res_var.set(v.as_any().downcast_ref::<T>().unwrap().clone());
                }
                retain
            } else {
                false
            }
        })));

        // map res_var -> cfg_var, manages fallback binding lifetime.
        let fall_var = Mutex::new(Some((fall_var, fall_to_res)));
        res_var
            .hook(Box::new(move |v| {
                let mut fall_var = fall_var.lock();
                let update_id = VARS.update_id();

                if let Some((fv, _)) = &*fall_var {
                    if fv.last_update() == update_id {
                        // update from fallback
                        return true;
                    } else {
                        *fall_var = None;
                    }
                }

                let (sto_id, _) = last_bidi_update.load(atomic::Ordering::Relaxed);
                if update_id != sto_id {
                    // self_to_other did not cause this assign.
                    last_bidi_update.store((sto_id, update_id), atomic::Ordering::Relaxed);
                    let _ = cfg_var.set(v.as_any().downcast_ref::<T>().unwrap().clone());
                }

                true
            }))
            .perm();

        res_var.boxed()
    }
}
