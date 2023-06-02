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
        self.shared.contains_key(key) || self.cfg.lock().contains_key(key)
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
    /// The previous source will be dropped and all active config variables are set and rebound to the new config.
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

#[cfg(test)]
mod tests {
    use crate::app::App;

    use super::*;

    #[test]
    fn swap_config_in_memory() {
        let mut app = App::default().run_headless(false);

        let mut cfg = SwapConfig::new();

        let v = cfg.get("key", || true);
        assert!(v.get());
        v.set(false).unwrap();
        app.update(false).assert_wait();

        let v2 = cfg.get("key", || true);
        assert!(!v2.get() && !v.get());
        assert_eq!(v.var_ptr(), v2.var_ptr());
    }

    #[test]
    fn swap_config_swap() {
        let mut app = App::default().run_headless(false);

        let mut inner1 = TestConfig::default();
        let c1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        c1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let c1 = test.get("key", || 0);

        assert_eq!(32, c1.get());
    }

    #[test]
    fn swap_config_swap_load() {
        let mut app = App::default().run_headless(false);

        let mut inner1 = TestConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let cfg = test.get("key", || 0);

        assert_eq!(32, cfg.get());

        let mut inner2 = TestConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        inner_v2.set(RawConfigValue::serialize(42).unwrap()).unwrap();
        app.update(false).assert_wait();

        test.replace_source(Box::new(inner2));
        app.update(false).assert_wait();

        assert_eq!(42, cfg.get());
    }

    #[test]
    fn swap_config_swap_load_delayed() {
        let mut app = App::default().run_headless(false);

        let mut inner1 = TestConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let cfg = test.get("key", || 0);

        assert_eq!(32, cfg.get());

        let mut inner2 = TestConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        app.update(false).assert_wait();

        test.replace_source(Box::new(inner2));
        app.update(false).assert_wait();

        assert_eq!(0, cfg.get());

        inner_v2.set(RawConfigValue::serialize(42).unwrap()).unwrap();
        app.update(false).assert_wait();
        assert_eq!(42, cfg.get());
    }

    #[test]
    fn swap_config_swap_fallback_delayed() {
        let mut app = App::default().run_headless(false);

        let mut fallback = TestConfig::default();
        fallback
            .get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true)
            .set(RawConfigValue::serialize(100).unwrap())
            .unwrap();

        let mut inner1 = TestConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(FallbackConfig::new(inner1, fallback)));

        let cfg = test.get("key", || -1);

        assert_eq!(32, cfg.get());

        let mut fallback = TestConfig::default();
        fallback
            .get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true)
            .set(RawConfigValue::serialize(100).unwrap())
            .unwrap();
        let mut inner2 = TestConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), true);
        app.update(false).assert_wait();

        test.replace_source(Box::new(FallbackConfig::new(inner2, fallback)));
        app.update(false).assert_wait();

        assert_eq!(0, cfg.get());

        inner_v2.set(RawConfigValue::serialize(42).unwrap()).unwrap();
        app.update(false).assert_wait();
        assert_eq!(42, cfg.get());
    }

    #[derive(Default)]
    struct TestConfig(SwapConfig);

    impl AnyConfig for TestConfig {
        fn status(&self) -> BoxedVar<ConfigStatus> {
            self.0.status()
        }

        fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, _: bool) -> BoxedVar<RawConfigValue> {
            self.0.get_raw(key, default, true)
        }

        fn contains_key(&self, key: &ConfigKey) -> bool {
            self.0.contains_key(key)
        }
    }

    impl Config for TestConfig {
        fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
            self.0.get(key, default)
        }
    }
}
