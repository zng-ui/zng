use crate::var::*;

use super::*;

use parking_lot::Mutex;

/// Represents a config source that swap its backend without disconnecting any bound keys.
///
/// Note that the [`CONFIG`] service already uses this type internally.
pub struct SwapConfig {
    cfg: Mutex<Box<dyn AnyConfig>>,
    shared: ConfigVars,

    source_status: BoxedVar<ConfigStatus>,
    status: ArcVar<ConfigStatus>,
    status_binding: VarHandle,
}

impl AnyConfig for SwapConfig {
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

    fn status(&self) -> BoxedVar<ConfigStatus> {
        self.status.read_only().boxed()
    }
}
impl Config for SwapConfig {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared.get_or_bind(key.into(), |key| {
            // not in shared, bind with source json var.

            let default = default();
            let source_var = self.cfg.get_mut().get_raw(
                key.clone(),
                RawConfigValue::serialize(&default).unwrap_or_else(|e| panic!("invalid default value, {e}")),
                false,
            );
            let var = var(RawConfigValue::deserialize(source_var.get()).unwrap_or(default));

            source_var
                .bind_filter_map_bidi(
                    &var,
                    // Raw -> T
                    clmv!(key, |raw| {
                        match RawConfigValue::deserialize(raw.clone()) {
                            Ok(value) => Some(value),
                            Err(e) => {
                                tracing::error!("swap config get({key:?}) error, {e:?}");
                                None
                            }
                        }
                    }),
                    // T -> Raw
                    clmv!(key, source_var, |value| {
                        let _strong_ref = &source_var;

                        match RawConfigValue::serialize(value) {
                            Ok(raw) => Some(raw),
                            Err(e) => {
                                tracing::error!("swap config set({key:?}) error, {e:?}");
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
            shared: ConfigVars::default(),
            source_status: NilConfig.status(),
            status: var(ConfigStatus::Loaded),
            status_binding: VarHandle::dummy(),
        }
    }

    /// Load the config.
    ///
    /// The previous source will be dropped. Note that some variables produced from the previous source
    /// can be reused, do not set `cfg` to a source that will still be alive after `cfg` is dropped.
    pub fn load(&mut self, cfg: impl AnyConfig) {
        self.replace_source(Box::new(cfg))
    }

    fn replace_source(&mut self, source: Box<dyn AnyConfig>) {
        self.source_status = source.status();
        self.status.set(self.source_status.get());
        self.status_binding = self.source_status.bind(&self.status);

        *self.cfg.get_mut() = source; // drop previous source first

        self.shared.rebind(&mut **self.cfg.get_mut());
    }
}
impl Default for SwapConfig {
    fn default() -> Self {
        Self::new()
    }
}
