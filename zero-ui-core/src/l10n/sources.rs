use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use crate::{
    fs_watcher::WATCHER,
    l10n::FluentParserErrors,
    text::Txt,
    var::{types::WeakArcVar, *},
};

use super::{L10nSource, Lang, LangMap, LangResourceStatus};

/// Represents localization resources synchronized from files in a directory.
///
/// The expected directory layout is `{dir}/{lang}.flt` for lang only and `{dir}/{lang}/file.flt` for
/// lang with file.
pub struct L10nDir {
    dir: PathBuf,
    dir_watch: BoxedVar<Arc<LangMap<HashMap<Txt, PathBuf>>>>,
    dir_watch_status: BoxedVar<LangResourceStatus>,
    res: HashMap<(Lang, Txt), L10nFile>,
}
impl L10nDir {
    /// Start watching the `dir` for localization files.
    pub fn open(dir: impl Into<PathBuf>) -> Self {
        Self::new(dir.into())
    }
    fn new(dir: PathBuf) -> Self {
        let status = var(LangResourceStatus::Loading);
        let dir_watch = WATCHER.read_dir(
            dir.clone(),
            true,
            Arc::default(),
            clmv!(status, |d| {
                status.set_ne(LangResourceStatus::Loading);

                let mut set: LangMap<HashMap<Txt, PathBuf>> = LangMap::new();
                let mut errors: Vec<Arc<dyn std::error::Error + Send + Sync>> = vec![];
                let mut dir = None;
                for entry in d.min_depth(0).max_depth(1) {
                    match entry {
                        Ok(f) => {
                            let ty = f.file_type();
                            if dir.is_none() {
                                // get the watched dir
                                if !ty.is_dir() {
                                    tracing::error!("L10N path not a directory");
                                    status.set_ne(LangResourceStatus::NotAvailable);
                                    return None;
                                }
                                dir = Some(f.path().to_owned());
                            }

                            const EXT: unicase::Ascii<&'static str> = unicase::Ascii::new("ftl");

                            if ty.is_file() {
                                // match dir/lang.flt files
                                if let Some(name_and_ext) = f.file_name().to_str() {
                                    if let Some((name, ext)) = name_and_ext.rsplit_once('.') {
                                        if ext.is_ascii() && unicase::Ascii::new(ext) == EXT {
                                            // found .flt file.
                                            match Lang::from_str(name) {
                                                Ok(lang) => {
                                                    // and it is named correctly.
                                                    set.get_exact_or_insert(lang, Default::default)
                                                        .insert(Txt::from_str(""), dir.as_ref().unwrap().join(name_and_ext));
                                                }
                                                Err(e) => {
                                                    errors.push(Arc::new(e));
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if f.depth() == 1 && ty.is_dir() {
                                // match dir/lang/file.flt files
                                if let Some(name) = f.file_name().to_str() {
                                    match Lang::from_str(name) {
                                        Ok(lang) => {
                                            let inner = set.get_exact_or_insert(lang, Default::default);
                                            for entry in std::fs::read_dir(f.path()).into_iter().flatten() {
                                                match entry {
                                                    Ok(f) => {
                                                        if let Ok(name_and_ext) = f.file_name().into_string() {
                                                            if let Some((name, ext)) = name_and_ext.rsplit_once('.') {
                                                                if ext.is_ascii() && unicase::Ascii::new(ext) == EXT {
                                                                    // found .flt file.
                                                                    inner.insert(Txt::from_str(name), f.path());
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => errors.push(Arc::new(e)),
                                                }
                                            }
                                            if inner.is_empty() {
                                                set.pop();
                                            }
                                        }
                                        Err(e) => errors.push(Arc::new(e)),
                                    }
                                }
                            }
                        }
                        Err(e) => errors.push(Arc::new(e)),
                    }
                }

                if errors.is_empty() {
                    status.set_ne(LangResourceStatus::Loaded)
                } else {
                    let s = LangResourceStatus::Errors(errors);
                    tracing::error!("loading available {s}");
                    status.set(s)
                }

                Some(Arc::new(set))
            }),
        );

        Self {
            dir,
            dir_watch: dir_watch.boxed(),
            dir_watch_status: status.read_only().boxed(),
            res: HashMap::new(),
        }
    }
}
impl L10nSource for L10nDir {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<Txt, PathBuf>>>> {
        self.dir_watch.clone()
    }
    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        self.dir_watch_status.clone()
    }

    fn lang_resource(&mut self, lang: Lang, file: Txt) -> BoxedVar<Option<Arc<fluent::FluentResource>>> {
        match self.res.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(out) = e.get().res.upgrade() {
                    out
                } else {
                    let (lang, file) = e.key();
                    let out = load_file(e.get().status.clone(), &self.dir, lang, file);
                    e.get_mut().res = out.downgrade();
                    out
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut f = L10nFile::new();
                let (lang, file) = e.key();
                let out = load_file(f.status.clone(), &self.dir, lang, file);
                f.res = out.downgrade();
                e.insert(f);
                out
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: Txt) -> BoxedVar<LangResourceStatus> {
        self.res
            .entry((lang, file))
            .or_insert_with(L10nFile::new)
            .status
            .read_only()
            .boxed()
    }
}
struct L10nFile {
    res: BoxedWeakVar<Option<Arc<fluent::FluentResource>>>,
    status: ArcVar<LangResourceStatus>,
}
impl L10nFile {
    fn new() -> Self {
        Self {
            res: WeakArcVar::default().boxed(),
            status: var(LangResourceStatus::Loading),
        }
    }
}
fn load_file(status: ArcVar<LangResourceStatus>, dir: &Path, lang: &Lang, file: &Txt) -> BoxedVar<Option<Arc<fluent::FluentResource>>> {
    status.set_ne(LangResourceStatus::Loading);

    let path = if file.is_empty() {
        lang.to_string()
    } else {
        format!("{lang}/{file}")
    };

    WATCHER
        .read(dir.join(path), None, move |file| {
            status.set_ne(LangResourceStatus::Loading);

            match file.and_then(|mut f| f.string()) {
                Ok(flt) => match fluent::FluentResource::try_new(flt) {
                    Ok(flt) => {
                        status.set_ne(LangResourceStatus::Loaded);
                        // ok
                        return Some(Some(Arc::new(flt)));
                    }
                    Err(e) => {
                        let e = FluentParserErrors(e.1);
                        tracing::error!("error parsing fluent resource, {e}");
                        status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                    }
                },
                Err(e) => {
                    if matches!(e.kind(), io::ErrorKind::NotFound) {
                        status.set_ne(LangResourceStatus::NotAvailable);
                    } else {
                        tracing::error!("error loading fluent resource, {e}");
                        status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                    }
                }
            }
            // not ok
            Some(None)
        })
        .boxed()
}

/// Represents localization source that can swap the actual source without disconnected variables
/// taken on resources.
///
/// Note that [`L10N.load`] already uses this source internally.
///
/// [`L10N.load`]: super::L10N::load
pub struct SwapL10nSource {
    actual: Box<dyn L10nSource>,

    available_langs: ArcVar<Arc<LangMap<HashMap<Txt, PathBuf>>>>,
    available_langs_status: ArcVar<LangResourceStatus>,

    res: HashMap<(Lang, Txt), L10nFile>,
}
impl SwapL10nSource {
    /// New with [`NilL10nSource`].
    pub fn new() -> Self {
        Self {
            actual: Box::new(NilL10nSource),
            available_langs: var(Arc::default()),
            available_langs_status: var(LangResourceStatus::NotAvailable),
            res: HashMap::new(),
        }
    }

    /// Swaps the backend source with `source`.
    pub fn load(&mut self, source: impl L10nSource) {
        self.swap_source(Box::new(source))
    }
    fn swap_source(&mut self, new: Box<dyn L10nSource>) {
        self.actual = new;

        let actual_langs = self.actual.available_langs();
        self.available_langs.set(actual_langs.get());
        actual_langs.bind(&self.available_langs).perm();

        let actual_status = self.actual.available_langs_status();
        self.available_langs_status.set_ne(actual_status.get());
        actual_status.bind(&self.available_langs_status).perm();

        for ((lang, file), f) in &self.res {
            if let Some(res) = f.res.upgrade() {
                let actual_f = self.actual.lang_resource(lang.clone(), file.clone());
                // todo!("actual_f -> f.res, needs to drop when f.res drops");

                let actual_s = self.actual.lang_resource_status(lang.clone(), file.clone());
                f.status.set_ne(actual_s.get());
                actual_s.bind(&f.status).perm();
            } else {
                f.status.set_ne(LangResourceStatus::NotAvailable);
            }
        }
    }
}
impl Default for SwapL10nSource {
    fn default() -> Self {
        Self::new()
    }
}
impl L10nSource for SwapL10nSource {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<Txt, PathBuf>>>> {
        self.available_langs.read_only().boxed()
    }

    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        self.available_langs_status.read_only().boxed()
    }

    fn lang_resource(&mut self, lang: Lang, file: Txt) -> BoxedVar<Option<Arc<fluent::FluentResource>>> {
        match self.res.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(out) = e.get().res.upgrade() {
                    out
                } else {
                    let (lang, file) = e.key();
                    let out = self.actual.lang_resource(lang.clone(), file.clone());
                    e.get_mut().res = out.downgrade(); // !!: TODO, bind something here.
                    out
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut f = L10nFile::new();
                let (lang, file) = e.key();
                let out = self.actual.lang_resource(lang.clone(), file.clone());
                f.res = out.downgrade(); // !!: TODO, bind something here.
                e.insert(f);
                out
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: Txt) -> BoxedVar<LangResourceStatus> {
        self.res
            .entry((lang, file))
            .or_insert_with(L10nFile::new)
            .status
            .read_only()
            .boxed()
    }
}

/// Localization source that is never available.
pub struct NilL10nSource;
impl L10nSource for NilL10nSource {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<Txt, PathBuf>>>> {
        LocalVar(Arc::default()).boxed()
    }

    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        LocalVar(LangResourceStatus::NotAvailable).boxed()
    }

    fn lang_resource(&mut self, _: Lang, _: Txt) -> BoxedVar<Option<Arc<fluent::FluentResource>>> {
        LocalVar(None).boxed()
    }

    fn lang_resource_status(&mut self, _: Lang, _: Txt) -> BoxedVar<LangResourceStatus> {
        LocalVar(LangResourceStatus::NotAvailable).boxed()
    }
}
