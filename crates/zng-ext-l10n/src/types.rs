use std::{borrow::Cow, fmt, mem, ops, sync::Arc};

use fluent::types::FluentNumber;
use once_cell::sync::Lazy;
use zng_ext_fs_watcher::WatcherReadStatus;
use zng_layout::context::LayoutDirection;
use zng_txt::Txt;
use zng_var::{context_var, impl_from_and_into_var, ArcEq, ArcVar, BoxedVar, IntoVar, LocalVar, ReadOnlyArcVar, Var, VarValue};

use crate::{lang, service::L10N_SV, L10N};

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

type StatusError = Vec<Arc<dyn std::error::Error + Send + Sync>>;

/// Localized message variable builder.
///
/// See [`L10N.message`] for more details.
///
/// [`L10N.message`]: L10N::message
pub struct L10nMessageBuilder {
    pub(super) pkg_name: Txt,
    pub(super) file: Txt,
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

    /// Build the variable.
    pub fn build(self) -> impl Var<Txt> {
        let Self {
            pkg_name,
            file,
            id,
            attribute,
            fallback,
            args,
        } = self;
        let _ = pkg_name; // !!: TODO, include in file? what about version?
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
    pub fn fluent_value(&self) -> fluent::FluentValue {
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
#[derive(PartialEq, Eq, Hash, Clone, Default, PartialOrd, Ord)]
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
#[derive(Clone, PartialEq, Eq, Default, Hash)]
pub struct Langs(pub Vec<Lang>);
impl fmt::Debug for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DisplayLangs<'a>(&'a [Lang]);
        impl<'a> fmt::Debug for DisplayLangs<'a> {
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
    #[allow(clippy::map_identity)] // false positive, already fixed https://github.com/rust-lang/rust-clippy/pull/11792
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
        if self.0.len() == 1 {
            Some(&self.0[0])
        } else {
            None
        }
    }
}
