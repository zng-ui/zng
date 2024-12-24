use std::{collections::HashMap, path::PathBuf, sync::Arc};

use zng_var::{ArcEq, BoxedVar, LocalVar, Var as _};

use crate::{L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

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
