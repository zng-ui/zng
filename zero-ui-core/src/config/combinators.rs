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
pub struct FallbackConfig<P, F> {
    primary: Arc<Mutex<P>>,
    fallback: Arc<Mutex<LazyConfig<F>>>,
    copy_to_primary: bool,
}
impl<P: ConfigSource, F: ConfigSource> FallbackConfig<P, F> {
    /// New from primary and fallback, you can use [`ConfigSource::with_fallback`] to build.
    pub fn new(primary: P, fallback: F, copy_to_primary: bool) -> Self {
        Self {
            primary: Arc::new(Mutex::new(primary)),
            fallback: Arc::new(Mutex::new(LazyConfig::new(fallback))),
            copy_to_primary,
        }
    }
}
impl<P: ConfigSource, F: ConfigSource> ConfigSource for FallbackConfig<P, F> {
    fn init(&mut self, observer: AppExtSender<ConfigSourceUpdate>) {
        self.primary.lock().init(observer.clone());
        self.fallback.lock().init(observer);
    }

    fn deinit(&mut self) {
        self.primary.lock().deinit();
        self.fallback.lock().deinit();
    }

    fn read(&mut self, key: ConfigKey) -> BoxedFut<Result<Option<JsonValue>, ConfigError>> {
        let read = self.primary.lock().read(key.clone());
        let primary = self.primary.clone();
        let fallback = self.fallback.clone();
        let copy_to_primary = self.copy_to_primary;
        Box::pin(async move {
            let r = read.await?;
            if r.is_some() {
                Ok(r)
            } else {
                let copy_key = if copy_to_primary { Some(key.clone()) } else { None };

                let read = fallback.lock().read(key);
                let result = read.await;

                if let (Some(key), Ok(Some(value))) = (copy_key, &result) {
                    let write = primary.lock().write(key, value.clone());
                    let _ = write.await;
                }

                result
            }
        })
    }

    fn write(&mut self, key: ConfigKey, value: JsonValue) -> BoxedFut<Result<(), ConfigError>> {
        self.primary.lock().write(key, value)
    }

    fn remove(&mut self, key: ConfigKey) -> BoxedFut<Result<(), ConfigError>> {
        self.primary.lock().remove(key)
    }
}
