use std::path::PathBuf;

use atomic::{Atomic, Ordering};

use crate::{crate_util::RunOnDrop, fs_watcher::WATCHER, var::*};

use super::*;

/// Config source that auto syncs with file.
///
/// The [`WATCHER.sync`] is used to synchronize with the file, this type implements the binding
/// for each key.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct SyncConfig<M: ConfigMap> {
    sync_var: ArcVar<M>,
    status: ArcVar<ConfigStatus>,
    errors: ArcVar<ConfigErrors>,
    shared: ConfigVars,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let status = var(ConfigStatus::IDLE);
        let errors = var(ConfigErrors::default());
        let sync_var = WATCHER.sync(
            file,
            M::empty(),
            clmv!(status, errors, |r| {
                status.modify(|s| s.to_mut().insert(ConfigStatus::READ));
                let _end = RunOnDrop::new(|| status.modify(|s| s.to_mut().remove(ConfigStatus::READ)));
                match (|| M::read(r?))() {
                    Ok(ok) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                        Some(ok)
                    }
                    Err(e) => {
                        tracing::error!("sync config read error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_read(e)));
                        None
                    }
                }
            }),
            clmv!(status, errors, |map, w| {
                status.modify(|s| s.to_mut().insert(ConfigStatus::WRITE));
                let _end = RunOnDrop::new(|| status.modify(|s| s.to_mut().remove(ConfigStatus::WRITE)));
                match (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                    }
                    Err(e) => {
                        tracing::error!("sync config write error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_write(e)));
                    }
                }
            }),
        );

        Self {
            sync_var,
            errors,
            status,
            shared: ConfigVars::default(),
        }
    }

    fn get_new_raw(
        sync_var: &ArcVar<M>,
        errors: &ArcVar<ConfigErrors>,
        key: ConfigKey,
        default: RawConfigValue,
    ) -> BoxedVar<RawConfigValue> {
        // init var to already present value, or default.
        let var = match sync_var.with(|m| ConfigMap::get_raw(m, &key)) {
            Ok(raw) => {
                // get ok, clear any entry errors
                if errors.with(|e| e.entry(&key).next().is_some()) {
                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                }

                match raw {
                    Some(raw) => var(raw),
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
                    match map.as_any().downcast_ref::<M>().unwrap().get_raw(&key) {
                        Ok(raw) => {
                            // get ok
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }

                            if let Some(raw) = raw {
                                var.set(raw);
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
                let raw = value.as_any().downcast_ref::<RawConfigValue>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    // set, only if actually changed
                    match ConfigMap::set_raw(m, key.clone(), raw) {
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

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        if shared {
            self.shared
                .get_or_bind(key, |key| Self::get_new_raw(&self.sync_var, &self.errors, key.clone(), default))
        } else {
            Self::get_new_raw(&self.sync_var, &self.errors, key, default)
        }
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.sync_var.with(|q| q.contains_key(key))
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        self.status.clone().boxed()
    }
}
impl<M: ConfigMap> Config for SyncConfig<M> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared
            .get_or_bind(key.into(), |key| Self::get_new(&self.sync_var, &self.errors, key.clone(), default))
    }
}
