use std::{borrow::Cow, collections::HashMap, fmt, io::Read as _, path::PathBuf, sync::Arc};

use zng_var::{var, ArcEq, ArcVar, BoxedVar, Var as _};

use crate::{L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

/// Represents localization resources loaded from a TAR container.
///
/// The expected container layout is `{dir}/{lang}/{file}.ftl` app files and `{dir}/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`
/// for dependencies, same as [`L10nDir`].
///
/// [`L10nDir`]: crate::L10nDir
pub struct L10nTar {
    data: L10nTarData,
    available_langs: ArcVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    available_langs_status: ArcVar<LangResourceStatus>,
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
        };
        r.load_available_langs();
        r
    }
    fn load_available_langs(&self) {
        let status = self.available_langs_status.clone();
        let map = self.available_langs.clone();
        let data = self.data.clone();
        zng_task::spawn_wait(move || {
            let r = (|| -> std::io::Result<()> {
                let data = data.decode_bytes()?;
                let data: &[u8] = &data;
                let mut archive = tar::Archive::new(data);
                let entries = archive.entries()?;
                for entry in entries {
                    let entry = entry?;
                    let entry = entry.path()?;
                }
                Ok(())
            })();
            match r {
                Ok(()) => status.set(LangResourceStatus::Loaded),
                Err(e) =>  status.set(LangResourceStatus::Errors(vec![Arc::new(e)])),
            }          
        });
    }
}
impl L10nSource for L10nTar {
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.available_langs.read_only().boxed()
    }

    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus> {
        self.available_langs_status.read_only().boxed()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>> {
        todo!()
    }

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<LangResourceStatus> {
        todo!()
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
    pub fn is_gzip(&self) -> bool {
        let bytes = self.bytes();
        bytes.len() >= 2 && bytes[0..2] == [0x1F, 0x8B]
    }

    /// Decompress bytes.
    pub fn decode_bytes(&self) -> std::io::Result<Cow<[u8]>> {
        if self.is_gzip() {
            let bytes = self.bytes();
            let mut data = vec![];
            let mut decoder = flate2::read::GzDecoder::new(bytes);
            decoder.read_to_end(&mut data)?;
            Ok(Cow::Owned(data))
        } else {
           Ok( Cow::Borrowed(self.bytes()))
        }
    }
}
