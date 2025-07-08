use std::{borrow::Cow, collections::HashMap, fmt, mem, ops, path::PathBuf, sync::Arc};

use fluent::types::FluentNumber;
use once_cell::sync::Lazy;
use semver::Version;
use zng_ext_fs_watcher::WatcherReadStatus;
use zng_layout::context::LayoutDirection;
use zng_txt::{ToTxt, Txt};
use zng_var::{ArcEq, ArcVar, BoxedVar, IntoVar, LocalVar, ReadOnlyArcVar, Var, VarValue, context_var, impl_from_and_into_var};

use crate::{L10N, lang, service::L10N_SV};

/// Handle to multiple localization resources.
#[derive(Clone, Debug)]
pub struct LangResources(pub Vec<LangResource>);
impl ops::Deref for LangResources {
    type Target = Vec<LangResource>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for LangResources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl LangResources {
    /// Wait for all the resources to load.
    pub async fn wait(&self) {
        for res in &self.0 {
            res.wait().await;
        }
    }

    /// Drop all handles without dropping the resource.
    pub fn perm(self) {
        for res in self.0 {
            res.perm()
        }
    }
}

/// Handle to a localization resource.
#[derive(Clone)]
#[must_use = "resource can unload if dropped"]
pub struct LangResource {
    pub(super) res: BoxedVar<Option<ArcEq<fluent::FluentResource>>>,
    pub(super) status: BoxedVar<LangResourceStatus>,
}

impl fmt::Debug for LangResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LangResource")
            .field("status", &self.status.get())
            .finish_non_exhaustive()
    }
}
impl LangResource {
    /// Read-only variable with the resource.
    pub fn resource(&self) -> &BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
        &self.res
    }

    /// Read-only variable with the resource status.
    pub fn status(&self) -> &BoxedVar<LangResourceStatus> {
        &self.status
    }

    /// Drop the handle without unloading the resource.
    pub fn perm(self) {
        L10N_SV.write().push_perm_resource(self);
    }

    /// Await resource status to not be loading.
    pub async fn wait(&self) {
        while matches!(self.status.get(), LangResourceStatus::Loading) {
            self.status.wait_update().await;
        }
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
    Errors(StatusError),
}
impl fmt::Display for LangResourceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangResourceStatus::NotAvailable => write!(f, "not available"),
            LangResourceStatus::Loading => write!(f, "loading…"),
            LangResourceStatus::Loaded => write!(f, "loaded"),
            LangResourceStatus::Errors(e) => {
                writeln!(f, "errors:")?;
                for e in e {
                    writeln!(f, "   {e}")?;
                }
                Ok(())
            }
        }
    }
}
impl PartialEq for LangResourceStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Errors(a), Self::Errors(b)) => a.is_empty() && b.is_empty(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for LangResourceStatus {}
impl WatcherReadStatus<StatusError> for LangResourceStatus {
    fn idle() -> Self {
        Self::Loaded
    }

    fn reading() -> Self {
        Self::Loading
    }

    fn read_error(e: StatusError) -> Self {
        Self::Errors(e)
    }
}
impl WatcherReadStatus<LangResourceStatus> for LangResourceStatus {
    fn idle() -> Self {
        Self::Loaded
    }

    fn reading() -> Self {
        Self::Loading
    }

    fn read_error(e: LangResourceStatus) -> Self {
        e
    }
}

type StatusError = Vec<Arc<dyn std::error::Error + Send + Sync>>;

/// Localized message variable builder.
///
/// See [`L10N.message`] for more details.
///
/// [`L10N.message`]: L10N::message
pub struct L10nMessageBuilder {
    pub(super) file: LangFilePath,
    pub(super) id: Txt,
    pub(super) attribute: Txt,
    pub(super) fallback: Txt,
    pub(super) args: Vec<(Txt, BoxedVar<L10nArgument>)>,
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

    /// Build the message var for the given languages.
    pub fn build_for(self, lang: impl Into<Langs>) -> impl Var<Txt> {
        L10N_SV
            .write()
            .localized_message(lang.into(), self.file, self.id, self.attribute, self.fallback, self.args)
    }

    /// Build the message var for the contextual language.
    pub fn build(self) -> impl Var<Txt> {
        let Self {
            file,
            id,
            attribute,
            fallback,
            args,
        } = self;
        LANG_VAR.flat_map(move |l| {
            L10N_SV.write().localized_message(
                l.clone(),
                file.clone(),
                id.clone(),
                attribute.clone(),
                fallback.clone(),
                args.clone(),
            )
        })
    }
}

/// Represents an argument value for a localization message.
///
/// See [`L10nMessageBuilder::arg`] for more details.
#[derive(Clone, Debug, PartialEq)]
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
    fn from(b: bool) -> L10nArgument {
        b.to_txt().into()
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
impl L10nArgument {
    /// Borrow argument as a fluent value.
    pub fn fluent_value(&self) -> fluent::FluentValue<'_> {
        match self {
            L10nArgument::Txt(t) => fluent::FluentValue::String(Cow::Borrowed(t.as_str())),
            L10nArgument::Number(n) => fluent::FluentValue::Number(n.clone()),
        }
    }
    /// Clone argument as a fluent value.
    pub fn to_fluent_value(&self) -> fluent::FluentValue<'static> {
        match self {
            L10nArgument::Txt(t) => fluent::FluentValue::String(Cow::Owned(t.to_string())),
            L10nArgument::Number(n) => fluent::FluentValue::Number(n.clone()),
        }
    }
}

#[doc(hidden)]
pub struct L10nSpecialize<T>(pub Option<T>);
#[doc(hidden)]
pub trait IntoL10nVar {
    type Var: Var<L10nArgument>;
    fn to_l10n_var(&mut self) -> Self::Var;
}

impl<T: Into<L10nArgument>> IntoL10nVar for L10nSpecialize<T> {
    type Var = LocalVar<L10nArgument>;

    fn to_l10n_var(&mut self) -> Self::Var {
        LocalVar(self.0.take().unwrap().into())
    }
}
impl<T: VarValue + Into<L10nArgument>> IntoL10nVar for &mut L10nSpecialize<ArcVar<T>> {
    type Var = ReadOnlyArcVar<L10nArgument>;

    fn to_l10n_var(&mut self) -> Self::Var {
        self.0.take().unwrap().map_into()
    }
}
impl<V: Var<L10nArgument>> IntoL10nVar for &mut &mut L10nSpecialize<V> {
    type Var = V;

    fn to_l10n_var(&mut self) -> Self::Var {
        self.0.take().unwrap()
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
#[derive(PartialEq, Eq, Hash, Clone, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Lang(pub unic_langid::LanguageIdentifier);
impl Lang {
    /// Returns character direction of the language.
    pub fn direction(&self) -> LayoutDirection {
        crate::from_unic_char_direction(self.0.character_direction())
    }

    /// Compares a language to another allowing for either side to use the missing fields as wildcards.
    ///
    /// This allows for matching between `en` (treated as `en-*-*-*`) and `en-US`.
    pub fn matches(&self, other: &Self, self_as_range: bool, other_as_range: bool) -> bool {
        self.0.matches(&other.0, self_as_range, other_as_range)
    }
}
impl ops::Deref for Lang {
    type Target = unic_langid::LanguageIdentifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Debug for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::str::FromStr for Lang {
    type Err = unic_langid::LanguageIdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(lang!(und));
        }
        unic_langid::LanguageIdentifier::from_str(s).map(Lang)
    }
}

/// List of languages, in priority order.
#[derive(Clone, PartialEq, Eq, Default, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Langs(pub Vec<Lang>);
impl fmt::Debug for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DisplayLangs<'a>(&'a [Lang]);
        impl fmt::Debug for DisplayLangs<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.iter()).finish()
            }
        }
        if f.alternate() {
            f.debug_tuple("Langs").field(&DisplayLangs(&self.0)).finish()
        } else {
            fmt::Debug::fmt(&DisplayLangs(&self.0), f)
        }
    }
}
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
    fn from(lang: Option<Lang>) -> Langs {
        Langs(lang.into_iter().collect())
    }
}
impl fmt::Display for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for l in self.iter() {
            write!(f, "{sep}{l}")?;
            sep = ", ";
        }
        Ok(())
    }
}
impl std::str::FromStr for Langs {
    type Err = unic_langid::LanguageIdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Ok(Langs(vec![]));
        }
        let mut r = Self(vec![]);
        for lang in s.split(',') {
            r.0.push(lang.trim().parse()?)
        }
        Ok(r)
    }
}

/// Represents a map of [`Lang`] keys that can be partially matched.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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
    pub fn get_mut(&mut self, lang: &Lang) -> Option<&mut V> {
        if let Some(i) = self.best_i(lang) {
            Some(&mut self.inner[i].1)
        } else {
            None
        }
    }

    /// Returns the exact match for `lang`.
    pub fn get_exact_mut(&mut self, lang: &Lang) -> Option<&mut V> {
        if let Some(i) = self.exact_i(lang) {
            Some(&mut self.inner[i].1)
        } else {
            None
        }
    }

    /// Returns the current value or insert `new` and return a reference to it.
    pub fn get_exact_or_insert(&mut self, lang: Lang, new: impl FnOnce() -> V) -> &mut V {
        if let Some(i) = self.exact_i(&lang) {
            return &mut self.inner[i].1;
        }
        let i = self.inner.len();
        self.inner.push((lang, new()));
        &mut self.inner[i].1
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

    /// Remove the last inserted lang.
    pub fn pop(&mut self) -> Option<(Lang, V)> {
        self.inner.pop()
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
impl<V> LangMap<HashMap<LangFilePath, V>> {
    /// Returns the match for `lang` and `file`.
    pub fn get_file(&self, lang: &Lang, file: &LangFilePath) -> Option<&V> {
        let files = self.get(lang)?;
        if let Some(exact) = files.get(file) {
            return Some(exact);
        }
        Self::best_file(files, file).map(|(_, v)| v)
    }

    /// Returns the best match to `lang` and `file` currently in the map.
    pub fn best_file_match(&self, lang: &Lang, file: &LangFilePath) -> Option<&LangFilePath> {
        let files = self.get(lang)?;
        if let Some((exact, _)) = files.get_key_value(file) {
            return Some(exact);
        }
        Self::best_file(files, file).map(|(k, _)| k)
    }

    fn best_file<'a>(files: &'a HashMap<LangFilePath, V>, file: &LangFilePath) -> Option<(&'a LangFilePath, &'a V)> {
        let mut best = None;
        let mut best_dist = u64::MAX;
        for (k, v) in files {
            if let Some(d) = k.matches(file) {
                if d < best_dist {
                    best = Some((k, v));
                    best_dist = d;
                }
            }
        }
        best
    }
}
impl<V> IntoIterator for LangMap<V> {
    type Item = (Lang, V);

    type IntoIter = std::vec::IntoIter<(Lang, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
impl<V: PartialEq> PartialEq for LangMap<V> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (k, v) in &self.inner {
            if other.get_exact(k) != Some(v) {
                return false;
            }
        }
        true
    }
}
impl<V: Eq> Eq for LangMap<V> {}

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
        if self.0.len() == 1 { Some(&self.0[0]) } else { None }
    }
}

/// Localization resource file path in the localization directory.
///
/// In the default directory layout, localization dependencies are collected using `cargo zng l10n`
/// and copied to `l10n/{lang}/deps/{name}/{version}/`, and localization for the app ([`is_current_app`])
/// is placed in `l10n/{lang}/`.
///
/// [`is_current_app`]: Self::is_current_app
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct LangFilePath {
    /// Package name.
    pub pkg_name: Txt,
    /// Package version.
    pub pkg_version: Version,
    /// The localization file name, without extension.
    pub file: Txt,
}
impl Ord for LangFilePath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_pkg = self.actual_pkg_data();
        let other_pkg = other.actual_pkg_data();
        match self_pkg.0.cmp(other_pkg.0) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self_pkg.1.cmp(other_pkg.1) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.file().cmp(&other.file())
    }
}
impl PartialOrd for LangFilePath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::hash::Hash for LangFilePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.actual_pkg_data().hash(state);
        self.file().hash(state);
    }
}
impl Eq for LangFilePath {}
impl PartialEq for LangFilePath {
    fn eq(&self, other: &Self) -> bool {
        self.actual_pkg_data() == other.actual_pkg_data() && self.file() == other.file()
    }
}
impl LangFilePath {
    /// New from package name, version and file.
    pub fn new(pkg_name: impl Into<Txt>, pkg_version: Version, file: impl Into<Txt>) -> Self {
        let r = Self {
            pkg_name: pkg_name.into(),
            pkg_version,
            file: file.into(),
        };
        // these non-standard names are matched on fallback, but they can cause duplicate caching
        debug_assert!(
            r.file
                .rsplit_once('.')
                .map(|(_, ext)| !ext.eq_ignore_ascii_case("ftl"))
                .unwrap_or(true),
            "file `{}` must not have extension",
            r.file
        );
        debug_assert!(r.file != "_", "file `_` should be an empty string");
        r
    }

    /// Gets a file in the current app.
    ///
    /// This value indicates that the localization resources are directly on the `l10n/{lang?}/` directories, not
    /// in the dependencies directories.
    ///
    /// See [`zng_env::about()`] for more details.
    pub fn current_app(file: impl Into<Txt>) -> LangFilePath {
        let about = zng_env::about();
        Self::new(about.pkg_name.clone(), about.version.clone(), file.into())
    }

    /// Gets if this file is in the [`current_app`] resources, or the `pkg_name` is empty or the `pkg_version.pre` is `#.#.#-local`.
    ///
    /// [`current_app`]: Self::current_app
    pub fn is_current_app(&self) -> bool {
        self.is_current_app_no_check() || {
            let about = zng_env::about();
            self.pkg_name == about.pkg_name && self.pkg_version == about.version
        }
    }

    fn is_current_app_no_check(&self) -> bool {
        self.pkg_name.is_empty() || self.pkg_version.pre.as_str() == "local"
    }

    fn actual_pkg_data(&self) -> (&Txt, &Version) {
        if self.is_current_app_no_check() {
            let about = zng_env::about();
            (&about.pkg_name, &about.version)
        } else {
            (&self.pkg_name, &self.pkg_version)
        }
    }

    /// Gets the normalized package name.
    ///
    /// This is the app package name if [`is_current_app`], otherwise is just the `pkg_name` value.
    ///
    /// [`is_current_app`]: Self::is_current_app
    pub fn pkg_name(&self) -> Txt {
        self.actual_pkg_data().0.clone()
    }

    /// Gets the normalized package version.
    ///
    /// This is the app version if [`is_current_app`], otherwise is just the `pkg_version` value.
    ///
    /// [`is_current_app`]: Self::is_current_app
    pub fn pkg_version(&self) -> Version {
        self.actual_pkg_data().1.clone()
    }

    /// Gets the normalized file name.
    ///
    /// This `"_"` for empty file or the file.
    pub fn file(&self) -> Txt {
        if self.file.is_empty() {
            Txt::from_char('_')
        } else {
            self.file.clone()
        }
    }

    /// Get the file path, relative to the localization dir.
    ///
    /// * Empty file name is the same as `_`.
    /// * If package [`is_current_app`] gets `{lang}/{file}.ftl`.
    /// * Else if is another package gets `{lang}/deps/{pkg_name}/{pkg_version}/{file}.ftl`.
    ///
    /// [`is_current_app`]: Self::is_current_app
    pub fn to_path(&self, lang: &Lang) -> PathBuf {
        let mut file = self.file.as_str();
        if file.is_empty() {
            file = "_";
        }
        if self.is_current_app() {
            format!("{lang}/{file}.ftl")
        } else {
            format!("{lang}/deps/{}/{}/{file}.ftl", self.pkg_name, self.pkg_version)
        }
        .into()
    }

    /// Gets a value that indicates if the resources represented by `self` can be used for `search`.
    ///
    /// The number indicates the quality of the match:
    ///
    /// * `0` is an exact match.
    /// * `b1` is a match with only version `build` differences.
    /// * `b10` is a match with only version `pre` differences.
    /// * `(0..u16::MAX) << 16` is a match with only `patch` differences and the absolute distance.
    /// * `(0..u16::MAX) << 16 * 2` is a match with `minor` differences and the absolute distance.
    /// * `(0..u16::MAX) << 16 * 3` is a match with `major` differences and the absolute distance.
    /// * `None`` is a `pkg_name` mismatch.
    pub fn matches(&self, search: &Self) -> Option<u64> {
        let (self_name, self_version) = self.actual_pkg_data();
        let (search_name, search_version) = search.actual_pkg_data();

        if self_name != search_name {
            return None;
        }

        if self.file != search.file {
            let file_a = self.file.rsplit_once('.').map(|t| t.0).unwrap_or(self.file.as_str());
            let file_b = search.file.rsplit_once('.').map(|t| t.0).unwrap_or(self.file.as_str());
            if file_a != file_b {
                let is_empty_a = file_a == "_" || file_a.is_empty();
                let is_empty_b = file_b == "_" || file_b.is_empty();
                if !(is_empty_a && is_empty_b) {
                    return None;
                }
            }
            tracing::warn!(
                "fallback matching `{}` with `{}`, file was not expected to have extension",
                self.file,
                search.file
            )
        }

        fn dist(a: u64, b: u64, shift: u64) -> u64 {
            let (l, s) = match a.cmp(&b) {
                std::cmp::Ordering::Equal => return 0,
                std::cmp::Ordering::Less => (b, a),
                std::cmp::Ordering::Greater => (a, b),
            };

            (l - s).min(u16::MAX as u64) << (16 * shift)
        }

        let mut d = 0;
        if self_version.build != search_version.build {
            d = 1;
        }
        if self_version.pre != search_version.pre {
            d |= 0b10;
        }

        d |= dist(self_version.patch, search_version.patch, 1);
        d |= dist(self_version.minor, search_version.minor, 2);
        d |= dist(self_version.major, search_version.major, 3);

        Some(d)
    }
}
impl_from_and_into_var! {
    fn from(file: Txt) -> LangFilePath {
        LangFilePath::current_app(file)
    }

    fn from(file: &'static str) -> LangFilePath {
        LangFilePath::current_app(file)
    }

    fn from(file: String) -> LangFilePath {
        LangFilePath::current_app(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_matches() {
        fn check(a: &str, b: &str, c: &str) {
            let ap = LangFilePath::new("name", a.parse().unwrap(), "file");
            let bp = LangFilePath::new("name", b.parse().unwrap(), "file");
            let cp = LangFilePath::new("name", c.parse().unwrap(), "file");

            let ab = ap.matches(&bp);
            let ac = ap.matches(&cp);

            assert!(ab < ac, "expected {a}.matches({b}) < {a}.matches({c})")
        }

        check("0.0.0", "0.0.1", "0.1.0");
        check("0.0.1", "0.1.0", "1.0.0");
        check("0.0.0-pre", "0.0.0-pre+build", "0.0.0-other+build");
        check("0.0.0+build", "0.0.0+build", "0.0.0+other");
        check("0.0.1", "0.0.2", "0.0.3");
        check("0.1.0", "0.2.0", "0.3.0");
        check("1.0.0", "2.0.0", "3.0.0");
        check("1.0.0", "1.1.0", "2.0.0");
    }
}
