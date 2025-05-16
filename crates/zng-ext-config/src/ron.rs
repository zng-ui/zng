use std::mem;

use super::*;

use ::ron as serde_ron;

impl ConfigMap for indexmap::IndexMap<ConfigKey, serde_ron::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.ron().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        file.write_ron(&self, true)
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
            match value.clone().into_rust() {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut VarModify<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_ron::to_string(&value) {
            Ok(value) => match serde_ron::from_str(&value) {
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

/// Represents a config source that synchronizes with a RON file.
pub type RonConfig = SyncConfig<indexmap::IndexMap<ConfigKey, serde_ron::Value>>;

impl TryFrom<serde_ron::Value> for RawConfigValue {
    type Error = RonValueRawError;

    fn try_from(value: serde_ron::Value) -> Result<Self, Self::Error> {
        let ok = match value {
            serde_ron::Value::Bool(b) => serde_json::Value::Bool(b),
            serde_ron::Value::Char(c) => serde_json::Value::String(c.to_string()),
            serde_ron::Value::String(s) => serde_json::Value::String(s),
            serde_ron::Value::Number(n) => serde_json::Value::Number(match n {
                serde_ron::Number::I8(n) => serde_json::Number::from(n),
                serde_ron::Number::I16(n) => serde_json::Number::from(n),
                serde_ron::Number::I32(n) => serde_json::Number::from(n),
                serde_ron::Number::I64(n) => serde_json::Number::from(n),
                serde_ron::Number::U8(n) => serde_json::Number::from(n),
                serde_ron::Number::U16(n) => serde_json::Number::from(n),
                serde_ron::Number::U32(n) => serde_json::Number::from(n),
                serde_ron::Number::U64(n) => serde_json::Number::from(n),
                serde_ron::Number::F32(n) => match serde_json::Number::from_f64(n.get() as _) {
                    Some(n) => n,
                    None => return Err(RonValueRawError::UnsupportedFloat(n.get() as _)),
                },
                serde_ron::Number::F64(n) => match serde_json::Number::from_f64(n.get()) {
                    Some(n) => n,
                    None => return Err(RonValueRawError::UnsupportedFloat(n.get())),
                },
                _ => return Err(RonValueRawError::UnsupportedValue),
            }),
            serde_ron::Value::Option(o) => match o {
                Some(v) => return Self::try_from(*v),
                None => serde_json::Value::Null,
            },
            serde_ron::Value::Seq(s) => serde_json::Value::Array({
                let mut r = Vec::with_capacity(s.len());
                for v in s {
                    r.push(RawConfigValue::try_from(v)?.0);
                }
                r
            }),
            serde_ron::Value::Map(mut m) => serde_json::Value::Object({
                let mut r = serde_json::Map::with_capacity(m.len());
                // ron::Map is not IntoIter
                for (k, v) in m.iter_mut() {
                    r.insert(
                        ron_map_key(k)?,
                        RawConfigValue::try_from(mem::replace(v, serde_ron::Value::Unit))?.0,
                    );
                }
                r
            }),
            serde_ron::Value::Unit => serde_json::Value::Null,
            serde_ron::Value::Bytes(_) => return Err(RonValueRawError::UnsupportedValue),
        };
        Ok(Self(ok))
    }
}
impl TryFrom<RawConfigValue> for serde_ron::Value {
    type Error = RonValueRawError;

    fn try_from(value: RawConfigValue) -> Result<Self, Self::Error> {
        let ok = match value.0 {
            serde_json::Value::Null => serde_ron::Value::Unit,
            serde_json::Value::Bool(b) => serde_ron::Value::Bool(b),
            serde_json::Value::Number(n) => serde_ron::Value::Number(if let Some(i) = n.as_i64() {
                serde_ron::Number::from(i)
            } else {
                let f = n.as_f64().unwrap();
                serde_ron::Number::from(f)
            }),
            serde_json::Value::String(s) => serde_ron::Value::String(s),
            serde_json::Value::Array(a) => serde_ron::Value::Seq({
                let mut r = Vec::with_capacity(a.len());
                for v in a {
                    r.push(RawConfigValue(v).try_into()?);
                }
                r
            }),
            serde_json::Value::Object(o) => serde_ron::Value::Map({
                // ron::Map has no with_capacity
                let mut r = serde_ron::Map::new();
                for (k, v) in o {
                    r.insert(serde_ron::Value::String(k), serde_ron::Value::try_from(RawConfigValue(v))?);
                }
                r
            }),
        };
        Ok(ok)
    }
}

fn ron_map_key(key: &serde_ron::Value) -> Result<String, RonValueRawError> {
    let ok = match key {
        serde_ron::Value::String(s) => s.clone(),
        serde_ron::Value::Bool(b) => b.to_string(),
        serde_ron::Value::Char(c) => format!("{c}"),
        serde_ron::Value::Number(n) => match n {
            ::ron::Number::I8(n) => n.to_string(),
            ::ron::Number::I16(n) => n.to_string(),
            ::ron::Number::I32(n) => n.to_string(),
            ::ron::Number::I64(n) => n.to_string(),
            ::ron::Number::U8(n) => n.to_string(),
            ::ron::Number::U16(n) => n.to_string(),
            ::ron::Number::U32(n) => n.to_string(),
            ::ron::Number::U64(n) => n.to_string(),
            _ => return Err(RonValueRawError::UnsupportedKey), // no floats, no any new variant
        },
        serde_ron::Value::Unit => String::new(),
        serde_ron::Value::Option(o) => match o {
            Some(o) => return ron_map_key(o),
            None => String::new(),
        },
        serde_ron::Value::Map(_) | serde_ron::Value::Seq(_) | serde_ron::Value::Bytes(_) => return Err(RonValueRawError::UnsupportedKey),
    };
    Ok(ok)
}

/// Error converting ron::Value, RawConfigValue.
#[derive(Debug, Clone, Copy)]
pub enum RonValueRawError {
    /// JSON only supports finite floats.
    UnsupportedFloat(f64),
    /// JSON only supports key types that are [`fmt::Display`].
    UnsupportedKey,
    /// RON added a new number format or value kind that is not supported yet.
    UnsupportedValue,
}
impl fmt::Display for RonValueRawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error converting ron to internal json, ")?;
        match self {
            RonValueRawError::UnsupportedFloat(fl) => write!(f, "json does not support float `{fl}`"),
            RonValueRawError::UnsupportedKey => write!(f, "json does not support non-display keys"),
            RonValueRawError::UnsupportedValue => write!(f, "conversion to json does not support the number format or value kind"),
        }
    }
}
impl std::error::Error for RonValueRawError {}
impl From<RonValueRawError> for Arc<dyn std::error::Error + Send + Sync> {
    fn from(value: RonValueRawError) -> Self {
        Arc::new(value)
    }
}
