use std::{collections::HashMap, io, path::PathBuf, str::FromStr as _, sync::Arc};

use semver::Version;
use zng_clone_move::clmv;
use zng_ext_fs_watcher::WATCHER;
use zng_txt::Txt;
use zng_var::{ArcEq, Var, WeakVar, var, var_local, weak_var};

use crate::{FluentParserErrors, L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

/// Represents localization resources synchronized from files in a directory.
///
/// The expected directory layout is `{dir}/{lang}/{file}.ftl` app files and `{dir}/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`
/// for dependencies.
pub struct L10nDir {
    dir_watch: Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    dir_watch_status: Var<LangResourceStatus>,
    res: HashMap<(Lang, LangFilePath), L10nFile>,
}
impl L10nDir {
    /// Start watching the `dir` for localization files.
    pub fn open(dir: impl Into<PathBuf>) -> Self {
        Self::new(dir.into())
    }
    fn new(dir: PathBuf) -> Self {
        let (dir_watch, status) = WATCHER.read_dir_status(
            dir.clone(),
            true,
            Arc::default(),
            clmv!(|d| {
                let mut set: LangMap<HashMap<LangFilePath, PathBuf>> = LangMap::new();
                let mut errors: Vec<Arc<dyn std::error::Error + Send + Sync>> = vec![];
                let mut dir = None;
                for entry in d.min_depth(0).max_depth(5) {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            errors.push(Arc::new(e));
                            continue;
                        }
                    };
                    let ty = entry.file_type();

                    if dir.is_none() {
                        // get the watched dir (first because of min_depth(0))
                        if !ty.is_dir() {
                            tracing::error!("L10N path not a directory");
                            return Err(LangResourceStatus::NotAvailable);
                        }
                        dir = Some(entry.path().to_owned());
                        continue;
                    }

                    const EXT: unicase::Ascii<&'static str> = unicase::Ascii::new("ftl");

                    let is_ftl = ty.is_file()
                        && entry
                            .file_name()
                            .to_str()
                            .and_then(|n| n.rsplit_once('.'))
                            .map(|(_, ext)| ext.is_ascii() && unicase::Ascii::new(ext) == EXT)
                            .unwrap_or(false);

                    if !is_ftl {
                        continue;
                    }

                    let mut utf8_path = [""; 5];
                    for (i, part) in entry.path().iter().rev().take(entry.depth()).enumerate() {
                        match part.to_str() {
                            Some(p) => utf8_path[entry.depth() - i - 1] = p,
                            None => continue,
                        }
                    }

                    let (lang, file) = match entry.depth() {
                        // lang/file.ftl
                        2 => {
                            let lang = utf8_path[0];
                            let file_str = utf8_path[1].rsplit_once('.').unwrap().0;
                            let file = Txt::from_str(if file_str == "_" { "" } else { file_str });
                            (lang, LangFilePath::current_app(file))
                        }
                        // lang/deps/pkg-name/pkg-version/file.ftl
                        5 => {
                            if utf8_path[1] != "deps" {
                                continue;
                            }
                            let lang = utf8_path[0];
                            let pkg_name = Txt::from_str(utf8_path[2]);
                            let pkg_version: Version = match utf8_path[3].parse() {
                                Ok(v) => v,
                                Err(e) => {
                                    errors.push(Arc::new(e));
                                    continue;
                                }
                            };
                            let file_str = utf8_path[4].rsplit_once('.').unwrap().0;
                            let file = Txt::from_str(if file_str == "_" { "" } else { file_str });

                            (lang, LangFilePath::new(pkg_name, pkg_version, file))
                        }
                        _ => {
                            continue;
                        }
                    };

                    let lang = match Lang::from_str(lang) {
                        Ok(l) => l,
                        Err(e) => {
                            errors.push(Arc::new(e));
                            continue;
                        }
                    };

                    set.get_exact_or_insert(lang, Default::default)
                        .insert(file, entry.path().to_owned());
                }

                if errors.is_empty() {
                    // Loaded set by `dir_watch` to avoid race condition in wait.
                } else {
                    let s = LangResourceStatus::Errors(errors);
                    tracing::error!("'loading available' {s}");
                    return Err(s);
                }

                Ok(Some(Arc::new(set)))
            }),
        );

        Self {
            dir_watch,
            dir_watch_status: status.read_only(),
            res: HashMap::new(),
        }
    }
}
impl L10nSource for L10nDir {
    fn available_langs(&mut self) -> Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.dir_watch.clone()
    }
    fn available_langs_status(&mut self) -> Var<LangResourceStatus> {
        self.dir_watch_status.clone()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> Var<Option<ArcEq<fluent::FluentResource>>> {
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

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> Var<LangResourceStatus> {
        self.res.entry((lang, file)).or_insert_with(L10nFile::new).status.read_only()
    }
}
struct L10nFile {
    res: WeakVar<Option<ArcEq<fluent::FluentResource>>>,
    status: Var<LangResourceStatus>,
}
impl L10nFile {
    fn new() -> Self {
        Self {
            res: weak_var(),
            status: var(LangResourceStatus::Loading),
        }
    }
}

fn resource_var(
    dir_watch: &Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    status: Var<LangResourceStatus>,
    lang: Lang,
    file: LangFilePath,
) -> Var<Option<ArcEq<fluent::FluentResource>>> {
    dir_watch
        .map(move |w| w.get_file(&lang, &file).cloned())
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
                // set Loaded status only after `r` updates to ensure the value is available.
                r.bind_filter_map(&status, |v| v.as_ref().map(|_| LangResourceStatus::Loaded))
                    .perm();
                r
            }
            None => var_local(None),
        })
}
