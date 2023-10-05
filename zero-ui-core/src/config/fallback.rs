use std::sync::atomic::AtomicBool;

use parking_lot::Mutex;

use crate::var::*;

use super::*;

/// Reset controls of a [`FallbackConfig`].
pub trait FallbackConfigReset: AnyConfig + Sync {
    /// Removes the `key` from the config and updates all active config variables back to
    /// the fallback value. Note that if you assign the config variable the key will be re-inserted on the config.
    ///
    /// The `FallbackConfig` type is an `Arc` internally, so you can keep a clone of it and call
    /// reset on this clone to reset the config moved inside [`CONFIG`] or another combinator.
    fn reset(&self, key: &ConfigKey);

    /// Returns a read-only var that is `true` when the `key` has an entry in the read-write config.
    fn can_reset(&self, key: ConfigKey) -> BoxedVar<bool>;

    /// Clone a reference to the config.
    fn clone_boxed(&self) -> Box<dyn FallbackConfigReset>;
}
impl Clone for Box<dyn FallbackConfigReset> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

/// Represents a copy-on-write config source that wraps two other sources, the read-write config and a read-only fallback config.
///
/// The config variables are connected to both sources, if the read-write config is not set the var will update with the
/// fallback config, if it is set it will sync with the read-write config.
pub struct FallbackConfig<S: Config, F: Config>(Arc<Mutex<FallbackConfigData<S, F>>>);
impl<S: Config, F: Config> FallbackConfig<S, F> {
    /// New from the read-write config and read-only fallback.
    pub fn new(config: S, fallback: F) -> Self {
        Self(Arc::new(Mutex::new(FallbackConfigData {
            fallback,
            config,
            vars: HashMap::new(),
        })))
    }

    /// Removes the `key` from the config and updates all active config variables back to
    /// the fallback value. Note that if you assign the config variable the key will be re-inserted on the config.
    ///
    /// The `FallbackConfig` type is an `Arc` internally, so you can keep a clone of it and call
    /// reset on this clone to reset the config moved inside [`CONFIG`] or another combinator.
    pub fn reset(&self, key: &ConfigKey) {
        FallbackConfigData::reset(&self.0, key);
    }

    /// Returns a read-only var that is `true` when the `key` has an entry in the read-write config.
    pub fn can_reset(&self, key: ConfigKey) -> BoxedVar<bool> {
        self.0.lock().config.contains_key(key)
    }
}
impl<S: Config, F: Config> Clone for FallbackConfig<S, F> {
    fn clone(&self) -> Self {
        FallbackConfig(Arc::clone(&self.0))
    }
}
impl<S: Config, F: Config> FallbackConfigReset for FallbackConfig<S, F> {
    fn reset(&self, key: &ConfigKey) {
        self.reset(key)
    }

    fn can_reset(&self, key: ConfigKey) -> BoxedVar<bool> {
        self.can_reset(key)
    }

    fn clone_boxed(&self) -> Box<dyn FallbackConfigReset> {
        Box::new(self.clone())
    }
}
impl<S: Config, F: Config> AnyConfig for FallbackConfig<S, F> {
    fn status(&self) -> BoxedVar<ConfigStatus> {
        let d = self.0.lock();
        merge_var!(d.fallback.status(), d.config.status(), |fallback, over| {
            ConfigStatus::merge_status([fallback.clone(), over.clone()].into_iter())
        })
        .boxed()
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        let mut d = self.0.lock();
        let d = &mut *d;

        if d.vars.len() > 1000 {
            d.vars.retain(|_, v| v.retain());
        }

        let entry = d.vars.entry(key.clone()).or_default();
        if let Some(res) = entry.res.upgrade() {
            return res.boxed();
        }

        let cfg_contains_key_var = d.config.contains_key(key.clone());
        let is_already_set = cfg_contains_key_var.get();

        let cfg_var = d.config.get_raw(key.clone(), default.clone(), shared);

        let fall_var = d.fallback.get_raw(key, default, shared);

        let res_var = var(if is_already_set { cfg_var.get() } else { fall_var.get() });
        entry.res = res_var.downgrade();

        // based on `Var::bind_bidi` code.
        let binding_tag = BindMapBidiTag::new_unique();

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct ResetTag;

        // fallback->res binding can re-enable on reset.
        let fall_res_enabled = Arc::new(AtomicBool::new(!is_already_set));

        // bind cfg_var -> res_var, handles potential bidi binding
        let weak_res_var = res_var.downgrade();
        cfg_var
            .hook(Box::new(clmv!(fall_res_enabled, |args| {
                if let Some(res_var) = weak_res_var.upgrade() {
                    let is_from_other = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
                    if !is_from_other {
                        // res_var did not cause this assign, propagate.

                        // disable fallback->res binding
                        fall_res_enabled.store(false, atomic::Ordering::Relaxed);

                        let value = args.downcast_value::<RawConfigValue>().unwrap().clone();

                        res_var.modify(move |v| {
                            if v.as_ref() != &value {
                                v.set(value);
                                v.push_tag(binding_tag);
                            }
                        });
                    }

                    true
                } else {
                    false
                }
            })))
            .perm();

        // bind fallback_var -> res_var.
        let weak_res_var = res_var.downgrade();
        fall_var
            .hook(Box::new(clmv!(fall_res_enabled, |args| {
                if let Some(res_var) = weak_res_var.upgrade() {
                    if fall_res_enabled.load(atomic::Ordering::Relaxed) {
                        let value = args.downcast_value::<RawConfigValue>().unwrap().clone();
                        res_var.modify(move |v| {
                            if v.as_ref() != &value {
                                v.set(value);
                                // don't set cfg_var from fallback update.
                                v.push_tag(binding_tag);
                            }
                        });
                    }

                    true
                } else {
                    false
                }
            })))
            .perm();

        // bind cfg_contains_key_var to restore sync with fallback_var when cannot sync with cfg_var anymore.
        let weak_fall_var = fall_var.downgrade();
        let weak_res_var = res_var.downgrade();
        cfg_contains_key_var
            .hook(Box::new(clmv!(fall_res_enabled, |args| {
                if let Some(res_var) = weak_res_var.upgrade() {
                    // still alive
                    let can_reset = args.downcast_value::<bool>().unwrap();
                    if !can_reset && !fall_res_enabled.load(atomic::Ordering::Relaxed) {
                        // cfg_var removed and we are sync with it.
                        if let Some(fall_var) = weak_fall_var.upgrade() {
                            // still alive, sync with fallback_var.
                            let fall_value = fall_var.get();
                            res_var.modify(move |vm| {
                                vm.set(fall_value);
                                vm.push_tag(ResetTag); // res_var will reset
                            });
                        } else {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            })))
            .perm();

        // map res_var -> cfg_var, manages fallback binding.
        res_var
            .hook(Box::new(move |args| {
                let _strong_ref = (&fall_var, &cfg_contains_key_var);

                let is_from_other = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
                if !is_from_other {
                    // not set from cfg/fallback

                    let is_reset = args.downcast_tags::<ResetTag>().next().is_some();
                    if is_reset {
                        fall_res_enabled.store(true, atomic::Ordering::Relaxed);
                    } else {
                        fall_res_enabled.store(false, atomic::Ordering::Relaxed);
                        let value = args.downcast_value::<RawConfigValue>().unwrap().clone();
                        let _ = cfg_var.modify(move |v| {
                            if v.as_ref() != &value {
                                v.set(value);
                                v.push_tag(binding_tag);
                            }
                        });
                    }
                }

                true
            }))
            .perm();

        res_var.boxed()
    }

    fn contains_key(&mut self, key: ConfigKey) -> BoxedVar<bool> {
        let mut d = self.0.lock();
        merge_var!(d.fallback.contains_key(key.clone()), d.config.contains_key(key), |&a, &b| a || b).boxed()
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        let mut d = self.0.lock();
        d.fallback.remove(key) || d.config.remove(key)
    }
}
impl<S: Config, F: Config> Config for FallbackConfig<S, F> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let default = default();
        self.get_raw(key.into(), RawConfigValue::serialize(&default).unwrap(), true)
            .filter_map_bidi(
                |raw| raw.clone().deserialize().ok(),
                |v| RawConfigValue::serialize(v).ok(),
                move || default.clone(),
            )
            .boxed()
    }
}

#[derive(Default)]
struct VarEntry {
    res: WeakArcVar<RawConfigValue>,
}
impl VarEntry {
    fn retain(&self) -> bool {
        self.res.strong_count() > 0
    }
}

struct FallbackConfigData<S: Config, F: Config> {
    fallback: F,
    config: S,

    vars: HashMap<ConfigKey, VarEntry>,
}
impl<S: Config, F: Config> FallbackConfigData<S, F> {
    fn reset(c: &Arc<Mutex<Self>>, key: &ConfigKey) {
        let mut d = c.lock();
        let d = &mut *d;

        d.vars.retain(|_, v| v.retain());

        // Just remove, we already bind with `config.contains_key` and will
        // reset when it changes to `false`.
        d.config.remove(key);
    }
}
