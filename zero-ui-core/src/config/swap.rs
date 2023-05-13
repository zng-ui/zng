use crate::var::*;

use super::*;

use parking_lot::Mutex;

/// Represents a config source that swap its backend without disconnecting any bound keys.
///
/// Note that the [`CONFIG`] service already uses this type internally.
pub struct SwapConfig {
    cfg: Mutex<Box<dyn AnyConfig>>,
    shared: ConfigVars,

    is_loaded: ArcVar<bool>,
    is_loaded_binding: VarHandle,
    errors: ArcVar<ConfigErrors>,
    errors_binding: VarHandle,
}

impl AnyConfig for SwapConfig {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors.clone().boxed()
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        if shared {
            self.shared
                .get_or_bind(key, |key| self.cfg.get_mut().get_raw(key.clone(), default, false))
        } else {
            self.cfg.get_mut().get_raw(key, default, false)
        }
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.cfg.lock().contains_key(key)
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        self.is_loaded.clone().boxed()
    }
}
impl Config for SwapConfig {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared.get_or_bind(key.into(), |key| {
            // not in shared, bind with source json var.

            let default = default();
            let source_var = self
                .cfg
                .get_mut()
                .get_raw(key.clone(), RawConfigValue::serialize(&default).unwrap(), false);
            let var = var(RawConfigValue::deserialize(source_var.get()).unwrap_or(default));

            let errors = &self.errors;

            source_var
                .bind_filter_map_bidi(
                    &var,
                    // Raw -> T
                    clmv!(key, errors, |raw| {
                        match RawConfigValue::deserialize(raw.clone()) {
                            Ok(value) => {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                                Some(value)
                            }
                            Err(e) => {
                                tracing::error!("swap config get({key:?}) error, {e:?}");
                                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                                None
                            }
                        }
                    }),
                    // T -> Raw
                    clmv!(key, errors, source_var, |value| {
                        let _strong_ref = &source_var;

                        match RawConfigValue::serialize(value) {
                            Ok(raw) => {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                                Some(raw)
                            }
                            Err(e) => {
                                tracing::error!("swap config set({key:?}) error, {e:?}");
                                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                                None
                            }
                        }
                    }),
                )
                .perm();

            var.boxed()
        })
    }
}
impl SwapConfig {
    /// New with [`NilConfig`] backend.
    pub fn new() -> Self {
        Self {
            cfg: Mutex::new(Box::new(NilConfig)),
            errors: var(ConfigErrors::default()),
            shared: ConfigVars::default(),
            is_loaded: var(true), // nil is loaded
            is_loaded_binding: VarHandle::dummy(),
            errors_binding: VarHandle::dummy(),
        }
    }

    /// Load the config.
    pub fn load(&mut self, cfg: impl AnyConfig) {
        self.replace_source(Box::new(cfg))
    }

    fn replace_source(&mut self, mut source: Box<dyn AnyConfig>) {
        let source_errors = source.errors();
        self.errors.set(source_errors.get());
        self.errors_binding = source_errors.bind(&self.errors);

        let source_is_loaded = source.is_loaded();
        self.is_loaded.set(source_is_loaded.get());
        self.is_loaded_binding = source_is_loaded.bind(&self.is_loaded);

        self.shared.rebind(&self.errors, &mut *source);

        *self.cfg.get_mut() = source;
    }
}
impl Default for SwapConfig {
    fn default() -> Self {
        Self::new()
    }
}
