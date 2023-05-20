//! Localization service [`L10N`] and helpers.
//!

use crate::{
    app::AppExtension,
    app_local,
    fs_watcher::WATCHER,
    text::Txt,
    var::{self, *},
};
use fluent::{types::FluentNumber, FluentResource};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fmt, io, mem, ops, path::PathBuf, str::FromStr, sync::Arc};

/// Localization service.
pub struct L10N;

/// Application extension that provides localization.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`L10N`]
#[derive(Default)]
pub struct L10nManager {}
impl AppExtension for L10nManager {
    fn update(&mut self) {
        L10N_SV.write().update();
    }
}

///<span data-del-macro-root></span> Gets a variable that localizes and formats the text in a widget context.
///
/// # Syntax
///
/// Macro expects a message ID string literal a *message template* string literal that is also used
/// as fallback, followed by optional named format arguments `arg = <arg>,..`.
///
/// The message string syntax is the [Fluent Project] syntax, interpolations in the form of `"{$var}"` are resolved to a local `$var`.
///
/// ```
/// # use zero_ui_core::{l10n::*, var::*};
/// let _scope = zero_ui_core::app::App::minimal();
/// let name = var("World");
/// let msg = l10n!("msg-id", "Hello {$name}!");
/// ```
///
/// # Scrapper
///
/// The `zero-ui-l10n-scrapper` tool can be used to collect all localizable text of Rust code files, it is a text based search that
/// matches this macro name and the two first input literals, avoid renaming this macro to support scrapping, otherwise you will
/// have to declare the message file manually.
///
/// The scrapper also has some support for comments, if the previous code line from a [`l10n!`] call is a comment starting with
/// prefix `l10n: #comment` the `#comment` is collected, same for a suffix comment in the same line of the [`l10n!`] call.
///
/// [Fluent Project]: https://projectfluent.org/fluent/guide/
#[macro_export]
macro_rules! l10n {
    ($message_id:tt, $message:tt $(,)?) => {
        $crate::l10n::__l10n! {
            l10n_path { $crate::l10n }
            message_id { $message_id }
            message { $message }
        }
    };
    ($message_id:tt, $message:tt, $($arg:ident = $arg_expr:expr),* $(,)?) => {
        {
            $(
                let $arg = $arg_expr;
            )*
            $crate::l10n::__l10n! {
                l10n_path { $crate::l10n }
                message_id { $message_id }
                message { $message }
            }
        }
    };
    ($($error:tt)*) => {
        std::compile_error!(r#"expected ("id", "message") or ("id", "msg {$arg}", arg=expr)"#)
    }
}
#[doc(inline)]
pub use l10n;

#[doc(hidden)]
pub use zero_ui_proc_macros::l10n as __l10n;

impl L10N {
    /// Start watching the `dir` for `"dir/{locale}.ftl"` files.
    ///
    /// The [`available_langs`] variable maintains an up-to-date list of locale files found, the files
    /// are only loaded when needed, and also are watched to update automatically.
    ///
    /// [`available_langs`]: Self::available_langs
    pub fn load_dir(&self, dir: impl Into<PathBuf>) {
        L10N_SV.write().load_dir(dir.into());
    }

    /// Available localization files.
    pub fn available_langs(&self) -> ReadOnlyArcVar<Arc<LangMap<PathBuf>>> {
        L10N_SV.read().available_langs.read_only()
    }

    /// Gets a read-write variable that sets the preferred languages for the app scope.
    /// Lang not available are ignored until they become available, the first language in the
    /// vec is the most preferred.
    ///
    /// Note that the [`LANG_VAR`] is used in message requests, the default value of that
    /// context variable is this one.
    pub fn app_lang(&self) -> ArcVar<Langs> {
        L10N_SV.read().app_lang.clone()
    }

    /// Gets a variable that is a localized message identified by `id` in the localization context
    /// where the variable is first used. The variable will update when the contextual language changes.
    ///
    /// If the message has variable arguments they must be provided using [`L10nMessageBuilder::arg`], the
    /// returned variable will also update when the arg variables update.
    ///
    /// The `id` can be compound with an attribute `"msg-id.attribute"`, the `fallback` is used
    /// when the message is not found in the localization context.
    ///
    /// Prefer using the [`l10n!`] macro instead of this method, the macro does compile time validation.
    pub fn message(&self, id: impl Into<Txt>, fallback: impl Into<Txt>) -> L10nMessageBuilder {
        L10N_SV.write().message(id.into(), fallback.into())
    }

    /// Function called by `l10n!`.
    #[doc(hidden)]
    pub fn l10n_message(&self, id: &'static str, fallback: &'static str) -> L10nMessageBuilder {
        self.message(Txt::from_static(id), Txt::from_static(fallback))
    }

    /// Gets a formatted message var localized to a given `lang`.
    ///
    /// The returned variable is read-only and will update when the backing resource changes and when the `args` variables change.
    ///
    /// The `lang` resource is lazy loaded and stays in memory only when there are variables alive linked to it, each lang
    /// in the list is matched to available resources if no match is available the `fallback` message is used. The variable
    /// may temporary contain the `fallback` as lang resources are loaded asynchrony.
    pub fn localized_messsage(
        &self,
        lang: impl Into<Langs>,
        id: impl Into<Txt>,
        fallback: impl Into<Txt>,
        args: impl Into<Vec<(Txt, BoxedVar<L10nArgument>)>>,
    ) -> ReadOnlyArcVar<Txt> {
        L10N_SV.write().message_text(lang.into(), id.into(), fallback.into(), args.into())
    }

    /// Gets a handle to the `lang` resource.
    ///
    /// The resource will be loaded and stay in memory until all clones of the handle are dropped, this
    /// can be used to pre-load resources so that localized messages find it immediately avoiding flashing
    /// the fallback text in the UI.
    ///
    /// If the resource directory or file changes it is auto-reloaded, just like when a message variable
    /// held on the resource does.
    pub fn lang_resource(&self, lang: impl Into<Lang>) -> LangResourceHandle {
        L10N_SV.write().lang_resource(lang.into())
    }
}

/// Handle to localization resources for a language.
///
/// See [`L10N.lang_resource`] for more details.
///
/// [`L10N.lang_resource`]: L10N::lang_resource
#[derive(Clone)]
pub struct LangResourceHandle(crate::crate_util::Handle<ArcVar<LangResourceStatus>>);
impl fmt::Debug for LangResourceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LangResourceHandle({:?})", self.status().get())
    }
}
impl LangResourceHandle {
    /// Localization resource status.
    ///
    /// This can change after load if [`L10N.load_dir`] is called to set a different dir, or the resource
    /// file is created in the dir.
    ///  
    /// [`L10N.load_dir`]: L10N::load_dir
    pub fn status(&self) -> ReadOnlyArcVar<LangResourceStatus> {
        self.0.data().read_only()
    }

    /// Wait for the resource to load, if it is available.
    pub async fn wait(&self) {
        let status = self.status();
        while matches!(status.get(), LangResourceStatus::Loading) {
            status.wait_is_new().await;
        }
    }

    /// Drop the handle without dropping the resource.
    ///
    /// The localization resource will stay in memory for duration of the current process, if the
    /// resource file changes it will automatically reload.
    ///
    /// [`L10N.load_dir`]: L10N::load_dir
    pub fn perm(self) {
        self.0.perm()
    }
}

/// Status of a localization resource.
#[derive(Clone, Debug)]
pub enum LangResourceStatus {
    /// Resource not available.
    ///
    /// This can change if the localization directory changes, or the file is created.
    NotAvailable,
    /// Resource is loading.
    Loading,
    /// Resource loaded ok.
    Loaded,
    /// Resource failed to load.
    ///
    /// This can be any IO or parse errors. If the resource if *not found* the status is set to
    /// `NotAvailable`, not an error. Localization messages fallback on error just like they do
    /// for not available.
    Error(Arc<dyn std::error::Error + Send + Sync>),
}
impl fmt::Display for LangResourceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangResourceStatus::NotAvailable => write!(f, "not available"),
            LangResourceStatus::Loading => write!(f, "loading…"),
            LangResourceStatus::Loaded => write!(f, "loaded"),
            LangResourceStatus::Error(e) => write!(f, "error: {e}"),
        }
    }
}

/// Localized message variable builder.
///
/// See [`L10N.message`] for more details.
pub struct L10nMessageBuilder {
    id: Txt,
    fallback: Txt,
    args: Vec<(Txt, BoxedVar<L10nArgument>)>,
}
impl L10nMessageBuilder {
    /// Add a format arg variable.
    pub fn arg(mut self, name: Txt, value: impl IntoVar<L10nArgument>) -> Self {
        self.args.push((name, value.into_var().boxed()));
        self
    }
    #[doc(hidden)]
    pub fn l10n_arg(self, name: &'static str, value: impl Var<L10nArgument>) -> Self {
        self.arg(Txt::from_static(name), value)
    }

    /// Build the variable.
    pub fn build(self) -> impl Var<Txt> {
        let Self { id, fallback, args } = self;
        LANG_VAR.flat_map(move |l| L10N.localized_messsage(l.clone(), id.clone(), fallback.clone(), args.clone()))
    }
}

/// Represents an argument value for a localization message.
///
/// See [`L10nMessageBuilder::arg`] for more details.
#[derive(Clone, Debug)]
pub enum L10nArgument {
    /// String.
    Txt(Txt),
    /// Number, with optional style details.
    Number(FluentNumber),
}
impl_from_and_into_var! {
    fn from(txt: Txt) -> L10nArgument {
        L10nArgument::Txt(txt)
    }
    fn from(txt: &'static str) -> L10nArgument {
        L10nArgument::Txt(Txt::from_static(txt))
    }
    fn from(txt: String) -> L10nArgument {
        L10nArgument::Txt(Txt::from(txt))
    }
    fn from(t: char) -> L10nArgument {
        L10nArgument::Txt(Txt::from_char(t))
    }
    fn from(number: FluentNumber) -> L10nArgument {
        L10nArgument::Number(number)
    }

}
macro_rules! impl_from_and_into_var_number {
    ($($literal:tt),+) => {
        impl_from_and_into_var! {
            $(
                fn from(number: $literal) -> L10nArgument {
                    FluentNumber::from(number).into()
                }
            )+
        }
    }
}
impl_from_and_into_var_number! {
    u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize, f32, f64
}

#[doc(hidden)]
pub struct L10nSpecialize<T>(pub T);
#[doc(hidden)]
pub trait IntoL10nVar {
    type Var: Var<L10nArgument>;
    fn into_l10n_var(self) -> Self::Var;
}

impl<T: Into<L10nArgument>> IntoL10nVar for L10nSpecialize<T> {
    type Var = var::LocalVar<L10nArgument>;

    fn into_l10n_var(self) -> Self::Var {
        var::LocalVar(self.0.into())
    }
}
impl<T: VarValue + Into<L10nArgument>> IntoL10nVar for &L10nSpecialize<ArcVar<T>> {
    type Var = var::types::ContextualizedVar<L10nArgument, var::ReadOnlyArcVar<L10nArgument>>;

    fn into_l10n_var(self) -> Self::Var {
        self.0.map_into()
    }
}
impl<V: Var<L10nArgument>> IntoL10nVar for &&L10nSpecialize<V> {
    type Var = V;

    fn into_l10n_var(self) -> Self::Var {
        self.0.clone()
    }
}

context_var! {
    /// Language of text in a widget context.
    ///
    /// Is [`L10N.app_lang`] by default.
    ///
    /// [`L10N.app_lang`]: L10N::app_lang
    pub static LANG_VAR: Langs = L10N.app_lang();
}

/// Identifies the language, region and script of text.
///
/// Use the [`lang!`] macro to construct one, it does compile-time validation.
///
/// Use the [`unic_langid`] crate for more advanced operations such as runtime parsing and editing identifiers, this
/// type is just an alias for the core struct of that crate.
///
/// [`unic_langid`]: https://docs.rs/unic-langid
pub type Lang = unic_langid::LanguageIdentifier;

/// List of languages, in priority order.
///
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Langs(pub Vec<Lang>);
impl Langs {
    /// The first lang on the list or `und` if the list is empty.
    pub fn best(&self) -> &Lang {
        static NONE: Lazy<Lang> = Lazy::new(|| lang!(und));
        self.first().unwrap_or(&NONE)
    }
}
impl ops::Deref for Langs {
    type Target = Vec<Lang>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for Langs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl_from_and_into_var! {
    fn from(lang: Lang) -> Langs {
        Langs(vec![lang])
    }
}

/// Represents a map of [`Lang`] keys that can be partially matched.
#[derive(Debug, Clone)]
pub struct LangMap<V> {
    inner: Vec<(Lang, V)>,
}
impl<V> Default for LangMap<V> {
    fn default() -> Self {
        Self { inner: Default::default() }
    }
}
impl<V> LangMap<V> {
    /// New empty default.
    pub fn new() -> Self {
        LangMap::default()
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        LangMap {
            inner: Vec::with_capacity(capacity),
        }
    }

    fn exact_i(&self, lang: &Lang) -> Option<usize> {
        for (i, (key, _)) in self.inner.iter().enumerate() {
            if key == lang {
                return Some(i);
            }
        }
        None
    }

    fn best_i(&self, lang: &Lang) -> Option<usize> {
        let mut best = None;
        let mut best_weight = 0;

        for (i, (key, _)) in self.inner.iter().enumerate() {
            if lang.matches(key, true, true) {
                let mut weight = 1;
                let mut eq = 0;

                if key.language == lang.language {
                    weight += 128;
                    eq += 1;
                }
                if key.region == lang.region {
                    weight += 40;
                    eq += 1;
                }
                if key.script == lang.script {
                    weight += 20;
                    eq += 1;
                }

                if eq == 3 && lang.variants().zip(key.variants()).all(|(a, b)| a == b) {
                    return Some(i);
                }

                if best_weight < weight {
                    best_weight = weight;
                    best = Some(i);
                }
            }
        }

        best
    }

    /// Returns the best match to `lang` currently in the map.
    pub fn best_match(&self, lang: &Lang) -> Option<&Lang> {
        if let Some(i) = self.best_i(lang) {
            Some(&self.inner[i].0)
        } else {
            None
        }
    }

    /// Returns the best match for `lang`.
    pub fn get(&self, lang: &Lang) -> Option<&V> {
        if let Some(i) = self.best_i(lang) {
            Some(&self.inner[i].1)
        } else {
            None
        }
    }

    /// Returns the exact match for `lang`.
    pub fn get_exact(&self, lang: &Lang) -> Option<&V> {
        if let Some(i) = self.exact_i(lang) {
            Some(&self.inner[i].1)
        } else {
            None
        }
    }

    /// Returns the best match for `lang`.
    pub fn get_mut(&mut self, lang: &Lang) -> Option<&V> {
        if let Some(i) = self.best_i(lang) {
            Some(&self.inner[i].1)
        } else {
            None
        }
    }

    /// Returns the exact match for `lang`.
    pub fn get_exact_mut(&mut self, lang: &Lang) -> Option<&V> {
        if let Some(i) = self.exact_i(lang) {
            Some(&self.inner[i].1)
        } else {
            None
        }
    }

    /// Insert the value with the exact match of `lang`.
    ///
    /// Returns the previous exact match.
    pub fn insert(&mut self, lang: Lang, value: V) -> Option<V> {
        if let Some(i) = self.exact_i(&lang) {
            Some(mem::replace(&mut self.inner[i].1, value))
        } else {
            self.inner.push((lang, value));
            None
        }
    }

    /// Remove the exact match of `lang`.
    pub fn remove(&mut self, lang: &Lang) -> Option<V> {
        if let Some(i) = self.exact_i(lang) {
            Some(self.inner.swap_remove(i).1)
        } else {
            None
        }
    }

    /// Remove all exact and partial matches of `lang`.
    ///
    /// Returns a count of items removed.
    pub fn remove_all(&mut self, lang: &Lang) -> usize {
        let mut count = 0;
        self.inner.retain(|(key, _)| {
            let rmv = lang.matches(key, true, false);
            if rmv {
                count += 1
            }
            !rmv
        });
        count
    }

    /// Returns if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of languages in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Iterator over lang keys.
    pub fn keys(&self) -> impl std::iter::ExactSizeIterator<Item = &Lang> {
        self.inner.iter().map(|(k, _)| k)
    }

    /// Iterator over values.
    pub fn values(&self) -> impl std::iter::ExactSizeIterator<Item = &V> {
        self.inner.iter().map(|(_, v)| v)
    }

    /// Iterator over values.
    pub fn values_mut(&mut self) -> impl std::iter::ExactSizeIterator<Item = &mut V> {
        self.inner.iter_mut().map(|(_, v)| v)
    }

    /// Into iterator of values.
    pub fn into_values(self) -> impl std::iter::ExactSizeIterator<Item = V> {
        self.inner.into_iter().map(|(_, v)| v)
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl std::iter::ExactSizeIterator<Item = (&Lang, &V)> {
        self.inner.iter().map(|(k, v)| (k, v))
    }

    /// Iterate over key-value pairs with mutable values.
    pub fn iter_mut(&mut self) -> impl std::iter::ExactSizeIterator<Item = (&Lang, &mut V)> {
        self.inner.iter_mut().map(|(k, v)| (&*k, v))
    }
}
impl<V> IntoIterator for LangMap<V> {
    type Item = (Lang, V);

    type IntoIter = std::vec::IntoIter<(Lang, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// <span data-del-macro-root></span> Compile-time validated [`Lang`] value.
///
/// The language is parsed during compile and any errors are emitted as compile time errors.
///
/// # Syntax
///
/// The input can be a single a single string literal with `-` separators, or a single ident with `_` as the separators.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::l10n::lang;
/// let en_us = lang!(en_US);
/// let en = lang!(en);
///
/// assert!(en.matches(&en_us, true, false));
/// assert_eq!(en_us, lang!("en-US"));
/// ```
#[macro_export]
macro_rules! lang {
    ($($tt:tt)+) => {
        {
            let lang: $crate::l10n::unic_langid::LanguageIdentifier = $crate::l10n::__lang!($($tt)+);
            lang
        }
    }
}
#[doc(inline)]
pub use crate::lang;

#[doc(hidden)]
pub use zero_ui_proc_macros::lang as __lang;

#[doc(hidden)]
pub use unic_langid;

struct L10nService {
    available_langs: ArcVar<Arc<LangMap<PathBuf>>>,
    app_lang: ArcVar<Langs>,

    dir_watcher: Option<ReadOnlyArcVar<Arc<LangMap<PathBuf>>>>,
    file_watchers: HashMap<Lang, LangResourceWatcher>,
    messages: HashMap<(Langs, Txt), MessageRequest>,
}
impl L10nService {
    fn new() -> Self {
        Self {
            available_langs: var(Arc::new(LangMap::new())),
            app_lang: var(Langs::default()),
            dir_watcher: None,
            file_watchers: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    fn load_dir(&mut self, dir: PathBuf) {
        let dir_watch = WATCHER.read_dir(dir, true, Arc::default(), |d| {
            let mut set = LangMap::new();
            let mut dir = None;
            for entry in d.min_depth(0).max_depth(1) {
                match entry {
                    Ok(f) => {
                        let ty = f.file_type();
                        if dir.is_none() {
                            // get the watched dir
                            if !ty.is_dir() {
                                tracing::error!("L10N path not a directory");
                                return None;
                            }
                            dir = Some(f.path().to_owned());
                        }
                        // search $.flt files in the dir
                        if ty.is_file() {
                            if let Some(name_and_ext) = f.file_name().to_str() {
                                if let Some((name, ext)) = name_and_ext.rsplit_once('.') {
                                    const EXT: unicase::Ascii<&'static str> = unicase::Ascii::new("flt");
                                    if ext.is_ascii() && unicase::Ascii::new(ext) == EXT {
                                        // found .flt file.
                                        match Lang::from_str(name) {
                                            Ok(lang) => {
                                                // and it is named correctly.
                                                set.insert(lang, dir.as_ref().unwrap().with_file_name(name_and_ext));
                                            }
                                            Err(e) => {
                                                if name != "template" {
                                                    tracing::debug!("`{name}.{ext}` is not a valid lang or 'template', {e}")
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => tracing::error!("L10N dir watcher error, {e}"),
                }
            }
            Some(Arc::new(set))
        });
        self.available_langs.set(dir_watch.get());
        dir_watch.bind(&self.available_langs).perm();
        self.dir_watcher = Some(dir_watch);
    }

    fn message(&mut self, id: Txt, fallback: Txt) -> L10nMessageBuilder {
        L10nMessageBuilder {
            id,
            fallback,
            args: vec![],
        }
    }

    fn message_text(&mut self, lang: Langs, id: Txt, fallback: Txt, args: Vec<(Txt, BoxedVar<L10nArgument>)>) -> ReadOnlyArcVar<Txt> {
        match self.messages.entry((lang, id)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(txt) = e.get().text.upgrade() {
                    // already requested
                    txt.read_only()
                } else {
                    // already requested and dropped, reload.
                    let handles = e
                        .key()
                        .0
                        .iter()
                        .map(|l| Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, l.clone()))
                        .collect();
                    let (r, txt) = MessageRequest::new(fallback, args, handles, &e.key().0, &e.key().1, &self.file_watchers);
                    *e.get_mut() = r;
                    txt
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                // not request, load.
                let handles = e
                    .key()
                    .0
                    .iter()
                    .map(|l| Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, l.clone()))
                    .collect();
                let (r, txt) = MessageRequest::new(fallback, args, handles, &e.key().0, &e.key().1, &self.file_watchers);
                e.insert(r);
                txt
            }
        }
    }

    fn lang_resource(&mut self, lang: Lang) -> LangResourceHandle {
        Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, lang)
    }
    fn lang_resource_impl(
        file_watchers: &mut HashMap<Lang, LangResourceWatcher>,
        available_langs: &ArcVar<Arc<LangMap<PathBuf>>>,
        lang: Lang,
    ) -> LangResourceHandle {
        file_watchers
            .entry(lang)
            .or_insert_with_key(|lang| {
                let lang = lang.clone();
                if let Some(file) = available_langs.get().get_exact(&lang) {
                    LangResourceWatcher::new(lang, file.clone())
                } else {
                    LangResourceWatcher::new_not_available(lang)
                }
            })
            .handle()
    }

    fn update(&mut self) {
        if let Some(watcher) = &self.dir_watcher {
            if let Some(available_langs) = watcher.get_new() {
                // renew watchers, keeps the same handlers
                for (lang, watcher) in self.file_watchers.iter_mut() {
                    let handle = watcher.handle.take().unwrap();
                    *watcher = if let Some(file) = available_langs.get_exact(lang) {
                        LangResourceWatcher::new_with_handle(lang.clone(), file.clone(), handle)
                    } else {
                        LangResourceWatcher::new_not_available_with_handle(lang.clone(), handle)
                    };
                }
            }
        } else {
            // no dir loaded
            return;
        }

        self.messages.retain(|k, request| request.update(&k.0, &k.1, &self.file_watchers));

        self.file_watchers.retain(|_, watcher| watcher.retain());
    }
}
app_local! {
    static L10N_SV: L10nService = L10nService::new();
}

struct LangResourceWatcher {
    handle: Option<crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>>,
    bundle: ReadOnlyArcVar<ArcFluentBundle>,
}
impl LangResourceWatcher {
    fn new(lang: Lang, file: PathBuf) -> Self {
        let status = var(LangResourceStatus::Loading);
        Self::new_with_handle(lang, file, crate::crate_util::Handle::new(status).0)
    }

    fn new_not_available(lang: Lang) -> Self {
        let status = var(LangResourceStatus::NotAvailable);
        Self::new_not_available_with_handle(lang, crate::crate_util::Handle::new(status).0)
    }

    fn new_with_handle(lang: Lang, file: PathBuf, handle: crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>) -> Self {
        let init = ConcurrentFluentBundle::new_concurrent(vec![lang.clone()]);
        let status = handle.data();
        let bundle = WATCHER.read(
            file,
            ArcFluentBundle::new(init),
            clmv!(status, |file| {
                status.set(LangResourceStatus::Loading);

                match file.and_then(|mut f| f.string()) {
                    Ok(flt) => match FluentResource::try_new(flt) {
                        Ok(flt) => {
                            let mut bundle = ConcurrentFluentBundle::new_concurrent(vec![lang.clone()]);
                            bundle.add_resource_overriding(flt);
                            status.set(LangResourceStatus::Loaded);
                            // ok
                            return Some(ArcFluentBundle::new(bundle));
                        }
                        Err(e) => {
                            let e = FluentParserErrors(e.1);
                            tracing::error!("error parsing fluent resource, {e}");
                            status.set(LangResourceStatus::Error(Arc::new(e)));
                        }
                    },
                    Err(e) => {
                        if matches!(e.kind(), io::ErrorKind::NotFound) {
                            status.set(LangResourceStatus::NotAvailable);
                        } else {
                            tracing::error!("error loading fluent resource, {e}");
                            status.set(LangResourceStatus::Error(Arc::new(e)));
                        }
                    }
                }
                // not ok
                None
            }),
        );
        Self {
            handle: Some(handle),
            bundle,
        }
    }

    fn new_not_available_with_handle(lang: Lang, handle: crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>) -> Self {
        handle.data().set(LangResourceStatus::NotAvailable);
        Self {
            handle: Some(handle),
            bundle: var({
                let init = ConcurrentFluentBundle::new_concurrent(vec![lang]);
                ArcFluentBundle::new(init)
            })
            .read_only(),
        }
    }

    fn handle(&self) -> LangResourceHandle {
        LangResourceHandle(self.handle.as_ref().unwrap().reanimate())
    }

    fn retain(&self) -> bool {
        !self.handle.as_ref().unwrap().is_dropped()
    }
}

type ConcurrentFluentBundle = fluent::bundle::FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>;

#[derive(Clone)]
struct ArcFluentBundle(Arc<ConcurrentFluentBundle>);
impl fmt::Debug for ArcFluentBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArcFluentBundle")
    }
}
impl ops::Deref for ArcFluentBundle {
    type Target = ConcurrentFluentBundle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ArcFluentBundle {
    pub fn new(bundle: ConcurrentFluentBundle) -> Self {
        Self(Arc::new(bundle))
    }
}

/// Errors found parsing a fluent resource file.
#[derive(Clone, Debug)]
pub struct FluentParserErrors(pub Vec<fluent_syntax::parser::ParserError>);
impl fmt::Display for FluentParserErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for e in &self.0 {
            write!(f, "{sep}{e}")?;
            sep = "\n";
        }
        Ok(())
    }
}
impl std::error::Error for FluentParserErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if self.0.len() == 1 {
            Some(&self.0[0])
        } else {
            None
        }
    }
}

struct MessageRequest {
    text: crate::var::types::WeakArcVar<Txt>,
    fallback: Txt,
    args: Vec<(Txt, BoxedVar<L10nArgument>)>,

    resource_handles: Box<[LangResourceHandle]>,
    current_resource: usize,
}
impl MessageRequest {
    fn new(
        fallback: Txt,
        args: Vec<(Txt, BoxedVar<L10nArgument>)>,
        resource_handles: Box<[LangResourceHandle]>,

        langs: &Langs,
        key: &Txt,
        resources: &HashMap<Lang, LangResourceWatcher>,
    ) -> (Self, ReadOnlyArcVar<Txt>) {
        let mut text = None;
        let mut current_resource = resource_handles.len();

        for (i, h) in resource_handles.iter().enumerate() {
            if matches!(h.status().get(), LangResourceStatus::Loaded) {
                let bundle = &resources.get(&langs[i]).unwrap().bundle;
                if bundle.with(|b| b.has_message(key)) {
                    // found something already loaded

                    // TODO, format resource to init the var, return.
                    text = Some(var(Txt::empty()));
                    current_resource = i;
                    break;
                }
            }
        }

        let text = text.unwrap_or_else(|| {
            // not available resource yet

            // TODO, format fallback to init var.
            var(Txt::empty())
        });

        let r = Self {
            text: text.downgrade(),
            fallback,
            args,
            resource_handles,
            current_resource,
        };

        (r, text.read_only())
    }

    fn update(&mut self, langs: &Langs, key: &Txt, resources: &HashMap<Lang, LangResourceWatcher>) -> bool {
        if let Some(txt) = self.text.upgrade() {
            for (i, h) in self.resource_handles.iter().enumerate() {
                if matches!(h.status().get(), LangResourceStatus::Loaded) {
                    let bundle = &resources.get(&langs[i]).unwrap().bundle;
                    if bundle.with(|b| b.has_message(key)) {
                        //  found best
                        if self.current_resource != i || bundle.is_new() || self.args.iter().any(|a| a.1.is_new()) {
                            self.current_resource = i;

                            // TODO, format resource
                            let _ = txt;
                        }
                        return true;
                    }
                }
            }

            // fallback
            if self.current_resource != self.resource_handles.len() || self.args.iter().any(|a| a.1.is_new()) {
                self.current_resource = self.resource_handles.len();

                // TODO, format fallback
                let _ = (txt, &self.fallback);
            }

            true
        } else {
            false
        }
    }
}
