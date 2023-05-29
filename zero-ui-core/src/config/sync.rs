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
    status: ArcVar<ConfigStatus>,
    shared: ConfigVars,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let status = var(ConfigStatus::Loading);
        let sync_var = WATCHER.sync(
            file,
            M::empty(),
            clmv!(status, |r| {
                status.set_ne(ConfigStatus::Loading);

                match (|| M::read(r?))() {
                    Ok(ok) => {
                        status.set_ne(ConfigStatus::Loaded);
                        Some(ok)
                    }
                    Err(e) => {
                        tracing::error!("sync config read error, {e:?}");
                        status.set(ConfigStatus::LoadErrors(vec![Arc::new(e)]));
                        None
                    }
                }
            }),
            clmv!(status, |map, w| {
                status.set_ne(ConfigStatus::Saving);

                match (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        status.set_ne(ConfigStatus::Loaded);
                    }
                    Err(e) => {
                        tracing::error!("sync config write error, {e:?}");
                        status.set(ConfigStatus::SaveErrors(vec![Arc::new(e)]));
                    }
                }
            }),
        );

        Self {
            sync_var,
            status,
            shared: ConfigVars::default(),
        }
    }

    fn get_new_raw(sync_var: &ArcVar<M>, key: ConfigKey, default: RawConfigValue) -> BoxedVar<RawConfigValue> {
        // init var to already present value, or default.
        let var = match sync_var.with(|m| ConfigMap::get_raw(m, &key)) {
            Ok(raw) => {
                // get ok
                match raw {
                    Some(raw) => var(raw),
                    None => var(default),
                }
            }
            Err(e) => {
                // get error
                tracing::error!("sync config get({key:?}) error, {e:?}");
                var(default)
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        sync_var
            .hook(Box::new(clmv!(key, |map| {
                let update_id = VARS.update_id();
                if update_id == last_update.load(Ordering::Relaxed) {
                    return true;
                }
                last_update.store(update_id, Ordering::Relaxed);
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get_raw(&key) {
                        Ok(raw) => {
                            // get ok
                            if let Some(raw) = raw {
                                var.set(raw);
                            }
                            // else backend lost entry but did not report as error.
                        }
                        Err(e) => {
                            // get error
                            tracing::error!("sync config get({key:?}) error, {e:?}");
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
        var.hook(Box::new(clmv!(|value| {
            let update_id = VARS.update_id();
            if update_id == last_update.load(Ordering::Relaxed) {
                return true;
            }
            last_update.store(update_id, Ordering::Relaxed);
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let raw = value.as_any().downcast_ref::<RawConfigValue>().unwrap().clone();
                sync_var.modify(clmv!(key, |m| {
                    // set, only if actually changed
                    match ConfigMap::set_raw(m, key.clone(), raw) {
                        Ok(()) => {
                            // set ok
                        }
                        Err(e) => {
                            // set error
                            tracing::error!("sync config set({key:?}) error, {e:?}");
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

    fn get_new<T: ConfigValue>(sync_var: &ArcVar<M>, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        // init var to already present value, or default.
        let key = key.into();
        let var = match sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => match value {
                Some(val) => var(val),
                None => var(default()),
            },
            Err(e) => {
                tracing::error!("sync config get({key:?}) error, {e:?}");
                var(default())
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        sync_var
            .hook(Box::new(clmv!(key, |map| {
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get::<T>(&key) {
                        Ok(value) => {
                            if let Some(value) = value {
                                var.set(value);
                            }
                        }
                        Err(e) => {
                            tracing::error!("sync config get({key:?}) error, {e:?}");
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
        var.hook(Box::new(clmv!(|value| {
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let value = value.as_any().downcast_ref::<T>().unwrap().clone();
                sync_var.modify(clmv!(key, |m| {
                    match ConfigMap::set(m, key.clone(), value) {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::error!("sync config set({key:?}) error, {e:?}");
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        if shared {
            self.shared
                .get_or_bind(key, |key| Self::get_new_raw(&self.sync_var, key.clone(), default))
        } else {
            Self::get_new_raw(&self.sync_var, key, default)
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
            .get_or_bind(key.into(), |key| Self::get_new(&self.sync_var, key.clone(), default))
    }
}
