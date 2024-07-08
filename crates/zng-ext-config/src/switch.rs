use zng_clone_move::clmv;
use zng_var::MergeVarBuilder;

use super::*;

/// Represents multiple config sources that are matched by key.
///
/// When a config key is requested a closure defined for each config case in the switch
/// is called, if the closure returns a key the config case is used.
///
/// Note that the returned config variables are linked directly with the matched configs,
/// and if none matches returns from a fallback [`MemoryConfig`]. If a config is pushed after no match
/// the already returned variable will not update to link with the new config.
#[derive(Default)]
pub struct SwitchConfig {
    cfgs: Vec<SwitchCfg>,
}
impl SwitchConfig {
    /// New default empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a config case on the switch.
    ///
    /// The `match_key` closure will be called after the match of previous configs, if it returns `Some(key)`
    /// the key will be used on the `config` to retrieve the value variable.
    pub fn push(&mut self, match_key: impl Fn(&ConfigKey) -> Option<ConfigKey> + Send + Sync + 'static, config: impl AnyConfig) {
        self.cfgs.push(SwitchCfg {
            match_key: Box::new(match_key),
            cfg: Box::new(config),
        })
    }

    /// Push a config case matched by a key `prefix`.
    ///
    /// The `prefix` is stripped from the key before it is passed on to the `config`.
    ///
    /// Always matches the config if the prefix is empty.
    pub fn push_prefix(&mut self, prefix: impl Into<Txt>, config: impl AnyConfig) {
        let prefix = prefix.into();
        if prefix.is_empty() {
            self.push(|key| Some(key.clone()), config)
        } else {
            self.push(move |key| key.strip_prefix(prefix.as_str()).map(Txt::from_str), config)
        }
    }

    /// Push the config and return.
    ///
    /// See [`push`] for more details.
    ///
    /// [`push`]: Self::push
    pub fn with(mut self, match_key: impl Fn(&ConfigKey) -> Option<ConfigKey> + Send + Sync + 'static, config: impl AnyConfig) -> Self {
        self.push(match_key, config);
        self
    }

    /// Push the config and return.
    ///
    /// See [`push_prefix`] for more details.
    ///
    /// [`push_prefix`]: Self::push
    pub fn with_prefix(mut self, prefix: impl Into<Txt>, config: impl AnyConfig) -> Self {
        self.push_prefix(prefix, config);
        self
    }

    fn cfg_mut(&mut self, key: &ConfigKey) -> Option<(ConfigKey, &mut dyn AnyConfig)> {
        for c in &mut self.cfgs {
            if let Some(key) = (c.match_key)(key) {
                return Some((key, &mut *c.cfg));
            }
        }
        None
    }
}
impl AnyConfig for SwitchConfig {
    fn status(&self) -> BoxedVar<ConfigStatus> {
        let mut s = MergeVarBuilder::with_capacity(self.cfgs.len());
        for c in &self.cfgs {
            s.push(c.cfg.status());
        }
        s.build(|status| ConfigStatus::merge_status(status.iter().cloned())).boxed()
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool, shared: bool) -> BoxedVar<RawConfigValue> {
        match self.cfg_mut(&key) {
            Some((key, cfg)) => cfg.get_raw(key, default, insert, shared),
            None => LocalVar(default).boxed(),
        }
    }

    fn contains_key(&mut self, key: ConfigKey) -> BoxedVar<bool> {
        match self.cfg_mut(&key) {
            Some((key, cfg)) => cfg.contains_key(key),
            None => LocalVar(false).boxed(),
        }
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        match self.cfg_mut(key) {
            Some((key, cfg)) => cfg.remove(&key),
            None => false,
        }
    }

    fn low_memory(&mut self) {
        for c in &mut self.cfgs {
            c.cfg.low_memory();
        }
    }
}
impl Config for SwitchConfig {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: T, insert: bool) -> BoxedVar<T> {
        let key = key.into();
        match self.cfg_mut(&key) {
            Some((key, cfg)) => {
                let source_var = cfg.get_raw(
                    key.clone(),
                    RawConfigValue::serialize(&default).unwrap_or_else(|e| panic!("invalid default value, {e}")),
                    insert,
                    false,
                );
                let var = var(RawConfigValue::deserialize(source_var.get()).unwrap_or(default));

                source_var
                    .bind_filter_map_bidi(
                        &var,
                        // Raw -> T
                        clmv!(key, |raw| {
                            match RawConfigValue::deserialize(raw.clone()) {
                                Ok(value) => Some(value),
                                Err(e) => {
                                    tracing::error!("switch config get({key:?}) error, {e:?}");
                                    None
                                }
                            }
                        }),
                        // T -> Raw
                        clmv!(key, source_var, |value| {
                            let _strong_ref = &source_var;

                            match RawConfigValue::serialize(value) {
                                Ok(raw) => Some(raw),
                                Err(e) => {
                                    tracing::error!("switch config set({key:?}) error, {e:?}");
                                    None
                                }
                            }
                        }),
                    )
                    .perm();

                var.boxed()
            }
            None => LocalVar(default).boxed(),
        }
    }
}

struct SwitchCfg {
    match_key: Box<dyn Fn(&ConfigKey) -> Option<ConfigKey> + Send + Sync>,
    cfg: Box<dyn AnyConfig>,
}
