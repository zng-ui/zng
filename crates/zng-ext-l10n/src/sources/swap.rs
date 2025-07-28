use std::{collections::HashMap, path::PathBuf, sync::Arc};

use zng_var::{ArcEq, Var, VarHandle, WeakVar, var, weak_var};

use crate::{L10nSource, Lang, LangFilePath, LangMap, LangResourceStatus};

use super::NilL10nSource;

/// Represents localization source that can swap the actual source without disconnecting variables
/// taken on resources.
///
/// Note that [`L10N.load`] already uses this source internally.
///
/// [`L10N.load`]: crate::L10N::load
pub struct SwapL10nSource {
    actual: Box<dyn L10nSource>,

    available_langs: Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>,
    available_langs_status: Var<LangResourceStatus>,

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
                f.res_strong_actual = res.hold(actual_f); // strong ref to `actual_f` is held by `res`.

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
    fn available_langs(&mut self) -> Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.available_langs.read_only()
    }

    fn available_langs_status(&mut self) -> Var<LangResourceStatus> {
        self.available_langs_status.read_only()
    }

    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> Var<Option<ArcEq<fluent::FluentResource>>> {
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
                    f.res_strong_actual = res.hold(actual_f); // strong ref to `actual_f` is held by `res`.
                    let res = res;
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
                f.res_strong_actual = res.hold(actual_f); // strong ref to `actual_f` is held by `res`.
                let res = res;
                f.res = res.downgrade();

                f.status.set_from(&actual_s);
                f.actual_weak_status = actual_s.bind(&f.status);

                e.insert(f);

                res
            }
        }
    }

    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> Var<LangResourceStatus> {
        self.res.entry((lang, file)).or_insert_with(SwapFile::new).status.read_only()
    }
}
struct SwapFile {
    res: WeakVar<Option<ArcEq<fluent::FluentResource>>>,
    status: Var<LangResourceStatus>,
    actual_weak_res: VarHandle,
    res_strong_actual: VarHandle,
    actual_weak_status: VarHandle,
}
impl SwapFile {
    fn new() -> Self {
        Self {
            res: weak_var(),
            status: var(LangResourceStatus::Loading),
            actual_weak_res: VarHandle::dummy(),
            res_strong_actual: VarHandle::dummy(),
            actual_weak_status: VarHandle::dummy(),
        }
    }
}
