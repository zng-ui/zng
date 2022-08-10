//! Config manager.
//!
//! The [`ConfigManager`] is an [app extension], it
//! is included in the [default app] and manages the [`Config`] service that can be used to store and retrieve
//! state that is persisted between application runs.
//!
//! [app extension]: crate::app::AppExtension
//! [default app]: crate::app::App::default

use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::Entry, HashMap, HashSet},
    error::Error,
    fmt,
    rc::Rc,
    sync::Arc,
};

use crate::{
    app::{AppEventSender, AppExtReceiver, AppExtSender, AppExtension},
    context::*,
    crate_util::BoxedFut,
    service::*,
    task::ui::UiTask,
    text::Text,
    var::*,
};

use serde_json::value::Value as JsonValue;

mod file_source;
pub use file_source::{ConfigFile, ConfigFileBuilder};

mod combinators;
pub use combinators::*;

/// Application extension that manages the app configuration access point ([`Config`]).
///
/// Note that this extension does not implement a [`ConfigSource`], it just manages whatever source is installed and
/// the config variables.
#[derive(Default)]
pub struct ConfigManager {}
impl ConfigManager {}
impl AppExtension for ConfigManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Config::new(ctx.updates.sender()));
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        if let Some((mut source, _)) = Config::req(ctx).source.take() {
            source.deinit();
        }
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        Config::req(ctx.services).update(ctx.vars);
    }
}

/// Key to a persistent config in [`Config`].
pub type ConfigKey = Text;

/// A type that can be a [`Config`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait ConfigValue: VarValue + PartialEq + serde::Serialize + serde::de::DeserializeOwned {}
impl<T: VarValue + PartialEq + serde::Serialize + serde::de::DeserializeOwned> ConfigValue for T {}

/// Return `true` to retain, `false` to drop.
type ConfigTask = Box<dyn FnMut(&Vars, &RcVar<ConfigStatus>) -> bool>;
type OnceConfigTask = Box<dyn FnOnce(&Vars, &RcVar<ConfigStatus>)>;

/// Represents the configuration of the app.
///
/// This type does not implement any config scheme, a [`ConfigSource`] must be set to enable persistence, without a source
/// only the config variables work, and only for the duration of the app process.
///
/// Note that this is a service *singleton* that represents the config in use by the app, to load other config files
/// you can use the [`Config::load_alt`].
///
/// # Examples
///
/// The example demonstrates loading a config file and binding a config to a variable that is auto saves every time it changes.
///
/// ```no_run
/// # use zero_ui_core::{app::*, window::*, config::*, units::*};
/// # macro_rules! window { ($($tt:tt)*) => { unimplemented!() } }
/// App::default().run_window(|ctx| {
///     // require the Config service, it is available in the default App.
///     let cfg = Config::req(ctx.services);
///
///     // load a ConfigSource.
///     cfg.load(ConfigFile::new("app.config.json", true, 3.secs()));
///     
///     // read the "main.count" config and bind it to a variable.
///     let count = cfg.var("main.count", || 0);
///
///     window! {
///         title = "Persistent Counter";
///         padding = 20;
///         content = button! {
///             content = text(count.map(|c| formatx!("Count: {c}")));
///             on_click = hn!(|ctx, _| {
///                 // modifying the var updates the "main.count" config.
///                 count.modify(ctx, |mut c| *c += 1).unwrap();
///             });
///         }
///     }
/// })
/// ```
#[derive(Service)]
pub struct Config {
    update: AppEventSender,
    source: Option<(Box<dyn ConfigSource>, AppExtReceiver<ConfigSourceUpdate>)>,
    vars: HashMap<ConfigKey, ConfigVar>,

    status: RcVar<ConfigStatus>,

    once_tasks: Vec<OnceConfigTask>,
    tasks: Vec<ConfigTask>,

    alts: Vec<std::rc::Weak<RefCell<Config>>>,
}
impl Config {
    fn new(update: AppEventSender) -> Self {
        Config {
            update,
            source: None,
            vars: HashMap::new(),

            status: var(ConfigStatus::default()),

            once_tasks: vec![],
            tasks: vec![],
            alts: vec![],
        }
    }

    fn update(&mut self, vars: &Vars) {
        // run once tasks
        for task in self.once_tasks.drain(..) {
            task(vars, &self.status);
        }

        // collect source updates
        let mut read = HashSet::new();
        let mut read_all = false;
        if let Some((_, source_tasks)) = &self.source {
            while let Ok(task) = source_tasks.try_recv() {
                match task {
                    ConfigSourceUpdate::Refresh(key) => {
                        if !read_all {
                            read.insert(key);
                        }
                    }
                    ConfigSourceUpdate::RefreshAll => read_all = true,
                    ConfigSourceUpdate::InternalError(e) => {
                        self.status.modify(vars, move |mut s| {
                            s.set_internal_error(e);
                        });
                    }
                }
            }
        }

        // run retained tasks
        self.tasks.retain_mut(|t| t(vars, &self.status));

        // Update config vars:
        // - Remove dropped vars.
        // - React to var assigns.
        // - Apply source requests.
        let mut var_tasks = vec![];
        self.vars.retain(|key, var| match var.upgrade(vars) {
            Some((any_var, write)) => {
                if write {
                    // var was set by the user, start a write task.
                    var_tasks.push(var.write(ConfigVarTaskArgs { vars, key, var: any_var }));
                } else if read_all || read.contains(key) {
                    // source notified a potential change, start a read task.
                    var_tasks.push(var.read(ConfigVarTaskArgs { vars, key, var: any_var }));
                }
                true // retain var
            }
            None => false, // var was dropped, remove entry
        });

        for task in var_tasks {
            task(self);
        }

        // update loaded alts.
        self.alts.retain(|alt| match alt.upgrade() {
            Some(alt) => {
                alt.borrow_mut().update(vars);
                true
            }
            None => false,
        })
    }

    /// Set the config source, replaces the previous source.
    pub fn load(&mut self, mut source: impl ConfigSource) {
        let (sender, receiver) = self.update.ext_channel();
        if !self.vars.is_empty() {
            let _ = sender.send(ConfigSourceUpdate::RefreshAll);
        }

        source.init(sender);
        self.source = Some((Box::new(source), receiver));
    }

    /// Open an alternative config source disconnected from the actual app source.
    #[must_use]
    pub fn load_alt(&mut self, source: impl ConfigSource) -> ConfigAlt {
        let e = ConfigAlt::load(self.update.clone(), source);
        self.alts.push(Rc::downgrade(&e.0));
        e
    }

    /// Gets a variable that tracks the source write tasks.
    pub fn status(&self) -> ReadOnlyRcVar<ConfigStatus> {
        self.status.clone().into_read_only()
    }

    /// Remove any errors set in the [`status`].
    ///
    /// [`status`]: Self::status
    pub fn clear_errors<Vw: WithVars>(&mut self, vars: &Vw) {
        vars.with_vars(|vars| {
            self.status.modify(vars, |mut s| {
                if s.has_errors() {
                    s.read_error = None;
                    s.write_error = None;
                    s.internal_error = None;
                }
            });
        })
    }

    /// Read the config value currently associated with the `key` if it is of the same type.
    ///
    /// Returns a [`ResponseVar`] that will update once when the value finishes reading.
    pub fn read<K, T>(&mut self, key: K) -> ResponseVar<Option<T>>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.read_impl(key.into())
    }
    fn read_impl<T>(&mut self, key: ConfigKey) -> ResponseVar<Option<T>>
    where
        T: ConfigValue,
    {
        // channel with the caller.
        let (responder, rsp) = response_var();

        self.read_raw(key, move |vars, r| {
            responder.respond(vars, r);
        });

        rsp
    }
    fn read_raw<T, R>(&mut self, key: ConfigKey, respond: R)
    where
        T: ConfigValue,
        R: FnOnce(&Vars, Option<T>) + 'static,
    {
        if let Some((source, _)) = &mut self.source {
            let mut task = Some(UiTask::new(&self.update, source.read(key)));
            let mut respond = Some(respond);
            self.tasks.push(Box::new(move |vars, status| {
                let finished = task.as_mut().unwrap().update().is_some();
                if finished {
                    match task.take().unwrap().into_result().unwrap() {
                        Ok(r) => {
                            let respond = respond.take().unwrap();
                            respond(vars, r.and_then(|v| serde_json::from_value(v).ok()));
                        }
                        Err(e) => {
                            status.modify(vars, move |mut s| {
                                s.set_read_error(e);
                            });

                            let respond = respond.take().unwrap();
                            respond(vars, None);
                        }
                    }
                }
                !finished
            }));
            let _ = self.update.send_ext_update();
        } else {
            // no source, just respond with `None`.
            self.once_tasks.push(Box::new(move |vars, _| {
                respond(vars, None);
            }));
            let _ = self.update.send_ext_update();
        }
    }

    /// Write the config value associated with the `key`.
    pub fn write<K, T>(&mut self, key: K, value: T)
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.write_impl(key.into(), value)
    }
    fn write_impl<T>(&mut self, key: ConfigKey, value: T)
    where
        T: ConfigValue,
    {
        // register variable update if the entry is observed.
        let key = match self.vars.entry(key) {
            Entry::Occupied(entry) => {
                let key = entry.key().clone();
                if let Some(var) = entry.get().downcast::<T>() {
                    let value = value.clone();

                    self.once_tasks.push(Box::new(move |vars, _| {
                        var.modify(vars, move |mut v| {
                            if v.value != value {
                                v.value = value;
                                v.write.set(false);
                            }
                        });
                    }));

                    let _ = self.update.send_ext_update();
                } else {
                    // not observed anymore or changed type.
                    entry.remove();
                }
                key
            }
            Entry::Vacant(e) => e.into_key(),
        };

        // serialize and request write.
        self.write_source(key, value);
    }
    fn write_source<T>(&mut self, key: ConfigKey, value: T)
    where
        T: ConfigValue,
    {
        if let Some((source, _)) = &mut self.source {
            match serde_json::value::to_value(value) {
                Ok(json) => {
                    let task = UiTask::new(&self.update, source.write(key, json));
                    self.track_write_task(task);
                }
                Err(e) => {
                    self.once_tasks.push(Box::new(move |vars, status| {
                        status.modify(vars, move |mut s| s.set_write_error(ConfigError::new(e)));
                    }));
                    let _ = self.update.send_ext_update();
                }
            }
        }
    }

    /// Remove the `key` from the persistent storage.
    ///
    /// Note that if a variable is connected with the `key` it stays connected with the same value, and if the variable
    /// is modified the `key` is reinserted. This should be called to remove obsolete configs only.
    pub fn remove<K: Into<ConfigKey>>(&mut self, key: K) {
        self.remove_impl(key.into())
    }
    fn remove_impl(&mut self, key: ConfigKey) {
        if let Some((source, _)) = &mut self.source {
            let task = UiTask::new(&self.update, source.remove(key));
            self.track_write_task(task);
        }
    }

    fn track_write_task(&mut self, task: UiTask<Result<(), ConfigError>>) {
        let mut count = 0;
        let mut task = Some(task);
        self.tasks.push(Box::new(move |vars, status| {
            let finished = task.as_mut().unwrap().update().is_some();
            if finished {
                let r = task.take().unwrap().into_result().unwrap();
                status.modify(vars, move |mut s| {
                    s.pending -= count;
                    if let Err(e) = r {
                        s.set_write_error(e);
                    }
                });
            } else if count == 0 {
                // first try, add pending.
                count = 1;
                status.modify(vars, |mut s| s.pending += 1);
            }

            !finished
        }));
        let _ = self.update.send_ext_update();
    }

    /// Gets a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. This is equivalent of a two-way binding between the config storage and the variable.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn var<K, T, D>(&mut self, key: K, default_value: D) -> impl Var<T>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
        D: FnOnce() -> T,
    {
        self.var_with_source(key.into(), default_value).map_ref_bidi(
            |v| &v.value,
            |v| {
                v.write.set(true);
                &mut v.value
            },
        )
    }

    /// Binds a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. If the `target` is dropped the binding is dropped.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn bind<Vw: WithVars, K: Into<ConfigKey>, T: ConfigValue, D: FnOnce() -> T, V: Var<T>>(
        &mut self,
        vars: &Vw,
        key: K,
        default_value: D,
        target: &V,
    ) -> VarBindingHandle {
        let source = self.var_with_source(key.into(), default_value);
        vars.with_vars(|vars| {
            if let Some(target) = target.actual_var(vars).downgrade() {
                vars.bind(move |vars, binding| {
                    if let Some(target) = target.upgrade() {
                        if let Some(v) = source.get_new(vars) {
                            // source updated, notify
                            let _ = target.set_ne(vars, v.value.clone());
                        }
                        if let Some(value) = target.clone_new(vars) {
                            // user updated, write
                            source.modify(vars, move |mut v| {
                                if v.value != value {
                                    Cell::set(&v.write, true);
                                    v.value = value;
                                }
                            });
                        }
                    } else {
                        // dropped target, drop binding
                        binding.unbind();
                    }
                })
            } else {
                VarBindingHandle::dummy()
            }
        })
    }

    fn var_with_source<T: ConfigValue>(&mut self, key: ConfigKey, default_value: impl FnOnce() -> T) -> RcVar<ValueWithSource<T>> {
        let refresh;

        let r = match self.vars.entry(key) {
            Entry::Occupied(mut entry) => {
                if let Some(var) = entry.get().downcast::<T>() {
                    return var; // already observed and is the same type.
                }

                // entry stale or for the wrong type:

                // re-insert observer
                let (cfg_var, var) = ConfigVar::new(default_value());
                *entry.get_mut() = cfg_var;

                // and refresh the value.
                refresh = (entry.key().clone(), var.clone());

                var
            }
            Entry::Vacant(entry) => {
                let (cfg_var, var) = ConfigVar::new(default_value());

                refresh = (entry.key().clone(), var.clone());

                entry.insert(cfg_var);

                var
            }
        };

        let (key, var) = refresh;
        let value = self.read::<_, T>(key);
        self.tasks.push(Box::new(move |vars, _| {
            if let Some(rsp) = value.rsp_clone(vars) {
                if let Some(value) = rsp {
                    var.modify(vars, move |mut v| {
                        if v.value != value {
                            v.value = value;
                            v.write.set(false);
                        }
                    });
                }
                false // task finished
            } else {
                true // retain
            }
        }));

        r
    }
}

/// Represents a loaded config source that is not the main config.
///
/// This type allows interaction with a [`ConfigSource`] just like the [`Config`] service, but without affecting the
/// actual app config, so that the same config key can be loaded  from different sources with different values.
///
/// Note that some config sources can auto-reload if their backing file is modified, so modifications using this type
/// can end-up affecting the actual [`Config`] too.
///
/// You can use the [`Config::load_alt`] method to create an instance of this type.
pub struct ConfigAlt(Rc<RefCell<Config>>);
impl ConfigAlt {
    fn load(updates: AppEventSender, source: impl ConfigSource) -> Self {
        let mut cfg = Config::new(updates);
        cfg.load(source);
        ConfigAlt(Rc::new(RefCell::new(cfg)))
    }

    /// Flush writes and unload.
    pub fn unload(self) {
        // drop
    }

    /// Gets a variable that tracks the source write tasks.
    pub fn status(&self) -> ReadOnlyRcVar<ConfigStatus> {
        self.0.borrow().status()
    }

    /// Remove any errors set in the [`status`].
    ///
    /// [`status`]: Self::status
    pub fn clear_errors<Vw: WithVars>(&mut self, vars: &Vw) {
        self.0.borrow_mut().clear_errors(vars)
    }

    /// Read the config value currently associated with the `key` if it is of the same type.
    ///
    /// Returns a [`ResponseVar`] that will update once when the value finishes reading.
    pub fn read<K, T>(&mut self, key: K) -> ResponseVar<Option<T>>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.0.borrow_mut().read(key)
    }

    /// Write the config value associated with the `key`.
    pub fn write<K, T>(&mut self, key: K, value: T)
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.0.borrow_mut().write(key, value)
    }

    /// Remove the `key` from the persistent storage.
    ///
    /// Note that if a variable is connected with the `key` it stays connected with the same value, and if the variable
    /// is modified the `key` is reinserted. This should be called to remove obsolete configs only.
    pub fn remove<K: Into<ConfigKey>>(&mut self, key: K) {
        self.0.borrow_mut().remove(key)
    }

    /// Gets a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. This is equivalent of a two-way binding between the config storage and the variable.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn var<K, T, D>(&mut self, key: K, default_value: D) -> impl Var<T>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
        D: FnOnce() -> T,
    {
        self.0.borrow_mut().var(key, default_value)
    }

    /// Binds a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. If the `target` is dropped the binding is dropped.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn bind<Vw: WithVars, K: Into<ConfigKey>, T: ConfigValue, D: FnOnce() -> T, V: Var<T>>(
        &mut self,
        vars: &Vw,
        key: K,
        default_value: D,
        target: &V,
    ) -> VarBindingHandle {
        Config::bind(&mut *self.0.borrow_mut(), vars, key, default_value, target)
    }
}
impl Drop for ConfigAlt {
    fn drop(&mut self) {
        if let Some((mut s, _)) = self.0.borrow_mut().source.take() {
            s.deinit();
        }
    }
}

type VarUpdateTask = Box<dyn FnOnce(&mut Config)>;

/// ConfigVar actual value, tracks if updates need to be send to source.
#[derive(Debug, Clone, PartialEq)]
struct ValueWithSource<T: ConfigValue> {
    value: T,
    write: Rc<Cell<bool>>,
}

struct ConfigVar {
    var: Box<dyn AnyWeakVar>,
    write: Rc<Cell<bool>>,
    run_task: Box<dyn Fn(ConfigVarTask, ConfigVarTaskArgs) -> VarUpdateTask>,
}
impl ConfigVar {
    fn new<T: ConfigValue>(initial_value: T) -> (Self, RcVar<ValueWithSource<T>>) {
        let write = Rc::new(Cell::new(false));
        let var = var(ValueWithSource {
            value: initial_value,
            write: write.clone(),
        });
        let r = ConfigVar {
            var: var.downgrade().into_any(),
            write,
            run_task: Box::new(ConfigVar::run_task_impl::<T>),
        };
        (r, var)
    }

    /// Returns var and if it needs to write.
    fn upgrade(&mut self, vars: &Vars) -> Option<(Box<dyn AnyVar>, bool)> {
        self.var.upgrade_any().map(|v| {
            let write = self.write.get() && v.is_new_any(vars);
            (v, write)
        })
    }

    fn downcast<T: ConfigValue>(&self) -> Option<RcVar<ValueWithSource<T>>> {
        self.var.as_any().downcast_ref::<types::WeakRcVar<ValueWithSource<T>>>()?.upgrade()
    }

    fn read(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Read, args)
    }
    fn write(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Write, args)
    }
    fn run_task_impl<T: ConfigValue>(task: ConfigVarTask, args: ConfigVarTaskArgs) -> VarUpdateTask {
        if let Some(var) = args.var.as_any().downcast_ref::<RcVar<ValueWithSource<T>>>() {
            match task {
                ConfigVarTask::Read => {
                    let key = args.key.clone();
                    let var = var.clone();
                    Box::new(move |config| {
                        config.read_raw::<T, _>(key, move |vars, value| {
                            if let Some(value) = value {
                                var.modify(vars, move |mut v| {
                                    if v.value != value {
                                        v.value = value;
                                        v.write.set(false);
                                    }
                                });
                            }
                        });
                    })
                }
                ConfigVarTask::Write => {
                    let key = args.key.clone();
                    let value = var.get_clone(args.vars).value;
                    Box::new(move |config| {
                        config.write_source(key, value);
                    })
                }
            }
        } else {
            Box::new(|_| {})
        }
    }
}

struct ConfigVarTaskArgs<'a> {
    vars: &'a Vars,
    key: &'a ConfigKey,
    var: Box<dyn AnyVar>,
}

enum ConfigVarTask {
    Read,
    Write,
}

/// Current [`Config`] status.
#[derive(Debug, Clone, Default)]
pub struct ConfigStatus {
    /// Number of pending writes.
    pub pending: usize,

    /// Last error during a read operation.
    pub read_error: Option<ConfigError>,
    /// Number of read errors.
    pub read_errors: u32,

    /// Last error during a write operation.
    pub write_error: Option<ConfigError>,
    /// Number of write errors.
    pub write_errors: u32,

    /// Last internal error.
    pub internal_error: Option<ConfigError>,
    /// Number of internal errors.
    pub internal_errors: u32,
}
impl ConfigStatus {
    /// Returns `true` if there are any errors currently in the status.
    ///
    /// The errors can be cleared using [`Config::clear_errors`].
    pub fn has_errors(&self) -> bool {
        self.read_error.is_some() || self.write_error.is_some() || self.internal_error.is_some()
    }

    fn set_read_error(&mut self, e: ConfigError) {
        self.read_error = Some(e);
        self.read_errors += 1;
    }

    fn set_write_error(&mut self, e: ConfigError) {
        self.write_error = Some(e);
        self.write_errors += 1;
    }

    fn set_internal_error(&mut self, e: ConfigError) {
        self.internal_error = Some(e);
        self.internal_errors += 1;
    }
}
impl fmt::Display for ConfigStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::cmp::Ordering::*;
        match self.pending.cmp(&1) {
            Equal => writeln!(f, "{} update pending…", self.pending)?,
            Greater => writeln!(f, "{} updates pending…", self.pending)?,
            Less => {}
        }

        if let Some(e) = &self.internal_error {
            write!(f, "internal error: ")?;
            fmt::Display::fmt(e, f)?;
            writeln!(f)?;
        }
        if let Some(e) = &self.read_error {
            write!(f, "read error: ")?;
            fmt::Display::fmt(e, f)?;
            writeln!(f)?;
        }
        if let Some(e) = &self.write_error {
            write!(f, "write error: ")?;
            fmt::Display::fmt(e, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Error in a [`ConfigSource`].
#[derive(Debug, Clone)]
pub struct ConfigError(pub Arc<dyn Error + Send + Sync>);
impl ConfigError {
    /// New error.
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Arc::new(error))
    }

    /// New error from string.
    pub fn new_str(error: impl Into<String>) -> Self {
        struct StringError(String);
        impl fmt::Debug for StringError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }
        impl fmt::Display for StringError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
        impl Error for StringError {}
        Self::new(StringError(error.into()))
    }
}
impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.0.source()
    }
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::new(e)
    }
}
impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::new(e)
    }
}
impl From<flume::RecvError> for ConfigError {
    fn from(e: flume::RecvError) -> Self {
        ConfigError::new(e)
    }
}

/// Represents an implementation of [`Config`].
pub trait ConfigSource: Send + 'static {
    /// Called once when the source is installed.
    fn init(&mut self, observer: AppExtSender<ConfigSourceUpdate>);

    /// Called once when the app is shutdown.
    ///
    /// Sources should block and flush all pending writes here.
    fn deinit(&mut self);

    /// Read the most recent value associated with `key` in the config source.
    fn read(&mut self, key: ConfigKey) -> BoxedFut<Result<Option<JsonValue>, ConfigError>>;

    /// Write the `value` for `key` in the config source.
    fn write(&mut self, key: ConfigKey, value: JsonValue) -> BoxedFut<Result<(), ConfigError>>;

    /// Remove the `key` in the config source.
    fn remove(&mut self, key: ConfigKey) -> BoxedFut<Result<(), ConfigError>>;
}

/// External updates in a [`ConfigSource`].
#[derive(Clone, Debug)]
pub enum ConfigSourceUpdate {
    /// Value associated with the key may have changed from an external event, **not** a write operation.
    Refresh(ConfigKey),
    /// All values may have changed.
    RefreshAll,
    /// Error not directly related to a read or write operation.
    ///
    /// If a full refresh is required after this a `RefreshAll` is send.
    InternalError(ConfigError),
}
