use std::{path::PathBuf, sync::atomic::AtomicBool};

use atomic::{Atomic, Ordering};
use zng_clone_move::clmv;
use zng_ext_fs_watcher::WATCHER;
use zng_var::{ReadOnlyArcVar, VARS, VarUpdateId};

use super::*;

/// Config source that auto syncs with file.
///
/// The [`WATCHER.sync`] is used to synchronize with the file, this type implements the binding
/// for each key.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct SyncConfig<M: ConfigMap> {
    sync_var: ArcVar<M>,
    status: ReadOnlyArcVar<ConfigStatus>,
    shared: ConfigVars,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let (sync_var, status) = WATCHER.sync_status::<_, _, ConfigStatusError, ConfigStatusError>(
            file,
            M::empty(),
            |r| match (|| M::read(r?))() {
                Ok(ok) => Ok(Some(ok)),
                Err(e) => {
                    tracing::error!("sync config read error, {e:?}");
                    Err(vec![Arc::new(e)])
                }
            },
            |map, w| {
                let r = (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })();
                match r {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        tracing::error!("sync config write error, {e:?}");
                        Err(vec![Arc::new(e)])
                    }
                }
            },
        );

        Self {
            sync_var,
            status,
            shared: ConfigVars::default(),
        }
    }

    fn get_new_raw(sync_var: &ArcVar<M>, key: ConfigKey, default: RawConfigValue, insert: bool) -> BoxedVar<RawConfigValue> {
        // init var to already present value, or default.
        let mut used_default = false;
        let var = match sync_var.with(|m| ConfigMap::get_raw(m, &key)) {
            Ok(raw) => {
                // get ok
                match raw {
                    Some(raw) => var(raw),
                    None => {
                        used_default = true;
                        var(default)
                    }
                }
            }
            Err(e) => {
                // get error
                tracing::error!("sync config get({key:?}) error, {e:?}");
                used_default = true;
                var(default)
            }
        };

        if insert && used_default {
            var.update();
        }

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        let request_update = AtomicBool::new(used_default);
        sync_var
            .hook(clmv!(key, |map| {
                let update_id = VARS.update_id();
                if update_id == last_update.load(Ordering::Relaxed) {
                    return true;
                }
                last_update.store(update_id, Ordering::Relaxed);
                if let Some(var) = wk_var.upgrade() {
                    match map.value().get_raw(&key) {
                        Ok(raw) => {
                            // get ok
                            if let Some(raw) = raw {
                                var.set(raw);
                                if request_update.swap(false, Ordering::Relaxed) {
                                    // restored after reset, var can already have the value,
                                    // but upstream bindings are stale, cause an update.
                                    var.update();
                                }
                            } else {
                                // backend lost entry but did not report as error, probably a reset.
                                request_update.store(true, Ordering::Relaxed);
                            }
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
            }))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        var.hook(clmv!(|value| {
            let update_id = VARS.update_id();
            if update_id == last_update.load(Ordering::Relaxed) {
                return true;
            }
            last_update.store(update_id, Ordering::Relaxed);
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let raw = value.value().clone();
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
        }))
        .perm();

        var.boxed()
    }

    fn get_new<T: ConfigValue>(sync_var: &ArcVar<M>, key: impl Into<ConfigKey>, default: T, insert: bool) -> BoxedVar<T> {
        // init var to already present value, or default.
        let key = key.into();
        let mut used_default = false;
        let var = match sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => match value {
                Some(val) => var(val),
                None => {
                    used_default = true;
                    var(default)
                }
            },
            Err(e) => {
                tracing::error!("sync config get({key:?}) error, {e:?}");
                used_default = true;
                var(default)
            }
        };

        if insert && used_default {
            var.update();
        }

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        sync_var
            .hook(clmv!(key, |map| {
                if let Some(var) = wk_var.upgrade() {
                    match map.value().get::<T>(&key) {
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
            }))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        var.hook(clmv!(|value| {
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let value = value.value().clone();
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
        }))
        .perm();

        var.boxed()
    }
}
impl<M: ConfigMap> AnyConfig for SyncConfig<M> {
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool, shared: bool) -> BoxedVar<RawConfigValue> {
        if shared {
            self.shared
                .get_or_bind(key, |key| Self::get_new_raw(&self.sync_var, key.clone(), default, insert))
        } else {
            Self::get_new_raw(&self.sync_var, key, default, insert)
        }
    }

    fn contains_key(&mut self, key: ConfigKey) -> BoxedVar<bool> {
        self.sync_var.map(move |q| q.contains_key(&key)).boxed()
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        self.status.clone().boxed()
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        let contains = self.sync_var.with(|q| q.contains_key(key));
        if contains {
            self.sync_var.modify(clmv!(key, |m| {
                ConfigMap::remove(m, &key);
            }));
        }
        contains
    }

    fn low_memory(&mut self) {
        self.shared.low_memory();
    }
}
impl<M: ConfigMap> Config for SyncConfig<M> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: T, insert: bool) -> BoxedVar<T> {
        self.shared
            .get_or_bind(key.into(), |key| Self::get_new(&self.sync_var, key.clone(), default, insert))
    }
}
