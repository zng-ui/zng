use std::{marker::PhantomData, path::PathBuf};

use zng_clone_move::clmv;
use zng_ext_fs_watcher::WATCHER;
use zng_var::{ReadOnlyArcVar, types::SourceVarTag};

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
    sync_var: ArcVar<RawConfigMap>,
    backend: PhantomData<fn() -> B>,
    status: ReadOnlyArcVar<ConfigStatus>,
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> BoxedVar<RawConfigValue> {
        // init value
        let current_raw_value = self.sync_var.with(|m| m.get(&key).cloned());
        let value_var = match current_raw_value {
            Some(v) => var(v),
            None => {
                if insert {
                    self.sync_var.modify(clmv!(key, default, |args| {
                        if !args.contains_key(&key) {
                            args.to_mut().insert(key, default);
                        }
                    }));
                }
                var(default)
            }
        };

        // map -> value
        let sync_var_tag = SourceVarTag::new(&self.sync_var);
        let value_var_weak = value_var.downgrade();

        self.sync_var
            .hook(clmv!(key, |args| match value_var_weak.upgrade() {
                Some(value_var) => {
                    let is_from_value_var = args.downcast_tags::<SourceVarTag>().any(|&b| b == SourceVarTag::new(&value_var));

                    if !is_from_value_var && let Some(raw_value) = args.value().get(&key) {
                        value_var.modify(clmv!(raw_value, |args| {
                            args.set(raw_value);
                            args.push_tag(sync_var_tag);
                        }));
                    }
                    true // retain
                }
                None => {
                    false
                }
            }))
            .perm();

        // value -> map
        let value_var_tag = SourceVarTag::new(&value_var);
        let sync_var_weak = self.sync_var.downgrade();
        value_var
            .hook(move |args| match sync_var_weak.upgrade() {
                Some(sync_var) => {
                    let is_from_sync_var = args.downcast_tags::<SourceVarTag>().any(|&b| b == SourceVarTag::new(&sync_var));

                    if !is_from_sync_var {
                        let raw_value = args.value().clone();
                        sync_var.modify(clmv!(key, |args| {
                            if args.get(&key) != Some(&raw_value) {
                                args.to_mut().insert(key, raw_value);
                                args.push_tag(value_var_tag);
                            }
                        }));
                    }

                    true
                }
                None => false,
            })
            .perm();

        value_var.boxed()
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
                if m.contains_key(&key) {
                    m.to_mut().shift_remove(&key);
                }
            }));
        }
        contains
    }

    fn low_memory(&mut self) {}
}
