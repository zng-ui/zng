use super::*;

impl ConfigMap for HashMap<ConfigKey, serde_json::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.json().map_err(Into::into)
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        file.write_json(&self, true).map_err(Into::into)
    }

    fn get_json(&self, key: &ConfigKey) -> Result<Option<serde_json::Value>, Arc<dyn std::error::Error + Send + Sync>> {
        Ok(self.get(key).cloned())
    }

    fn set_json(map: &mut Cow<Self>, key: ConfigKey, value: serde_json::Value) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        if map.get(&key) != Some(&value) {
            map.to_mut().insert(key, value);
        }
        Ok(())
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.contains_key(key)
    }

    fn get<O: ConfigValue>(&self, key: &ConfigKey) -> Result<Option<O>, Arc<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.get_json(key)? {
            match serde_json::from_value(value) {
                Ok(s) => Ok(Some(s)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut Cow<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_json::to_value(value) {
            Ok(s) => Self::set_json(map, key, s),
            Err(e) => Err(Arc::new(e)),
        }
    }
}

/// Represents a config source that synchronizes with a JSON file.
pub type JsonConfig = SyncConfig<HashMap<ConfigKey, serde_json::Value>>;
