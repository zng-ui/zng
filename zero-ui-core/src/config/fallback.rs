use std::sync::atomic::AtomicBool;

use parking_lot::Mutex;

use crate::var::*;

use super::*;

struct FallbackConfigData<S: Config, F: Config> {
    fallback: F,
    config: S,

    vars: HashMap<ConfigKey, WeakArcVar<RawConfigValue>>,
}
impl<S: Config, F: Config> FallbackConfigData<S, F> {
    fn reset(c: &Arc<Mutex<Self>>, key: &ConfigKey) {
        let mut d = c.lock();
        let d = &mut *d;

        if d.vars.len() > 500 {
            d.vars.retain(|_, v| v.strong_count() > 0);
        }

        if d.config.contains_key(key) {
            // need to remove

            if let Some(res_wk) = d.vars.get(key) {
                if let Some(res) = res_wk.upgrade() {
                    // fallback config var is active, set it to fallback without
                    // propagating the value to d.config.

                    let fallback_value = d
                        .fallback
                        .get_raw(key.clone(), RawConfigValue(serde_json::Value::Null), false)
                        .get();

                    res.modify(move |v| {
                        v.set(fallback_value);
                        v.push_tag(ResetTag);
                    });
                } else {
                    d.vars.remove(key);
                }
            }
            d.config.remove(key);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResetTag;

/// Represents a config source that is read and written too, when a key is not present in the source
/// the fallback variable is used, but if that variable is modified the key is inserted in the primary config.
pub struct FallbackConfig<S: Config, F: Config> {
    data: Arc<Mutex<FallbackConfigData<S, F>>>,
}
impl<S: Config, F: Config> FallbackConfig<S, F> {
    /// New config.
    pub fn new(config: S, fallback: F) -> Self {
        Self {
            data: Arc::new(Mutex::new(FallbackConfigData {
                fallback,
                config,
                vars: HashMap::new(),
            })),
        }
    }

    /// Removes the `key` from the config and updates all active config variables back to
    /// the fallback value. Note that if you assign the config variable the key will be re-inserted on the config.
    pub fn reset(&mut self, key: &ConfigKey) {
        FallbackConfigData::reset(&self.data, key);
    }
}
impl<S: Config, F: Config> AnyConfig for FallbackConfig<S, F> {
    fn status(&self) -> BoxedVar<ConfigStatus> {
        let d = self.data.lock();
        merge_var!(d.fallback.status(), d.config.status(), |fallback, over| {
            ConfigStatus::merge_status([fallback.clone(), over.clone()].into_iter())
        })
        .boxed()
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        let mut d = self.data.lock();
        let d = &mut *d;

        if d.vars.len() > 1000 {
            d.vars.retain(|_, v| v.strong_count() > 0);
        }

        let key = match d.vars.entry(key) {
            hash_map::Entry::Occupied(e) => {
                if let Some(res) = e.get().upgrade() {
                    return res.boxed();
                } else {
                    e.remove_entry().0
                }
            }
            hash_map::Entry::Vacant(e) => e.into_key(),
        };

        let is_already_set = d.config.contains_key(&key);

        let cfg_var = d.config.get_raw(key.clone(), default.clone(), shared);

        let fall_var = d.fallback.get_raw(key.clone(), default, shared);

        let res_var = var(if is_already_set { cfg_var.get() } else { fall_var.get() });

        // based on `Var::bind_bidi` code.
        let binding_tag = BindMapBidiTag::new_unique();
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

        // map res_var -> cfg_var, manages fallback binding.
        res_var
            .hook(Box::new(move |args| {
                let _own = &fall_var;

                let is_from_other = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
                if !is_from_other {
                    // not set from cfg/fallback

                    let is_reset = args.downcast_tags::<ResetTag>().next().is_some();
                    if is_reset {
                        fall_res_enabled.store(true, atomic::Ordering::Relaxed);
                    } else {
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

        d.vars.insert(key, res_var.downgrade());

        res_var.boxed()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        let d = self.data.lock();
        d.fallback.contains_key(key) || d.config.contains_key(key)
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        let mut d = self.data.lock();
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
