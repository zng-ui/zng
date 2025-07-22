use super::*;

use crate::task::parking_lot::Mutex;
use zng_var::VarHandle;

/// Represents a config source that can swap its backing config source without disconnecting any bound keys.
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> BoxedVar<RawConfigValue> {
        self.shared
            .get_or_bind(key, |key| self.cfg.get_mut().get_raw(key.clone(), default, insert))
    }

    fn contains_key(&mut self, key: ConfigKey) -> BoxedVar<bool> {
        self.shared
            .get_or_bind_contains(key, |key| self.cfg.get_mut().contains_key(key.clone()))
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        self.status.read_only().boxed()
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        self.cfg.get_mut().remove(key)
    }

    fn low_memory(&mut self) {
        self.cfg.get_mut().low_memory();
        self.shared.low_memory();
    }
}
impl SwapConfig {
    /// New with [`MemoryConfig`] backend.
    pub fn new() -> Self {
        Self {
            cfg: Mutex::new(Box::<MemoryConfig>::default()),
            shared: ConfigVars::default(),
            source_status: LocalVar(ConfigStatus::Loaded).boxed(),
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
        self.status.set_from(&self.source_status);
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
    use zng_app::APP;
    use zng_ext_fs_watcher::FsWatcherManager;

    use super::*;

    #[test]
    fn swap_config_in_memory() {
        let mut app = APP
            .minimal()
            .extend(FsWatcherManager::default())
            .extend(ConfigManager::default())
            .run_headless(false);

        let mut cfg = SwapConfig::new();

        let raw_true = RawConfigValue::serialize(true).unwrap();
        let raw_false = RawConfigValue::serialize(false).unwrap();

        let v = cfg.get_raw("key".into(), raw_true.clone(), false);
        assert_eq!(v.get(), raw_true);
        v.set(raw_false.clone()).unwrap();
        app.update(false).assert_wait();

        let v2 = cfg.get_raw("key".into(), raw_true, false);
        assert!(v2.get() == raw_false && v.get() == raw_false);
        assert_eq!(v.var_ptr(), v2.var_ptr());
    }

    #[test]
    fn swap_config_swap() {
        let mut app = APP
            .minimal()
            .extend(FsWatcherManager::default())
            .extend(ConfigManager::default())
            .run_headless(false);

        let mut inner1 = MemoryConfig::default();
        let c1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        c1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let c1 = test.get("key", 0, false);

        assert_eq!(32, c1.get());
    }

    #[test]
    fn swap_config_swap_load() {
        let mut app = APP
            .minimal()
            .extend(FsWatcherManager::default())
            .extend(ConfigManager::default())
            .run_headless(false);

        let mut inner1 = MemoryConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let cfg = test.get("key", 0, false);

        assert_eq!(32, cfg.get());

        let mut inner2 = MemoryConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        inner_v2.set(RawConfigValue::serialize(42).unwrap()).unwrap();
        app.update(false).assert_wait();

        test.replace_source(Box::new(inner2));
        app.update(false).assert_wait();

        assert_eq!(42, cfg.get());
    }

    #[test]
    fn swap_config_swap_load_delayed() {
        let mut app = APP
            .minimal()
            .extend(FsWatcherManager::default())
            .extend(ConfigManager::default())
            .run_headless(false);

        let mut inner1 = MemoryConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(inner1));

        let cfg = test.get("key", 0, false);

        assert_eq!(32, cfg.get());

        let mut inner2 = MemoryConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
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
        let mut app = APP
            .minimal()
            .extend(FsWatcherManager::default())
            .extend(ConfigManager::default())
            .run_headless(false);

        let mut fallback = MemoryConfig::default();
        fallback
            .get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false)
            .set(RawConfigValue::serialize(100).unwrap())
            .unwrap();

        let mut inner1 = MemoryConfig::default();
        let inner_v1 = inner1.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        inner_v1.set(RawConfigValue::serialize(32).unwrap()).unwrap();
        app.update(false).assert_wait();

        let mut test = SwapConfig::new();
        test.replace_source(Box::new(FallbackConfig::new(inner1, fallback)));

        let cfg = test.get("key", -1, false);

        assert_eq!(32, cfg.get());

        let mut fallback = MemoryConfig::default();
        fallback
            .get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false)
            .set(RawConfigValue::serialize(100).unwrap())
            .unwrap();
        let mut inner2 = MemoryConfig::default();
        let inner_v2 = inner2.get_raw("key".into(), RawConfigValue::serialize(0).unwrap(), false);
        app.update(false).assert_wait();

        test.replace_source(Box::new(FallbackConfig::new(inner2, fallback)));
        app.update(false).assert_wait();

        assert_eq!(0, cfg.get());

        inner_v2.set(RawConfigValue::serialize(42).unwrap()).unwrap();
        app.update(false).assert_wait();
        assert_eq!(42, cfg.get());
    }
}
