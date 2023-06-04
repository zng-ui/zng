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
}

/// Represents a config source that synchronizes with a TOML file.
pub type TomlConfig = SyncConfig<indexmap::IndexMap<ConfigKey, serde_toml::Value>>;

impl TryFrom<serde_toml::Value> for RawConfigValue {
    type Error = TomlValueRawError;

    fn try_from(value: serde_toml::Value) -> Result<Self, Self::Error> {
        let ok = match value {
            serde_toml::Value::String(s) => serde_json::Value::String(s),
            serde_toml::Value::Integer(n) => serde_json::Value::Number(n.into()),
            serde_toml::Value::Float(f) => match serde_json::Number::from_f64(f) {
                Some(f) => serde_json::Value::Number(f),
                None => return Err(TomlValueRawError::InvalidFloat(f)),
            },
            serde_toml::Value::Boolean(b) => serde_json::Value::Bool(b),
            serde_toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
            serde_toml::Value::Array(a) => serde_json::Value::Array({
                let mut r = Vec::with_capacity(a.len());
                for v in a {
                    r.push(RawConfigValue::try_from(v)?.0);
                }
                r
            }),
            serde_toml::Value::Table(m) => serde_json::Value::Object({
                let mut r = serde_json::Map::with_capacity(m.len());
                for (k, v) in m {
                    r.insert(k, RawConfigValue::try_from(v)?.0);
                }
                r
            }),
        };
        Ok(Self(ok))
    }
}
impl TryFrom<RawConfigValue> for serde_toml::Value {
    type Error = TomlValueRawError;

    fn try_from(value: RawConfigValue) -> Result<Self, Self::Error> {
        let ok = match value.0 {
            serde_json::Value::Null => return Err(TomlValueRawError::Null),
            serde_json::Value::Bool(b) => serde_toml::Value::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    serde_toml::Value::Float(f)
                } else if let Some(i) = n.as_i64() {
                    serde_toml::Value::Integer(i)
                } else {
                    let i = n.as_u64().unwrap();
                    if i > i64::MAX as u64 {
                        return Err(TomlValueRawError::InvalidInt(i));
                    }
                    serde_toml::Value::Integer(i as i64)
                }
            }
            serde_json::Value::String(s) => serde_toml::Value::String(s),
            serde_json::Value::Array(a) => serde_toml::Value::Array({
                let mut r = Vec::with_capacity(a.len());
                for v in a {
                    match RawConfigValue(v).try_into() {
                        Ok(v) => r.push(v),
                        Err(TomlValueRawError::Null) => continue,
                        e => return e,
                    }
                }
                r
            }),
            serde_json::Value::Object(m) => serde_toml::Value::Table({
                let mut r = serde_toml::Table::with_capacity(m.len());
                for (k, v) in m {
                    match RawConfigValue(v).try_into() {
                        Ok(v) => {
                            r.insert(k, v);
                        }
                        Err(TomlValueRawError::Null) => continue,
                        e => return e,
                    }
                }
                r
            }),
        };
        Ok(ok)
    }
}

/// Error converting toml::Value, RawConfigValue.
#[derive(Debug, Clone, Copy)]
pub enum TomlValueRawError {
    /// JSON only supports finite floats.
    InvalidFloat(f64),
    /// TOML does not support `null`.
    Null,
    /// TOML only supports integers up to `i64::MAX`.
    InvalidInt(u64),
}
impl fmt::Display for TomlValueRawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlValueRawError::InvalidFloat(fl) => write!(f, "json does not support float `{fl}`"),
            TomlValueRawError::Null => write!(f, "toml does not support `null`"),
            TomlValueRawError::InvalidInt(i) => write!(f, "toml does not support int > i64::MAX ({i})"),
        }
    }
}
impl std::error::Error for TomlValueRawError {}
impl From<TomlValueRawError> for Arc<dyn std::error::Error + Send + Sync> {
    fn from(value: TomlValueRawError) -> Self {
        Arc::new(value)
    }
}
