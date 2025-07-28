use crate::task::parking_lot::Mutex;
use zng_var::var_expr;

use super::*;

/// Reset controls of a [`FallbackConfig`].
pub trait FallbackConfigReset: AnyConfig + Sync {
    /// Removes the `key` from the config source and reverts all active variables to the fallback source.
    fn reset(&self, key: &ConfigKey);

    /// Gets if the config source contains the `key`.
    fn can_reset(&self, key: ConfigKey) -> Var<bool>;

    /// Clone a reference to the config.
    fn clone_boxed(&self) -> Box<dyn FallbackConfigReset>;
}
impl Clone for Box<dyn FallbackConfigReset> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

/// Represents a copy-on-write config source that wraps two other sources, a read-write config and a read-only fallback config.
///
/// The config variables are connected to both sources, if the read-write config is not set the var will update with the
/// fallback config, if it is set it will sync with the read-write config.
///
/// The `FallbackConfig` type is an `Arc` internally, so you can keep a cloned reference to it after moving it into
/// [`CONFIG`] or another combinator config.
pub struct FallbackConfig<S: Config, F: Config>(Arc<Mutex<FallbackConfigData<S, F>>>);

impl<S: Config, F: Config> FallbackConfig<S, F> {
    /// New from write source and fallback source.
    pub fn new(source: S, fallback: F) -> Self {
        Self(Arc::new(Mutex::new(FallbackConfigData {
            source,
            fallback,
            output: Default::default(),
        })))
    }
}

impl<S: AnyConfig, F: AnyConfig> AnyConfig for FallbackConfig<S, F> {
    fn status(&self) -> Var<ConfigStatus> {
        let self_ = self.0.lock();
        var_expr! {
            ConfigStatus::merge_status([#{self_.fallback.status()}.clone(), #{self_.source.status()}.clone()].into_iter())
        }
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> Var<RawConfigValue> {
        self.0.lock().bind_raw(key, default, insert)
    }

    fn contains_key(&mut self, key: ConfigKey) -> Var<bool> {
        let mut self_ = self.0.lock();
        var_expr! {
            *#{self_.source.contains_key(key.clone())} || *#{self_.fallback.contains_key(key)}
        }
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        let mut self_ = self.0.lock();
        let a = self_.source.remove(key);
        let b = self_.fallback.remove(key);
        a || b
    }

    fn low_memory(&mut self) {
        let mut self_ = self.0.lock();
        self_.source.low_memory();
        self_.fallback.low_memory();
        self_.output.retain(|_, v| v.output_weak.strong_count() > 0);
    }
}
impl<S: AnyConfig, F: AnyConfig> FallbackConfigReset for FallbackConfig<S, F> {
    fn reset(&self, key: &ConfigKey) {
        let mut self_ = self.0.lock();
        // remove from source
        self_.source.remove(key);
    }

    fn can_reset(&self, key: ConfigKey) -> Var<bool> {
        let mut self_ = self.0.lock();
        self_.source.contains_key(key)
    }

    fn clone_boxed(&self) -> Box<dyn FallbackConfigReset> {
        Box::new(Self(self.0.clone()))
    }
}

struct FallbackConfigData<S, F> {
    source: S,
    fallback: F,
    output: HashMap<ConfigKey, OutputEntry>,
}

impl<S: AnyConfig, F: AnyConfig> FallbackConfigData<S, F> {
    fn bind_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> Var<RawConfigValue> {
        if let Some(entry) = self.output.get(&key)
            && let Some(output) = entry.output_weak.upgrade()
        {
            return output;
        }

        let fallback = self.fallback.get(key.clone(), default, false);
        let source = self.source.get(key.clone(), fallback.get(), insert);
        let source_contains = self.source.contains_key(key.clone());
        let fallback_tag = fallback.var_instance_tag();

        let output = var(if source_contains.get() { source.get() } else { fallback.get() });
        let weak_output = output.downgrade();

        // update output when not source_contains
        let fallback_hook = fallback.hook(clmv!(weak_output, source_contains, |args| {
            if let Some(output) = weak_output.upgrade() {
                if !source_contains.get() {
                    let value = args.value().clone();
                    output.modify(move |o| {
                        o.set(value);
                        o.push_tag(fallback_tag);
                    });
                }
                true // retain hook
            } else {
                false // output dropped
            }
        }));

        // update output
        let source_hook = source.hook(clmv!(weak_output, |args| {
            if let Some(output) = weak_output.upgrade() {
                let output_tag = output.var_instance_tag();
                if !args.contains_tag(&output_tag) {
                    output.set(args.value().clone());
                }
                true
            } else {
                false // output dropped
            }
        }));

        // reset output to fallback when contains changes to false
        // or set output to source in case the entry is back with the same value (no source update)
        let weak_fallback = fallback.downgrade(); // fallback_hook holds source_contains
        let source_contains_hook = source_contains.hook(clmv!(weak_output, source, |args| {
            if let Some(output) = weak_output.upgrade() {
                if *args.value() {
                    output.set(source.get());
                } else {
                    let fallback = weak_fallback.upgrade().unwrap();
                    let fallback_value = fallback.get();
                    let fallback_tag = fallback.var_instance_tag();
                    output.modify(move |o| {
                        o.set(fallback_value);
                        o.push_tag(fallback_tag);
                    });
                }

                true
            } else {
                false // output dropped
            }
        }));

        // update source
        let output_tag = output.var_instance_tag();
        output
            .hook(move |args| {
                let _hold = (&fallback, &fallback_hook, &source_hook, &source_contains_hook);

                if !args.contains_tag(&fallback_tag) {
                    let value = args.value().clone();
                    source.modify(move |s| {
                        s.set(value);
                        s.update(); // in case of reset the source var can retain the old value
                        s.push_tag(output_tag);
                    });
                }
                true
            })
            .perm();

        self.output.insert(
            key,
            OutputEntry {
                output_weak: output.downgrade(),
            },
        );

        output
    }
}
struct OutputEntry {
    output_weak: WeakVar<RawConfigValue>,
}
