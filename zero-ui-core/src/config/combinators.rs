use std::mem;

use parking_lot::Mutex;

use super::*;

/// Config wrapper that only initializes the inner config on the first read/write op.
///
/// Note that [`Config`] and [`ConfigAlt`] already implement this internally, this type is useful only as
/// a building block of combinator config sources.
pub struct LazyConfig<C> {
    cfg: C,
    update: Option<AppExtSender<ConfigSourceUpdate>>,
    inited: bool,
}
impl<C: ConfigSource> LazyConfig<C> {
    /// New lazy config.
    pub fn new(cfg: C) -> Self {
        LazyConfig {
            cfg,
            update: None,
            inited: false,
        }
    }

    fn init_cfg(&mut self) {
        self.cfg.init(self.update.take().expect("not inited"));
    }
}
impl<C: ConfigSource> ConfigSource for LazyConfig<C> {
    fn init(&mut self, observer: AppExtSender<ConfigSourceUpdate>) {
        self.update = Some(observer);
    }

    fn deinit(&mut self) {
        if mem::take(&mut self.inited) {
            self.cfg.deinit();
        }
        self.update = None;
    }

    fn read(&mut self, key: ConfigKey) -> BoxedFut<Result<Option<JsonValue>, ConfigError>> {
        self.init_cfg();
        self.cfg.read(key)
    }

    fn write(&mut self, key: ConfigKey, value: JsonValue) -> BoxedFut<Result<(), ConfigError>> {
        self.init_cfg();
        self.cfg.write(key, value)
    }

    fn remove(&mut self, key: ConfigKey) -> BoxedFut<Result<(), ConfigError>> {
        self.init_cfg();
        self.cfg.remove(key)
    }
}

/// Represents a config source that reads from a fallback source if a key is not found.
///
/// Reads from the fallback are automatically written on the primary source.
pub struct FallbackConfig<P, F> {
    primary: P,
    fallback: Arc<Mutex<LazyConfig<F>>>,
}
impl<P: ConfigSource, F: ConfigSource> FallbackConfig<P, F> {
    /// New from primary and fallback, you can use [`ConfigSource::with_fallback`] to build.
    pub fn new(primary: P, fallback: F) -> Self {
        Self {
            primary,
            fallback: Arc::new(Mutex::new(LazyConfig::new(fallback))),
        }
    }
}
impl<P: ConfigSource, F: ConfigSource> ConfigSource for FallbackConfig<P, F> {
    fn init(&mut self, observer: AppExtSender<ConfigSourceUpdate>) {
        self.primary.init(observer.clone());
        self.fallback.lock().init(observer);
    }

    fn deinit(&mut self) {
        self.primary.deinit();
        self.fallback.lock().deinit();
    }

    fn read(&mut self, key: ConfigKey) -> BoxedFut<Result<Option<JsonValue>, ConfigError>> {
        let prim = self.primary.read(key.clone());
        let fallback = self.fallback.clone();
        Box::pin(async move {
            let r = prim.await?;
            if r.is_some() {
                Ok(r)
            } else {
                let f = fallback.lock().read(key);
                f.await
            }
        })
    }

    fn write(&mut self, key: ConfigKey, value: JsonValue) -> BoxedFut<Result<(), ConfigError>> {
        self.primary.write(key, value)
    }

    fn remove(&mut self, key: ConfigKey) -> BoxedFut<Result<(), ConfigError>> {
        self.primary.remove(key)
    }
}
