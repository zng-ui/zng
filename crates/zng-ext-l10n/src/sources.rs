use std::{collections::HashMap, io, path::PathBuf, str::FromStr, sync::Arc};

use zng_clone_move::clmv;
use zng_ext_fs_watcher::WATCHER;
use zng_txt::Txt;
use zng_var::{types::WeakArcVar, var, AnyVar, ArcEq, ArcVar, BoxedVar, BoxedWeakVar, LocalVar, Var, VarHandle, WeakVar};

use crate::{FluentParserErrors, L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

/// Represents localization resources synchronized from files in a directory.
///
/// The expected directory layout is `{dir}/{lang}.ftl` for lang only and `{dir}/{lang}/{file}.ftl` for
/// lang with files. The `{dir}/{lang}/_.ftl` file is also a valid "lang only" file.
pub struct L10nDir {
    dir_watch: BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    dir_watch_status: BoxedVar<LangResourceStatus>,
    res: HashMap<(Lang, LangFilePath), L10nFile>,
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
                status.set(LangResourceStatus::Loading);

                let mut set: LangMap<HashMap<LangFilePath, PathBuf>> = LangMap::new();
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
                                    status.set(LangResourceStatus::NotAvailable);
                                    return None;
                                }
                                dir = Some(f.path().to_owned());
                            }

                            const EXT: unicase::Ascii<&'static str> = unicase::Ascii::new("ftl");

                            if ty.is_file() {
                                // match dir/lang.ftl files
                                if let Some(name_and_ext) = f.file_name().to_str() {
                                    if let Some((name, ext)) = name_and_ext.rsplit_once('.') {
                                        if ext.is_ascii() && unicase::Ascii::new(ext) == EXT {
                                            tracing::debug!("found {name}.{ext}");
                                            // found .ftl file.
                                            match Lang::from_str(name) {
                                                Ok(lang) => {
                                                    // and it is named correctly.
                                                    set.get_exact_or_insert(lang, Default::default)
                                                        .insert(LangFilePath::current_app(""), dir.as_ref().unwrap().join(name_and_ext));
                                                }
                                                Err(e) => {
                                                    errors.push(Arc::new(e));
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if f.depth() == 1 && ty.is_dir() {
                                // match dir/lang/file.ftl files
                                if let Some(dir_name) = f.file_name().to_str() {
                                    match Lang::from_str(dir_name) {
                                        Ok(lang) => {
                                            let inner = set.get_exact_or_insert(lang, Default::default);
                                            for entry in std::fs::read_dir(f.path()).into_iter().flatten() {
                                                match entry {
                                                    Ok(f) => {
                                                        if let Ok(name_and_ext) = f.file_name().into_string() {
                                                            if let Some((mut name, ext)) = name_and_ext.rsplit_once('.') {
                                                                if ext.is_ascii() && unicase::Ascii::new(ext) == EXT {
                                                                    // found .ftl file.
                                                                    tracing::debug!("found {dir_name}/{name}.{ext}");

                                                                    if name == "_" {
                                                                        name = "";
                                                                    }
                                                                    inner.insert(LangFilePath::current_app(Txt::from_str(name)), f.path());
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
                                        Err(e) => {
                                            tracing::trace!("dir {dir_name}/ is not l10n, {e}");
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => errors.push(Arc::new(e)),
                    }
                }

                if errors.is_empty() {
                    // Loaded set by `dir_watch` to avoid race condition in wait.
                } else {
                    let s = LangResourceStatus::Errors(errors);
                    tracing::error!("'loading available' {s}");
                    status.set(s)
                }

                Some(Arc::new(set))
            }),
        );
        dir_watch.bind_map(&status, |_| LangResourceStatus::Loaded).perm();

        Self {
            dir_watch: dir_watch.boxed(),
            dir_watch_status: status.read_only().boxed(),
            res: HashMap::new(),
        }
    }
}
impl L10nSource for L10nDir {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.dir_watch.clone()
    }
    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        self.dir_watch_status.clone()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
        match self.res.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(out) = e.get().res.upgrade() {
                    out
                } else {
                    let (lang, file) = e.key();
                    let out = resource_var(&self.dir_watch, e.get().status.clone(), lang.clone(), file.clone());
                    e.get_mut().res = out.downgrade();
                    out
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut f = L10nFile::new();
                let (lang, file) = e.key();
                let out = resource_var(&self.dir_watch, f.status.clone(), lang.clone(), file.clone());
                f.res = out.downgrade();
                e.insert(f);
                out
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<LangResourceStatus> {
        self.res
            .entry((lang, file))
            .or_insert_with(L10nFile::new)
            .status
            .read_only()
            .boxed()
    }
}
struct L10nFile {
    res: BoxedWeakVar<Option<ArcEq<fluent::FluentResource>>>,
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

fn resource_var(
    dir_watch: &BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    status: ArcVar<LangResourceStatus>,
    lang: Lang,
    file: LangFilePath,
) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
    dir_watch
        .map(move |w| w.get(&lang).and_then(|m| m.get(&file)).cloned())
        .flat_map(move |p| match p {
            Some(p) => {
                status.set(LangResourceStatus::Loading);

                let r = WATCHER.read(
                    p.clone(),
                    None,
                    clmv!(status, |file| {
                        status.set(LangResourceStatus::Loading);

                        match file.and_then(|mut f| f.string()) {
                            Ok(flt) => match fluent::FluentResource::try_new(flt) {
                                Ok(flt) => {
                                    // ok
                                    // Loaded set by `r` to avoid race condition in waiter.
                                    return Some(Some(ArcEq::new(flt)));
                                }
                                Err(e) => {
                                    let e = FluentParserErrors(e.1);
                                    tracing::error!("error parsing fluent resource, {e}");
                                    status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                                }
                            },
                            Err(e) => {
                                if matches!(e.kind(), io::ErrorKind::NotFound) {
                                    status.set(LangResourceStatus::NotAvailable);
                                } else {
                                    tracing::error!("error loading fluent resource, {e}");
                                    status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                                }
                            }
                        }
                        // not ok
                        Some(None)
                    }),
                );
                r.bind_map(&status, |_| LangResourceStatus::Loaded).perm();
                r.boxed()
            }
            None => LocalVar(None).boxed(),
        })
}

/// Represents localization source that can swap the actual source without disconnecting variables
/// taken on resources.
///
/// Note that [`L10N.load`] already uses this source internally.
///
/// [`L10N.load`]: super::L10N::load
pub struct SwapL10nSource {
    actual: Box<dyn L10nSource>,

    available_langs: ArcVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    available_langs_status: ArcVar<LangResourceStatus>,

    res: HashMap<(Lang, LangFilePath), SwapFile>,
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
        self.available_langs.set_from(&actual_langs);
        actual_langs.bind(&self.available_langs).perm();

        let actual_status = self.actual.available_langs_status();
        self.available_langs_status.set_from(&actual_status);
        actual_status.bind(&self.available_langs_status).perm();

        for ((lang, file), f) in &mut self.res {
            if let Some(res) = f.res.upgrade() {
                let actual_f = self.actual.lang_resource(lang.clone(), file.clone());
                f.actual_weak_res = actual_f.bind(&res); // weak ref to `res` is held by `actual_f`
                f.res_strong_actual = res.hook_any(Box::new(move |_| {
                    // strong ref to `actual_f` is held by `res`.
                    let _hold = &actual_f;
                    true
                }));

                let actual_s = self.actual.lang_resource_status(lang.clone(), file.clone());
                f.status.set_from(&actual_s);
                f.actual_weak_status = actual_s.bind(&f.status);
            } else {
                f.status.set(LangResourceStatus::NotAvailable);
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
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.available_langs.read_only().boxed()
    }

    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        self.available_langs_status.read_only().boxed()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
        match self.res.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(res) = e.get().res.upgrade() {
                    res
                } else {
                    let (lang, file) = e.key();
                    let actual_f = self.actual.lang_resource(lang.clone(), file.clone());
                    let actual_s = self.actual.lang_resource_status(lang.clone(), file.clone());

                    let f = e.get_mut();

                    let res = var(actual_f.get());
                    f.actual_weak_res = actual_f.bind(&res); // weak ref to `res` is held by `actual_f`
                    f.res_strong_actual = res.hook_any(Box::new(move |_| {
                        // strong ref to `actual_f` is held by `res`.
                        let _hold = &actual_f;
                        true
                    }));
                    let res = res.boxed();
                    f.res = res.downgrade();

                    f.status.set_from(&actual_s);
                    f.actual_weak_status = actual_s.bind(&f.status);

                    res
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut f = SwapFile::new();
                let (lang, file) = e.key();
                let actual_f = self.actual.lang_resource(lang.clone(), file.clone());
                let actual_s = self.actual.lang_resource_status(lang.clone(), file.clone());

                let res = var(actual_f.get());
                f.actual_weak_res = actual_f.bind(&res); // weak ref to `res` is held by `actual_f`
                f.res_strong_actual = res.hook_any(Box::new(move |_| {
                    // strong ref to `actual_f` is held by `res`.
                    let _hold = &actual_f;
                    true
                }));
                let res = res.boxed();
                f.res = res.downgrade();

                f.status.set_from(&actual_s);
                f.actual_weak_status = actual_s.bind(&f.status);

                e.insert(f);

                res
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<LangResourceStatus> {
        self.res
            .entry((lang, file))
            .or_insert_with(SwapFile::new)
            .status
            .read_only()
            .boxed()
    }
}
struct SwapFile {
    res: BoxedWeakVar<Option<ArcEq<fluent::FluentResource>>>,
    status: ArcVar<LangResourceStatus>,
    actual_weak_res: VarHandle,
    res_strong_actual: VarHandle,
    actual_weak_status: VarHandle,
}
impl SwapFile {
    fn new() -> Self {
        Self {
            res: WeakArcVar::default().boxed(),
            status: var(LangResourceStatus::Loading),
            actual_weak_res: VarHandle::dummy(),
            res_strong_actual: VarHandle::dummy(),
            actual_weak_status: VarHandle::dummy(),
        }
    }
}

/// Localization source that is never available.
pub struct NilL10nSource;
impl L10nSource for NilL10nSource {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        LocalVar(Arc::default()).boxed()
    }

    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        LocalVar(LangResourceStatus::NotAvailable).boxed()
    }

    fn lang_resource(&mut self, _: Lang, _: LangFilePath) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
        LocalVar(None).boxed()
    }

    fn lang_resource_status(&mut self, _: Lang, _: LangFilePath) -> BoxedVar<LangResourceStatus> {
        LocalVar(LangResourceStatus::NotAvailable).boxed()
    }
}
