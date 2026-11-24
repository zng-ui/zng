use zng_ext_l10n::Lang;

/// Hyphenation service.
///
/// Note that embedded data is required to support a language, if the feature `"hypher_all"` is enabled
/// data for all supported languages is embedded (adds some 1.1MB of data), otherwise you must add a `hypher` dependency
/// and select specific languages.
///
/// ```toml
/// [dependencies]
/// # only embed English hyphenation data.
/// hypher = { version = "0.1", default-features = false, features = ["english"] }
/// ```
pub struct HYPHENATION;
impl HYPHENATION {
    /// Try to hyphenate the `word` using the `lang` rules.
    ///
    /// Returns a vector of indexes that allow a line break.
    pub fn hyphenate(&self, lang: &Lang, word: &str) -> Vec<usize> {
        self.hyphenate_opt(lang, word).unwrap_or_default()
    }

    /// Try to hyphenate the `word` using the `lang` rules.
    ///
    /// Returns a vector of indexes that allow a line break. Returns `None` if the `lang` is not supported or the
    /// `word` contains non-word characters.
    pub fn hyphenate_opt(&self, lang: &Lang, word: &str) -> Option<Vec<usize>> {
        let lang = self.lang_to_hypher_lang(lang)?;
        self.hyphenate_opt_lang(word, lang)
    }

    /// Get the best `hypher::Lang` for the `lang`.
    pub fn lang_to_hypher_lang(&self, lang: &Lang) -> Option<hypher::Lang> {
        hypher::Lang::from_iso(lang.0.language.as_str().as_bytes().try_into().ok()?)
    }

    /// Hyphenate with language already resolved.
    ///
    /// Returns `None` if the `word` contains non-word characters.
    pub fn hyphenate_opt_lang(&self, word: &str, lang: hypher::Lang) -> Option<Vec<usize>> {
        fn is_word(word: &str) -> bool {
            // r"^\w+$"
            !word.is_empty() && word.chars().all(|c| c.is_alphanumeric() || c == '_')
        }
        if !is_word(word) {
            return None;
        }

        let ptr = word.as_ptr() as usize;
        let breaks: Vec<_> = hypher::hyphenate(word, lang).skip(1).map(|s| s.as_ptr() as usize - ptr).collect();
        Some(breaks)
    }
}
