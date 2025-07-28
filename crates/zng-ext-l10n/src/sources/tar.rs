use std::{borrow::Cow, collections::HashMap, fmt, io::Read as _, path::PathBuf, str::FromStr as _, sync::Arc};

use semver::Version;
use zng_clone_move::clmv;
use zng_txt::Txt;
use zng_var::{ArcEq, Var, WeakVar, var, var_local, weak_var};

use crate::{FluentParserErrors, L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

/// Represents localization resources loaded from a `.tar` or `.tar.gz` container.
///
/// The expected container layout is `root_dir/{lang}/{file}.ftl` app files and `root_dir/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`
/// for dependencies, same as [`L10nDir`], `root_dir` can have any name.
///
/// [`L10nDir`]: crate::L10nDir
pub struct L10nTar {
    data: L10nTarData,
    available_langs: Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    available_langs_status: Var<LangResourceStatus>,
    res: HashMap<(Lang, LangFilePath), L10nEntry>,
}
impl L10nTar {
    /// Load from TAR data.
    pub fn load(data: impl Into<L10nTarData>) -> Self {
        Self::load_impl(data.into())
    }
    fn load_impl(data: L10nTarData) -> Self {
        let r = Self {
            data,
            available_langs: var(Arc::new(LangMap::new())),
            available_langs_status: var(LangResourceStatus::Loading),
            res: HashMap::default(),
        };
        r.load_available_langs();
        r
    }
    fn load_available_langs(&self) {
        let status = self.available_langs_status.clone();
        let map = self.available_langs.clone();
        let data = self.data.clone();
        zng_task::spawn_wait(move || {
            let r = (|| -> std::io::Result<_> {
                let mut set: LangMap<HashMap<LangFilePath, PathBuf>> = LangMap::new();
                let mut errors: Vec<Arc<dyn std::error::Error + Send + Sync>> = vec![];
                // resource_var expects the "fatal" errors here to not insert in map
                let data = data.decode_bytes()?;
                let data: &[u8] = &data;
                let mut archive = tar::Archive::new(std::io::Cursor::new(data));
                let entries = archive.entries_with_seek()?;
                for entry in entries {
                    let entry = entry?;
                    let ty = entry.header().entry_type();
                    let entry = entry.path()?;

                    const EXT: unicase::Ascii<&'static str> = unicase::Ascii::new("ftl");

                    let is_ftl = ty.is_file()
                        && entry
                            .file_name()
                            .and_then(|s| s.to_str())
                            .and_then(|n| n.rsplit_once('.'))
                            .map(|(_, ext)| ext.is_ascii() && unicase::Ascii::new(ext) == EXT)
                            .unwrap_or(false);

                    if !is_ftl {
                        continue;
                    }

                    let utf8_path: Vec<_> = entry.iter().take(6).map(|s| s.to_str().unwrap_or("")).collect();
                    let utf8_path = &utf8_path[1..];

                    let (lang, mut file) = match utf8_path.len() {
                        // lang/file.ftl
                        2 => {
                            let lang = utf8_path[0];
                            let file = Txt::from_str(utf8_path[1].rsplit_once('.').unwrap().0);
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
                            let file = Txt::from_str(utf8_path[4]);

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

                    if file.file == "_" {
                        file.file = "".into();
                    }

                    set.get_exact_or_insert(lang, Default::default)
                        .insert(file, entry.as_ref().to_owned());
                }
                map.set(set);
                Ok(errors)
            })();
            match r {
                Ok(e) => {
                    if e.is_empty() {
                        status.set(LangResourceStatus::Loaded)
                    } else {
                        let e = LangResourceStatus::Errors(e);
                        tracing::error!("'loading available' {e}");
                        status.set(e)
                    }
                }
                Err(e) => {
                    tracing::error!("failed to load tar, {e}");
                    status.set(LangResourceStatus::Errors(vec![Arc::new(e)]))
                }
            }
        });
    }
}
impl L10nSource for L10nTar {
    fn available_langs(&mut self) -> Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.available_langs.read_only()
    }

    fn available_langs_status(&mut self) -> Var<LangResourceStatus> {
        self.available_langs_status.read_only()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> Var<Option<ArcEq<fluent::FluentResource>>> {
        match self.res.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(out) = e.get().res.upgrade() {
                    out.read_only()
                } else {
                    let (lang, file) = e.key();
                    let out = resource_var(
                        self.data.clone(),
                        &self.available_langs,
                        e.get().status.clone(),
                        lang.clone(),
                        file.clone(),
                    );
                    e.get_mut().res = out.downgrade();
                    out
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                let mut f = L10nEntry::new();
                let (lang, file) = e.key();
                let out = resource_var(
                    self.data.clone(),
                    &self.available_langs,
                    f.status.clone(),
                    lang.clone(),
                    file.clone(),
                );
                f.res = out.downgrade();
                e.insert(f);
                out
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> Var<LangResourceStatus> {
        self.res.entry((lang, file)).or_insert_with(L10nEntry::new).status.read_only()
    }
}

/// TAR data for [`L10nTar`].
#[derive(Clone, PartialEq, Eq)]
pub enum L10nTarData {
    /// Embedded data.
    Static(&'static [u8]),
    /// Loaded data.
    Arc(Arc<Vec<u8>>),
}
impl fmt::Debug for L10nTarData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(_) => f.debug_tuple("Static").finish_non_exhaustive(),
            Self::Arc(_) => f.debug_tuple("Arc").finish_non_exhaustive(),
        }
    }
}
impl From<&'static [u8]> for L10nTarData {
    fn from(value: &'static [u8]) -> Self {
        L10nTarData::Static(value)
    }
}
impl From<Arc<Vec<u8>>> for L10nTarData {
    fn from(value: Arc<Vec<u8>>) -> Self {
        L10nTarData::Arc(value)
    }
}
impl From<Vec<u8>> for L10nTarData {
    fn from(value: Vec<u8>) -> Self {
        L10nTarData::Arc(Arc::new(value))
    }
}
impl L10nTarData {
    /// Reference the data.
    pub fn bytes(&self) -> &[u8] {
        match self {
            L10nTarData::Static(b) => b,
            L10nTarData::Arc(b) => b,
        }
    }

    /// Check if the bytes have the GZIP magic number.
    pub fn is_gz(&self) -> bool {
        let bytes = self.bytes();
        bytes.len() >= 2 && bytes[0..2] == [0x1F, 0x8B]
    }

    /// Decompress bytes.
    pub fn decode_bytes(&self) -> std::io::Result<Cow<'_, [u8]>> {
        if self.is_gz() {
            let bytes = self.bytes();
            let mut data = vec![];
            let mut decoder = flate2::read::GzDecoder::new(bytes);
            decoder.read_to_end(&mut data)?;
            Ok(Cow::Owned(data))
        } else {
            Ok(Cow::Borrowed(self.bytes()))
        }
    }
}

struct L10nEntry {
    res: WeakVar<Option<ArcEq<fluent::FluentResource>>>,
    status: Var<LangResourceStatus>,
}
impl L10nEntry {
    fn new() -> Self {
        Self {
            res: weak_var(),
            status: var(LangResourceStatus::Loading),
        }
    }
}

fn resource_var(
    data: L10nTarData,
    available_langs: &Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    status: Var<LangResourceStatus>,
    lang: Lang,
    file: LangFilePath,
) -> Var<Option<ArcEq<fluent::FluentResource>>> {
    available_langs
        .map(move |w| w.get_file(&lang, &file).cloned())
        .flat_map(move |p| match p {
            Some(p) => {
                status.set(LangResourceStatus::Loading);
                let rsp = zng_task::wait_respond(clmv!(p, status, data, || {
                    const E: &str = "already decoded ok once to get entries";
                    let data = data.decode_bytes().expect(E);
                    let data: &[u8] = &data;
                    let mut archive = tar::Archive::new(std::io::Cursor::new(data));
                    for entry in archive.entries_with_seek().expect(E) {
                        let mut entry = entry.expect(E);
                        if entry.path().map(|ep| ep == p).unwrap_or(false) {
                            let mut flt = String::new();
                            if let Err(e) = entry.read_to_string(&mut flt) {
                                tracing::error!("error reading fluent resource, {e}");
                                status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                            } else {
                                match fluent::FluentResource::try_new(flt) {
                                    Ok(flt) => {
                                        // ok
                                        // Loaded set by `r` to avoid race condition in waiter.
                                        return Some(ArcEq::new(flt));
                                    }
                                    Err(e) => {
                                        let e = FluentParserErrors(e.1);
                                        tracing::error!("error parsing fluent resource, {e}");
                                        status.set(LangResourceStatus::Errors(vec![Arc::new(e)]));
                                    }
                                }
                            }
                            return None;
                        }
                    }
                    status.set(LangResourceStatus::NotAvailable);
                    None
                }));
                rsp.bind_filter_map(&status, |r| r.done().and_then(|r| r.as_ref()).map(|_| LangResourceStatus::Loaded))
                    .perm();
                rsp.map(|r| r.done().cloned().flatten())
            }
            None => var_local(None),
        })
}
