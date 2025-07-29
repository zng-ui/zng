use std::{collections::HashMap, path::PathBuf, sync::Arc};

use zng_var::{ArcEq, Var, const_var};

use crate::{L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

/// Localization source that is never available.
pub struct NilL10nSource;
impl L10nSource for NilL10nSource {
    fn available_langs(&mut self) -> Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        const_var(Arc::default())
    }

    fn available_langs_status(&mut self) -> Var<LangResourceStatus> {
        const_var(LangResourceStatus::NotAvailable)
    }

    fn lang_resource(&mut self, _: Lang, _: LangFilePath) -> Var<Option<ArcEq<fluent::FluentResource>>> {
        const_var(None)
    }

    fn lang_resource_status(&mut self, _: Lang, _: LangFilePath) -> Var<LangResourceStatus> {
        const_var(LangResourceStatus::NotAvailable)
    }
}
