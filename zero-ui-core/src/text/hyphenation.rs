use std::path::PathBuf;

use hyphenation::{Hyphenator as _, Load as _};
use parking_lot::Mutex;

use super::Lang;
use crate::app_local;

app_local! {
    static HYPHENATION: Hyphenation = Hyphenation {
        #[cfg(feature = "hyphenation_embed_all")]
        source: Mutex::new(Some(Box::new(HyphenationDataEmbedded))),
        #[cfg(not(feature = "hyphenation_embed_all"))]
        source: Mutex::new(None),

        dictionaries: vec![],
    };
}

/// Hyphenation service.
///
/// Note that dictionary data is required to support a language, if the feature `"hyphenation_embed_all"` is enabled
/// dictionaries for all supported languages is embedded, otherwise dictionaries must be loaded using a [`HyphenationDataSource`].
///
/// You can use the [`HyphenationDataDir`] to use external files, see the [hyphenation](https://github.com/tapeinosyne/hyphenation)
/// for more details about the data files.
pub struct Hyphenation {
    source: Mutex<Option<Box<dyn HyphenationDataSource>>>,
    dictionaries: Vec<hyphenation::Standard>,
}
impl Hyphenation {
    /// Set the hyphenation dictionaries source.
    ///
    /// Clears all cached dictionaries.
    pub fn set_data_source(source: impl HyphenationDataSource) {
        let mut h = HYPHENATION.write();
        *h.source.get_mut() = Some(Box::new(source));
        h.dictionaries.clear();
    }

    /// Try to hyphenate the `word` using the `lang` dictionary and rules.
    ///
    /// Returns a vector of indexes that allow a line break.
    pub fn hyphenate(lang: &Lang, word: &str) -> Vec<usize> {
        Self::hyphenate_opt(lang, word).unwrap_or_default()
    }

    /// Try to hyphenate the `word` using the `lang` dictionary and rules.
    ///
    /// Returns a vector of indexes that allow a line break. Returns `None` if the `lang` is not supported or the `word` contains non-word characters.
    pub fn hyphenate_opt(lang: &Lang, word: &str) -> Option<Vec<usize>> {
        let lang = Self::lang_to_hyphenation_language(lang)?;

        if !util::WORD_REGEX.read().is_match(word) {
            return None;
        }

        {
            let h = HYPHENATION.read();
            for d in &h.dictionaries {
                if d.language() == lang {
                    return Some(d.hyphenate(word).breaks);
                }
            }
        }

        let mut h = HYPHENATION.write();
        // incase
        for d in &h.dictionaries {
            if d.language() == lang {
                return Some(d.hyphenate(word).breaks);
            }
        }

        if let Some(source) = h.source.get_mut() {
            let d = source.load(lang)?;
            let r = Some(d.hyphenate(word).breaks);
            h.dictionaries.push(d);

            return r;
        }

        None
    }

    /// Get the best [`hyphenation::Language`] for the `lang`.
    pub fn lang_to_hyphenation_language(lang: &Lang) -> Option<hyphenation::Language> {
        for (l, r) in &*util::LANG_TO_LANGUAGE_MAP.read() {
            if lang.matches(l, false, true) {
                return Some(*r);
            }
        }

        None
    }
}

/// Represents a hyphenation dictionary source.
///
/// The data source must be registered in [`Hyphenation::set_data_source`].
///
///
pub trait HyphenationDataSource: Send + 'static {
    /// Get the path to a dictionary file for the language.
    fn load(&mut self, lang: hyphenation::Language) -> Option<hyphenation::Standard>;
}

/// Represents a hyphenation data source that searches a directory.
///
/// The file names must follow a pattern that includes the [`hyphenation::Language`] display print, the pattern mut be defined
/// with a replacement `{lang}`. For example the file `dir/en-us.bincode` is matched by `"{lang}.bincode"`.
///
/// See the [hyphenation](https://github.com/tapeinosyne/hyphenation) crate docs for more details about the data files.
pub struct HyphenationDataDir {
    dir: PathBuf,
    name_pattern: &'static str,
}
impl HyphenationDataDir {
    /// New from `dir` and file name pattern.
    pub fn new(dir: PathBuf, name_pattern: &'static str) -> Self {
        HyphenationDataDir { dir, name_pattern }
    }
}
impl HyphenationDataSource for HyphenationDataDir {
    fn load(&mut self, lang: hyphenation::Language) -> Option<hyphenation::Standard> {
        let name = self.name_pattern.replace("{lang}", lang.to_string().as_str());
        let file = self.dir.join(name);
        if file.exists() {
            match hyphenation::Standard::from_path(lang, file) {
                Ok(d) => Some(d),
                Err(e) => {
                    tracing::error!("error loading hyphenation dictionary, {e}");
                    None
                }
            }
        } else {
            None
        }
    }
}

/// Represents embedded hyphenation data.
///
/// This is the default data source when compiled with the feature `"hyphenation_embed_all"`.
#[cfg(feature = "hyphenation_embed_all")]
pub struct HyphenationDataEmbedded;

#[cfg(feature = "hyphenation_embed_all")]
impl HyphenationDataSource for HyphenationDataEmbedded {
    fn load(&mut self, lang: hyphenation::Language) -> Option<hyphenation::Standard> {
        match hyphenation::Standard::from_embedded(lang) {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::error!("error loading hyphenation dictionary, {e}");
                None
            }
        }
    }
}

mod util {
    use super::*;
    use hyphenation::Language::*;
    use regex::Regex;

    app_local! {
        pub static LANG_TO_LANGUAGE_MAP: Vec<(Lang, hyphenation::Language)> = vec![
            (lang!("af"), Afrikaans),
            (lang!("sq"), Albanian),
            (lang!("hy"), Armenian),
            (lang!("as"), Assamese),
            (lang!("eu"), Basque),
            (lang!("be"), Belarusian),
            (lang!("bn"), Bengali),
            (lang!("bg"), Bulgarian),
            (lang!("ca"), Catalan),
            (lang!("zh-latn-pinyin"), Chinese),
            (lang!("cop"), Coptic),
            (lang!("hr"), Croatian),
            (lang!("cs"), Czech),
            (lang!("da"), Danish),
            (lang!("nl"), Dutch),
            (lang!("en-gb"), EnglishGB),
            (lang!("en-us"), EnglishUS),
            (lang!("eo"), Esperanto),
            (lang!("et"), Estonian),
            (lang!("mul-ethi"), Ethiopic),
            (lang!("fi"), Finnish),
            // (lang!("fi-x-school"), FinnishScholastic),
            (lang!("fr"), French),
            (lang!("fur"), Friulan),
            (lang!("gl"), Galician),
            (lang!("ka"), Georgian),
            (lang!("de-1901"), German1901),
            (lang!("de-1996"), German1996),
            (lang!("de-ch-1901"), GermanSwiss),
            (lang!("grc"), GreekAncient),
            (lang!("el-monoton"), GreekMono),
            (lang!("el-polyton"), GreekPoly),
            (lang!("gu"), Gujarati),
            (lang!("hi"), Hindi),
            (lang!("hu"), Hungarian),
            (lang!("is"), Icelandic),
            (lang!("id"), Indonesian),
            (lang!("ia"), Interlingua),
            (lang!("ga"), Irish),
            (lang!("it"), Italian),
            (lang!("kn"), Kannada),
            (lang!("kmr"), Kurmanji),
            (lang!("la"), Latin),
            // (lang!("la-x-classic"), LatinClassic),
            // (lang!("la-x-liturgic"), LatinLiturgical),
            (lang!("lv"), Latvian),
            (lang!("lt"), Lithuanian),
            (lang!("mk"), Macedonian),
            (lang!("ml"), Malayalam),
            (lang!("mr"), Marathi),
            (lang!("mn-cyrl"), Mongolian),
            (lang!("nb"), NorwegianBokmal),
            (lang!("nn"), NorwegianNynorsk),
            (lang!("oc"), Occitan),
            (lang!("or"), Oriya),
            (lang!("pi"), Pali),
            (lang!("pa"), Panjabi),
            (lang!("pms"), Piedmontese),
            (lang!("pl"), Polish),
            (lang!("pt"), Portuguese),
            (lang!("ro"), Romanian),
            (lang!("rm"), Romansh),
            (lang!("ru"), Russian),
            (lang!("sa"), Sanskrit),
            (lang!("sr-cyrl"), SerbianCyrillic),
            (lang!("sh-cyrl"), SerbocroatianCyrillic),
            (lang!("sh-latn"), SerbocroatianLatin),
            (lang!("cu"), SlavonicChurch),
            (lang!("sk"), Slovak),
            (lang!("sl"), Slovenian),
            (lang!("es"), Spanish),
            (lang!("sv"), Swedish),
            (lang!("ta"), Tamil),
            (lang!("te"), Telugu),
            (lang!("th"), Thai),
            (lang!("tr"), Turkish),
            (lang!("tk"), Turkmen),
            (lang!("uk"), Ukrainian),
            (lang!("hsb"), Uppersorbian),
            (lang!("cy"), Welsh),
        ];

        pub static WORD_REGEX: Regex = Regex::new(r"^\w+$").unwrap();
    }
}
