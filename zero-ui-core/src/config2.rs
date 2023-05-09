use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use crate::{clmv, fs_watcher::WATCHER, text::Txt, var::*};

pub struct CONFIG;

impl CONFIG {}

/// Represents a config source
pub struct JsonConfig {
    sync_var: ArcVar<HashMap<Txt, serde_json::Value>>,
    error: ArcVar<Option<Arc<io::Error>>>,
}

impl JsonConfig {
    /// Open or create the `json_file` config.
    pub fn new(json_file: impl Into<PathBuf>) -> Self {
        let error = var(None);
        let sync_var = WATCHER.sync(
            json_file,
            HashMap::new(),
            clmv!(error, |r| {
                match (|| r?.json().map_err(io::Error::from))() {
                    Ok(ok) => {
                        if error.with(Option::is_some) {
                            error.set(None);
                        }
                        Some(ok)
                    }
                    Err(e) => {
                        error.set(Some(Arc::new(e)));
                        None
                    }
                }
            }),
            clmv!(error, |map, w| {
                match (|| {
                    let mut w = w?;
                    w.write_json(&map, true)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        if error.with(Option::is_some) {
                            error.set(None);
                        }
                    }
                    Err(e) => {
                        error.set(Some(Arc::new(e)));
                    }
                }
            }),
        );

        Self { sync_var, error }
    }

    /// Variable that shows the latest read or write error.
    ///
    /// After a sucessfull read or write the variable updates to `None`.
    pub fn error(&self) -> ReadOnlyArcVar<Option<Arc<io::Error>>> {
        self.error.read_only()
    }

    /// Gets a variable connected with the `key` in this config.
    pub fn var<T>(&self, default: impl Fn() -> T + Send + Sync + 'static, key: impl Into<Txt>) -> ArcVar<T>
    where
        T: VarValue + serde::Serialize + serde::de::DeserializeOwned,
    {
        let key = key.into();
        let var = match self
            .sync_var
            .with(|map| map.get(&key).and_then(|json| serde_json::from_value(json.clone()).ok()))
        {
            Some(init) => var(init),
            None => {
                let init = default();
                match serde_json::to_value(&init) {
                    Ok(json) => {
                        self.sync_var.modify(clmv!(key, |map| {
                            if map.get(&key) != Some(&json) {
                                // only `to_mut` causes a write
                                map.to_mut().insert(key, json);
                            }
                        }));
                    }
                    Err(e) => {
                        self.error.set(Some(Arc::new(e.into())));
                    }
                }
                var(init)
            }
        };

        var
    }
}
