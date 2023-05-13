use std::path::PathBuf;

use atomic::{Atomic, Ordering};

use crate::{fs_watcher::WATCHER, var::*};

use super::*;

/// Config source that auto syncs with file.
///
/// The [`WATCHER.sync`] is used to synchronize with the file, this type implements the binding
/// for each key.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct SyncConfig<M: ConfigMap> {
    sync_var: ArcVar<M>,
    is_loaded: ArcVar<bool>,
    errors: ArcVar<ConfigErrors>,
    shared: ConfigVars,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let is_loaded = var(false);
        let errors = var(ConfigErrors::default());
        let sync_var = WATCHER.sync(
            file,
            M::empty(),
            clmv!(is_loaded, errors, |r| {
                match (|| M::read(r?))() {
                    Ok(ok) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                        if !is_loaded.get() {
                            is_loaded.set(true);
                        }
                        Some(ok)
                    }
                    Err(e) => {
                        if is_loaded.get() {
                            is_loaded.set(false);
                        }
                        tracing::error!("sync config read error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_read(e)));
                        None
                    }
                }
            }),
            clmv!(is_loaded, errors, |map, w| {
                match (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                        if !is_loaded.get() {
                            is_loaded.set(true);
                        }
                    }
                    Err(e) => {
                        if is_loaded.get() {
                            is_loaded.set(false);
                        }
                        tracing::error!("sync config write error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_write(e)));
                    }
                }
            }),
        );

        Self {
            sync_var,
            errors,
            is_loaded,
            shared: ConfigVars::default(),
        }
    }

    fn get_new_json(
        sync_var: &ArcVar<M>,
        errors: &ArcVar<ConfigErrors>,
        key: ConfigKey,
        default: serde_json::Value,
    ) -> BoxedVar<serde_json::Value> {
        // init var to already present value, or default.
        let var = match sync_var.with(|m| ConfigMap::get_json(m, &key)) {
            Ok(json) => {
                // get ok, clear any entry errors
                if errors.with(|e| e.entry(&key).next().is_some()) {
                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                }

                match json {
                    Some(json) => var(json),
                    None => var(default),
                }
            }
            Err(e) => {
                // get error
                tracing::error!("sync config get({key:?}) error, {e:?}");
                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                var(default)
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        sync_var
            .hook(Box::new(clmv!(errors, key, |map| {
                let update_id = VARS.update_id();
                if update_id == last_update.load(Ordering::Relaxed) {
                    return true;
                }
                last_update.store(update_id, Ordering::Relaxed);
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get_json(&key) {
                        Ok(json) => {
                            // get ok
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }

                            if let Some(json) = json {
                                var.set(json);
                            }
                            // else backend lost entry but did not report as error.
                        }
                        Err(e) => {
                            // get error
                            tracing::error!("sync config get({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        }
                    }
                    // retain hook
                    true
                } else {
                    // entry var dropped, drop hook
                    false
                }
            })))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        var.hook(Box::new(clmv!(errors, |value| {
            let update_id = VARS.update_id();
            if update_id == last_update.load(Ordering::Relaxed) {
                return true;
            }
            last_update.store(update_id, Ordering::Relaxed);
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let json = value.as_any().downcast_ref::<serde_json::Value>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    // set, only if actually changed
                    match ConfigMap::set_json(m, key.clone(), json) {
                        Ok(()) => {
                            // set ok
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(move |e| e.to_mut().clear_entry(&key));
                            }
                        }
                        Err(e) => {
                            // set error
                            tracing::error!("sync config set({key:?}) error, {e:?}");
                            errors.modify(|es| es.to_mut().push(ConfigError::new_set(key, e)));
                        }
                    }
                }));

                // retain hook
                true
            } else {
                // config dropped, drop hook
                false
            }
        })))
        .perm();

        var.boxed()
    }

    fn get_new<T: ConfigValue>(
        sync_var: &ArcVar<M>,
        errors: &ArcVar<ConfigErrors>,
        key: impl Into<ConfigKey>,
        default: impl FnOnce() -> T,
    ) -> BoxedVar<T> {
        // init var to already present value, or default.
        let key = key.into();
        let var = match sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => {
                if errors.with(|e| e.entry(&key).next().is_some()) {
                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                }
                match value {
                    Some(val) => var(val),
                    None => var(default()),
                }
            }
            Err(e) => {
                tracing::error!("sync config get({key:?}) error, {e:?}");
                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                var(default())
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        sync_var
            .hook(Box::new(clmv!(errors, key, |map| {
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get::<T>(&key) {
                        Ok(value) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }

                            if let Some(value) = value {
                                var.set(value);
                            }
                        }
                        Err(e) => {
                            tracing::error!("sync config get({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        }
                    }
                    true
                } else {
                    false
                }
            })))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        var.hook(Box::new(clmv!(errors, |value| {
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let value = value.as_any().downcast_ref::<T>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    match ConfigMap::set(m, key.clone(), value) {
                        Ok(()) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }
                        }
                        Err(e) => {
                            tracing::error!("sync config set({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                        }
                    }
                }));
                true
            } else {
                false
            }
        })))
        .perm();

        var.boxed()
    }
}
impl<M: ConfigMap> AnyConfig for SyncConfig<M> {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors.clone().boxed()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        if shared {
            self.shared
                .get_or_bind(key, |key| Self::get_new_json(&self.sync_var, &self.errors, key.clone(), default))
        } else {
            Self::get_new_json(&self.sync_var, &self.errors, key, default)
        }
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.sync_var.with(|q| q.contains_key(key))
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        self.is_loaded.clone().boxed()
    }
}
impl<M: ConfigMap> Config for SyncConfig<M> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared
            .get_or_bind(key.into(), |key| Self::get_new(&self.sync_var, &self.errors, key.clone(), default))
    }
}
