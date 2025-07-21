use super::*;

use ::toml as serde_toml;

impl ConfigMap for indexmap::IndexMap<ConfigKey, serde_toml::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.toml()
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        if self.is_empty() {
            // helps diagnosticate issues with empty config, JSON and RON empty are `{}`, TOML is
            // zero-sized if we don't add this.
            file.write_text("#")
        } else {
            file.write_toml(&self, true)
        }
    }

    fn get_raw(&self, key: &ConfigKey) -> Result<Option<RawConfigValue>, Arc<dyn std::error::Error + Send + Sync>> {
        match self.get(key) {
            Some(sv) => match RawConfigValue::serialize(sv) {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(Arc::new(e)),
            },
            None => Ok(None),
        }
    }

    fn set_raw(map: &mut VarModify<Self>, key: ConfigKey, value: RawConfigValue) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let value = match value.deserialize() {
            Ok(v) => v,
            Err(e) => return Err(Arc::new(e)),
        };
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
            match serde_toml::to_string(&value) {
                Ok(value) => match serde_toml::from_str(&value) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(Arc::new(e)),
                },
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut VarModify<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_toml::to_string(&value) {
            Ok(value) => match serde_toml::from_str(&value) {
                Ok(value) => {
                    if map.get(&key) != Some(&value) {
                        map.to_mut().insert(key, value);
                    }
                    Ok(())
                }
                Err(e) => Err(Arc::new(e)),
            },
            Err(e) => Err(Arc::new(e)),
        }
    }

    fn remove(map: &mut VarModify<Self>, key: &ConfigKey) {
        if map.contains_key(key) {
            map.to_mut().shift_remove(key);
        }
    }
}

/// Represents a config source that synchronizes with a TOML file.
pub type TomlConfig = SyncConfig<indexmap::IndexMap<ConfigKey, serde_toml::Value>>;
