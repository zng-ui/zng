use zng_ext_fs_watcher::{WatchFile, WriteFile};

use super::*;

impl ConfigMap for indexmap::IndexMap<ConfigKey, serde_json::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.json().map_err(Into::into)
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        file.write_json(&self, true)
    }

    fn get_raw(&self, key: &ConfigKey) -> Result<Option<RawConfigValue>, Arc<dyn std::error::Error + Send + Sync>> {
        Ok(self.get(key).map(|v| RawConfigValue(v.clone())))
    }

    fn set_raw(map: &mut VarModify<Self>, key: ConfigKey, value: RawConfigValue) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let value = value.0;
        if map.get(&key) != Some(&value) {
            map.to_mut().insert(key, value);
        }
        Ok(())
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.contains_key(key)
    }

    fn get<O: ConfigValue>(&self, key: &ConfigKey) -> Result<Option<O>, Arc<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.get(key) {
            match serde_json::from_value(value.clone()) {
                Ok(s) => Ok(Some(s)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut VarModify<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_json::to_value(value) {
            Ok(value) => {
                if map.get(&key) != Some(&value) {
                    map.to_mut().insert(key, value);
                }
                Ok(())
            }
            Err(e) => Err(Arc::new(e)),
        }
    }

    fn remove(map: &mut VarModify<Self>, key: &ConfigKey) {
        if map.contains_key(key) {
            map.to_mut().shift_remove(key);
        }
    }
}

/// Represents a config source that synchronizes with a JSON file.
pub type JsonConfig = SyncConfig<indexmap::IndexMap<ConfigKey, serde_json::Value>>;
