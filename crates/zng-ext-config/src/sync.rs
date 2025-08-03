use std::{marker::PhantomData, path::PathBuf};

use zng_clone_move::clmv;
use zng_ext_fs_watcher::WATCHER;
use zng_var::Var;

use super::*;

/// Internal representation of configs.
pub type RawConfigMap = indexmap::IndexMap<ConfigKey, RawConfigValue>;

/// Represents a serializing/encoding backend for [`SyncConfig`].
pub trait SyncConfigBackend: 'static {
    /// Read/deserialize raw config from the file.
    ///
    /// This method runs in unblocked context.
    fn read(file: WatchFile) -> io::Result<RawConfigMap>;

    /// Write/serialize raw config to the file.
    fn write(file: &mut WriteFile, config: &RawConfigMap) -> io::Result<()>;
}

/// Config source that auto syncs with file.
///
/// The [`WATCHER.sync`] is used to synchronize with the file, this type implements the binding
/// for each key.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct SyncConfig<B: SyncConfigBackend> {
    sync_var: Var<RawConfigMap>,
    backend: PhantomData<fn() -> B>,
    status: Var<ConfigStatus>,
}
impl<B: SyncConfigBackend> SyncConfig<B> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let file = file.into();
        let (sync_var, status) = WATCHER.sync_status::<_, _, ConfigStatusError, ConfigStatusError>(
            file,
            RawConfigMap::default(),
            |r| match (|| B::read(r?))() {
                Ok(ok) => Ok(Some(ok)),
                Err(e) => {
                    tracing::error!("sync config read error, {e:?}");
                    Err(vec![Arc::new(e)])
                }
            },
            |map, w| {
                let r = (|| {
                    let mut w = w?;
                    B::write(&mut w, &map)?;
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
            backend: PhantomData,
            status,
        }
    }
}
impl<B: SyncConfigBackend> AnyConfig for SyncConfig<B> {
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> Var<RawConfigValue> {
        // init value
        let current_raw_value = self.sync_var.with(|m| m.get(&key).cloned());
        let value_var = match current_raw_value {
            Some(v) => var(v),
            None => {
                if insert {
                    self.sync_var.modify(clmv!(key, default, |args| {
                        if !args.contains_key(&key) {
                            args.insert(key, default);
                        }
                    }));
                }
                var(default)
            }
        };

        self.sync_var
            .bind_modify_bidi(
                &value_var,
                clmv!(key, |v, m| {
                    if let Some(value) = v.get(&key) {
                        m.set(value.clone());
                    }
                }),
                move |v, m| match m.get(&key) {
                    Some(prev) => {
                        if prev != v {
                            *m.get_mut(&key).unwrap() = v.clone();
                        }
                    }
                    None => {
                        m.insert(key.clone(), v.clone());
                    }
                },
            )
            .perm();

        value_var
    }

    fn contains_key(&mut self, key: ConfigKey) -> Var<bool> {
        self.sync_var.map(move |q| q.contains_key(&key))
    }

    fn status(&self) -> Var<ConfigStatus> {
        self.status.clone()
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        let contains = self.sync_var.with(|q| q.contains_key(key));
        if contains {
            self.sync_var.modify(clmv!(key, |m| {
                if m.contains_key(&key) {
                    m.shift_remove(&key);
                }
            }));
        }
        contains
    }

    fn low_memory(&mut self) {}
}
