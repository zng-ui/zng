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

        let fall_to_res = fall_var.bind(&res_var);

        let fall_var = Mutex::new(Some((fall_var, fall_to_res)));
        res_var
            .hook(Box::new(move |v| {
                let mut fall_var = fall_var.lock();
                if let Some((fv, _)) = &*fall_var {
                    if fv.last_update() != VARS.update_id() {
                        // update from assign, disconnect from fallback.
                        *fall_var = None;
                        let _ = cfg_var.set(v.as_any().downcast_ref::<RawConfigValue>().unwrap().clone());
                    } else {
                        // update from fallback
                        return true;
                    }
                } else {
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

        let fall_to_res = fall_var.bind(&res_var);

        let fall_var = Mutex::new(Some((fall_var, fall_to_res)));
        res_var
            .hook(Box::new(move |v| {
                let mut fall_var = fall_var.lock();
                if let Some((fv, _)) = &*fall_var {
                    if fv.last_update() != VARS.update_id() {
                        // update from assign, disconnect from fallback.
                        *fall_var = None;
                    } else {
                        // update from fallback
                        return true;
                    }
                }

                let _ = cfg_var.set(v.as_any().downcast_ref::<T>().unwrap().clone());
                true
            }))
            .perm();

        res_var.boxed()
    }
}
