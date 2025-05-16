use super::*;

impl ConfigMap for indexmap::IndexMap<ConfigKey, serde_yaml::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.yaml().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        file.write_yaml(&self)
    }

    fn get_raw(&self, key: &ConfigKey) -> Result<Option<RawConfigValue>, Arc<dyn std::error::Error + Send + Sync>> {
        match self.get(key) {
            Some(e) => Ok(Some(RawConfigValue::try_from(e.clone())?)),
            None => Ok(None),
        }
    }

    fn set_raw(map: &mut VarModify<Self>, key: ConfigKey, value: RawConfigValue) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let value = value.try_into()?;
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
            match serde_yaml::from_value(value.clone()) {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut VarModify<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_yaml::to_value(&value) {
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

/// Represents a config source that synchronizes with a YAML file.
pub type YamlConfig = SyncConfig<indexmap::IndexMap<ConfigKey, serde_yaml::Value>>;

impl TryFrom<serde_yaml::Value> for RawConfigValue {
    type Error = YamlValueRawError;

    fn try_from(value: serde_yaml::Value) -> Result<Self, Self::Error> {
        let ok = match value {
            serde_yaml::Value::Null => serde_json::Value::Null,
            serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
            serde_yaml::Value::Number(n) => {
                // serde_json does not implicit converts float to integer, so we try integers first here.
                serde_json::Value::Number(if let Some(n) = n.as_i64() {
                    n.into()
                } else if let Some(n) = n.as_u64() {
                    n.into()
                } else if let Some(n) = n.as_f64() {
                    match serde_json::Number::from_f64(n) {
                        Some(n) => n,
                        None => return Err(YamlValueRawError::UnsupportedFloat(n)),
                    }
                } else {
                    unreachable!()
                })
            }
            serde_yaml::Value::String(s) => serde_json::Value::String(s),
            serde_yaml::Value::Sequence(s) => serde_json::Value::Array({
                let mut r = Vec::with_capacity(s.len());
                for v in s {
                    r.push(RawConfigValue::try_from(v)?.0);
                }
                r
            }),
            serde_yaml::Value::Mapping(m) => serde_json::Value::Object({
                let mut o = serde_json::Map::with_capacity(m.len());
                for (key, value) in m {
                    o.insert(yaml_map_key(key)?, RawConfigValue::try_from(value)?.0);
                }
                o
            }),
            serde_yaml::Value::Tagged(v) => RawConfigValue::try_from(v.value)?.0,
        };

        Ok(Self(ok))
    }
}
impl TryFrom<RawConfigValue> for serde_yaml::Value {
    type Error = YamlValueRawError;

    fn try_from(value: RawConfigValue) -> Result<Self, Self::Error> {
        let ok = match value.0 {
            serde_json::Value::Null => serde_yaml::Value::Null,
            serde_json::Value::Bool(b) => serde_yaml::Value::Bool(b),
            serde_json::Value::Number(n) => {
                // serde_json does not implicit converts float to integer, so we try integers first here.
                serde_yaml::Value::Number(if let Some(f) = n.as_i64() {
                    serde_yaml::Number::from(f)
                } else if let Some(n) = n.as_u64() {
                    serde_yaml::Number::from(n)
                } else if let Some(n) = n.as_f64() {
                    serde_yaml::Number::from(n)
                } else {
                    unreachable!()
                })
            }
            serde_json::Value::String(s) => serde_yaml::Value::String(s),
            serde_json::Value::Array(a) => serde_yaml::Value::Sequence({
                let mut r = Vec::with_capacity(a.len());
                for v in a {
                    r.push(RawConfigValue(v).try_into()?);
                }
                r
            }),
            serde_json::Value::Object(o) => serde_yaml::Value::Mapping({
                let mut r = serde_yaml::Mapping::with_capacity(o.len());
                for (k, v) in o {
                    r.insert(serde_yaml::Value::String(k), RawConfigValue(v).try_into()?);
                }
                r
            }),
        };
        Ok(ok)
    }
}

fn yaml_map_key(key: serde_yaml::Value) -> Result<String, YamlValueRawError> {
    let ok = match key {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s,
        serde_yaml::Value::Sequence(_) | serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => {
            return Err(YamlValueRawError::UnsupportedKey);
        }
    };
    Ok(ok)
}

/// Error converting serde_yaml::Value, RawConfigValue.
#[derive(Debug, Clone, Copy)]
pub enum YamlValueRawError {
    /// JSON only supports finite floats.
    UnsupportedFloat(f64),
    /// JSON only supports key types that are [`fmt::Display`].
    UnsupportedKey,
}
impl fmt::Display for YamlValueRawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error converting yaml to internal json, ")?;
        match self {
            Self::UnsupportedFloat(fl) => write!(f, "json does not support float `{fl}`"),
            Self::UnsupportedKey => write!(f, "json does not support non-display keys"),
        }
    }
}
impl std::error::Error for YamlValueRawError {}
impl From<YamlValueRawError> for Arc<dyn std::error::Error + Send + Sync> {
    fn from(value: YamlValueRawError) -> Self {
        Arc::new(value)
    }
}
