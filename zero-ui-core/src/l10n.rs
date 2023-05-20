//! Localization service [`L10N`] and helpers.
//!

use crate::{
    app::AppExtension,
    app_local,
    fs_watcher::WATCHER,
    text::Txt,
    var::{self, *},
};
use fluent::types::FluentNumber;
use once_cell::sync::Lazy;
use std::{mem, ops, path::PathBuf, str::FromStr, sync::Arc};

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
impl AppExtension for L10nManager {}

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
    pub fn message_text(
        &self,
        lang: impl Into<Langs>,
        id: impl Into<Txt>,
        fallback: impl Into<Txt>,
        args: impl Into<Vec<(Txt, BoxedVar<L10nArgument>)>>,
    ) -> BoxedVar<Txt> {
        L10N_SV.write().message_text(lang.into(), id.into(), fallback.into(), args.into())
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
        LANG_VAR.flat_map(move |l| L10N.message_text(l.clone(), id.clone(), fallback.clone(), args.clone()))
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    watcher: Option<ReadOnlyArcVar<Arc<LangMap<PathBuf>>>>,
}
impl L10nService {
    fn new() -> Self {
        Self {
            available_langs: var(Arc::new(LangMap::new())),
            app_lang: var(Langs::default()),
            watcher: None,
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
        self.watcher = Some(dir_watch);
    }

    fn message(&mut self, id: Txt, fallback: Txt) -> L10nMessageBuilder {
        L10nMessageBuilder {
            id,
            fallback,
            args: vec![],
        }
    }

    fn message_text(&mut self, _lang: Langs, _id: Txt, fallback: Txt, _args: Vec<(Txt, BoxedVar<L10nArgument>)>) -> BoxedVar<Txt> {
        // TODO, register variable in service
        crate::var::LocalVar(fallback).boxed()
    }
}
app_local! {
    static L10N_SV: L10nService = L10nService::new();
}
