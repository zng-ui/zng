use crate::{
    app_local,
    crate_util::KeyPair,
    fs_watcher::WATCHER,
    l10n::FluentParserErrors,
    text::{ToText, Txt},
    var::{types::ArcCowVar, *},
};
use fluent::FluentResource;
use std::{borrow::Cow, collections::HashMap, fmt, io, ops, path::PathBuf, str::FromStr, sync::Arc};

use super::{L10nArgument, L10nMessageBuilder, Lang, LangMap, LangResourceHandle, LangResourceStatus, Langs};

pub(super) struct L10nService {
    pub(super) available_langs: ArcVar<Arc<LangMap<HashMap<Txt, PathBuf>>>>,
    pub(super) available_langs_status: ArcVar<LangResourceStatus>,
    pub(super) sys_lang: ArcVar<Langs>,
    pub(super) app_lang: ArcCowVar<Langs, ArcVar<Langs>>,

    dir_watcher: Option<ReadOnlyArcVar<Arc<LangMap<HashMap<Txt, PathBuf>>>>>,
    file_watchers: HashMap<(Lang, Txt), LangResourceWatcher>,
    messages: HashMap<(Langs, Txt, Txt, Txt), MessageRequest>,
}
impl L10nService {
    pub fn new() -> Self {
        let sys_lang = var(Langs::default());
        Self {
            available_langs: var(Arc::new(LangMap::new())),
            available_langs_status: var(LangResourceStatus::NotAvailable),
            app_lang: sys_lang.cow(),
            sys_lang,
            dir_watcher: None,
            file_watchers: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    pub fn load_dir(&mut self, dir: PathBuf) {
        let status = self.available_langs_status.clone();
        status.set_ne(LangResourceStatus::Loading);

        let dir_watch = WATCHER.read_dir(dir, true, Arc::default(), move |d| {
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
                                                    .insert(Txt::empty(), dir.as_ref().unwrap().join(name_and_ext));
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
                                                                inner.insert(name.to_text(), f.path());
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
        });
        self.available_langs.set(dir_watch.get());
        dir_watch.bind(&self.available_langs).perm();
        self.dir_watcher = Some(dir_watch);
    }

    pub fn message(file: Txt, id: Txt, attribute: Txt, validate: bool, fallback: Txt) -> L10nMessageBuilder {
        if validate {
            Self::validate_key(&file, &id, &attribute);
        }

        L10nMessageBuilder {
            file,
            id,
            attribute,
            fallback,
            args: vec![],
        }
    }

    fn validate_key(file: &str, id: &str, attribute: &str) {
        // file
        if !file.is_empty() {
            let mut first = true;
            let mut valid = true;
            let file: &std::path::Path = file.as_ref();
            for c in file.components() {
                if !first || !matches!(c, std::path::Component::Normal(_)) {
                    valid = false;
                    break;
                }
                first = false;
            }
            if !valid {
                panic!("invalid resource file name, must be a single file name");
            }
        }

        // https://github.com/projectfluent/fluent/blob/master/spec/fluent.ebnf
        // Identifier ::= [a-zA-Z] [a-zA-Z0-9_-]*
        fn validate(name: &str, value: &str) {
            let mut valid = true;
            let mut first = true;
            if !value.is_empty() {
                for c in value.chars() {
                    if !first && (c == '_' || c == '-' || c.is_ascii_digit()) {
                        continue;
                    }
                    if !c.is_ascii_lowercase() && !c.is_ascii_uppercase() {
                        valid = false;
                        break;
                    }

                    first = false;
                }
            } else {
                valid = false;
            }
            if !valid {
                panic!("invalid resource {name}, must start with letter, followed by any letters, digits, `_` or `-`");
            }
        }
        validate("id", id);
        if !attribute.is_empty() {
            validate("attribute", attribute)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn message_text(
        &mut self,
        lang: Langs,
        file: Txt,
        id: Txt,
        attribute: Txt,
        validate: bool,
        fallback: Txt,
        args: Vec<(Txt, BoxedVar<L10nArgument>)>,
    ) -> ReadOnlyArcVar<Txt> {
        if validate {
            Self::validate_key(&file, &id, &attribute);
        }

        match self.messages.entry((lang, file, id, attribute)) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if let Some(txt) = e.get().text.upgrade() {
                    // already requested
                    txt.read_only()
                } else {
                    // already requested and dropped, reload.
                    let (langs, file, id, attr) = e.key();
                    let handles = langs
                        .0
                        .iter()
                        .map(|l| Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, l.clone(), file.clone()))
                        .collect();

                    let (r, txt) = MessageRequest::new(fallback, args, handles, langs, file, id, attr, &self.file_watchers);
                    *e.get_mut() = r;
                    txt
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                // not request, load.
                let (langs, file, id, attr) = e.key();
                let handles = langs
                    .0
                    .iter()
                    .map(|l| Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, l.clone(), file.clone()))
                    .collect();
                let (r, txt) = MessageRequest::new(fallback, args, handles, langs, file, id, attr, &self.file_watchers);
                e.insert(r);
                txt
            }
        }
    }

    pub fn lang_resource(&mut self, lang: Lang, file: Txt, validate: bool) -> LangResourceHandle {
        if validate {
            Self::validate_key(&file, "i", "")
        }
        Self::lang_resource_impl(&mut self.file_watchers, &self.available_langs, lang, file)
    }
    fn lang_resource_impl(
        file_watchers: &mut HashMap<(Lang, Txt), LangResourceWatcher>,
        available_langs: &ArcVar<Arc<LangMap<HashMap<Txt, PathBuf>>>>,
        lang: Lang,
        file: Txt,
    ) -> LangResourceHandle {
        match file_watchers.entry((lang, file)) {
            std::collections::hash_map::Entry::Occupied(e) => e.get().handle(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let (lang, file) = e.key();
                let (w, h) = if let Some(files) = available_langs.get().get_exact(lang) {
                    if let Some(file) = files.get(file) {
                        LangResourceWatcher::new(lang.clone(), file.clone())
                    } else {
                        LangResourceWatcher::new_not_available(lang.clone())
                    }
                } else {
                    LangResourceWatcher::new_not_available(lang.clone())
                };
                e.insert(w);
                h
            }
        }
    }

    pub fn update(&mut self) {
        if let Some(watcher) = &self.dir_watcher {
            if let Some(available_langs) = watcher.get_new() {
                // renew watchers, keeps the same handlers
                for ((lang, file), watcher) in self.file_watchers.iter_mut() {
                    let file = available_langs.get_exact(lang).and_then(|f| f.get(file));
                    if watcher.file.as_ref() == file {
                        continue;
                    }

                    let handle = watcher.handle.take().unwrap();
                    *watcher = if let Some(file) = file {
                        LangResourceWatcher::new_with_handle(lang.clone(), file.clone(), handle)
                    } else {
                        LangResourceWatcher::new_not_available_with_handle(lang.clone(), handle)
                    };
                }
            }
        } else {
            // no dir loaded
            return;
        }

        self.messages
            .retain(|(langs, file, id, attr), request| request.update(langs, file, id, attr, &self.file_watchers));

        self.file_watchers.retain(|_lang, watcher| watcher.retain());
    }
}
app_local! {
    pub(super) static L10N_SV: L10nService = L10nService::new();
}

struct LangResourceWatcher {
    handle: Option<crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>>,
    bundle: ReadOnlyArcVar<ArcFluentBundle>,
    file: Option<PathBuf>,
}
impl LangResourceWatcher {
    fn new(lang: Lang, file: PathBuf) -> (Self, LangResourceHandle) {
        let status = var(LangResourceStatus::Loading);
        let (owner, handle) = crate::crate_util::Handle::new(status);
        let me = Self::new_with_handle(lang, file, owner);
        (me, LangResourceHandle(handle))
    }

    fn new_not_available(lang: Lang) -> (Self, LangResourceHandle) {
        let status = var(LangResourceStatus::NotAvailable);
        let (owner, handle) = crate::crate_util::Handle::new(status);
        let me = Self::new_not_available_with_handle(lang, owner);
        (me, LangResourceHandle(handle))
    }

    fn new_with_handle(lang: Lang, file: PathBuf, handle: crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>) -> Self {
        let init = ConcurrentFluentBundle::new_concurrent(vec![lang.clone()]);
        let status = handle.data();
        status.set_ne(LangResourceStatus::Loading);
        let bundle = WATCHER.read(
            file.clone(),
            ArcFluentBundle::new(init),
            clmv!(status, |file| {
                status.set_ne(LangResourceStatus::Loading);

                match file.and_then(|mut f| f.string()) {
                    Ok(flt) => match FluentResource::try_new(flt) {
                        Ok(flt) => {
                            let mut bundle = ConcurrentFluentBundle::new_concurrent(vec![lang.clone()]);
                            bundle.add_resource_overriding(flt);
                            status.set_ne(LangResourceStatus::Loaded);
                            // ok
                            return Some(ArcFluentBundle::new(bundle));
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
                None
            }),
        );
        Self {
            handle: Some(handle),
            bundle,
            file: Some(file),
        }
    }

    fn new_not_available_with_handle(lang: Lang, handle: crate::crate_util::HandleOwner<ArcVar<LangResourceStatus>>) -> Self {
        handle.data().set_ne(LangResourceStatus::NotAvailable);
        Self {
            handle: Some(handle),
            bundle: var({
                let init = ConcurrentFluentBundle::new_concurrent(vec![lang]);
                ArcFluentBundle::new(init)
            })
            .read_only(),
            file: None,
        }
    }

    fn handle(&self) -> LangResourceHandle {
        let handle = self.handle.as_ref().unwrap().reanimate();
        LangResourceHandle(handle)
    }

    fn retain(&self) -> bool {
        !self.handle.as_ref().unwrap().is_dropped()
    }
}

type ConcurrentFluentBundle = fluent::bundle::FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>;

#[derive(Clone)]
struct ArcFluentBundle(Arc<ConcurrentFluentBundle>);
impl fmt::Debug for ArcFluentBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArcFluentBundle")
    }
}
impl ops::Deref for ArcFluentBundle {
    type Target = ConcurrentFluentBundle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ArcFluentBundle {
    pub fn new(bundle: ConcurrentFluentBundle) -> Self {
        Self(Arc::new(bundle))
    }
}

struct MessageRequest {
    text: crate::var::types::WeakArcVar<Txt>,
    fallback: Txt,
    args: Vec<(Txt, BoxedVar<L10nArgument>)>,

    resource_handles: Box<[LangResourceHandle]>,
    current_resource: usize,
}
impl MessageRequest {
    #[allow(clippy::too_many_arguments)]
    fn new(
        fallback: Txt,
        args: Vec<(Txt, BoxedVar<L10nArgument>)>,
        resource_handles: Box<[LangResourceHandle]>,

        langs: &Langs,
        file: &Txt,
        id: &Txt,
        attribute: &Txt,
        resources: &HashMap<(Lang, Txt), LangResourceWatcher>,
    ) -> (Self, ReadOnlyArcVar<Txt>) {
        let mut text = None;
        let mut current_resource = resource_handles.len();

        for (i, h) in resource_handles.iter().enumerate() {
            if matches!(h.status().get(), LangResourceStatus::Loaded) {
                let bundle = &resources.get(&(&langs[i], file) as &dyn KeyPair<_, _>).unwrap().bundle;
                if bundle.with(|b| has_message(b, id, attribute)) {
                    // found something already loaded

                    let t = bundle.with(|b| format_message(b, id, attribute, &args));
                    text = Some(var(t));
                    current_resource = i;
                    break;
                }
            }
        }

        let text = text.unwrap_or_else(|| {
            // no available resource yet
            var(format_fallback(file, id, attribute, &fallback, &args))
        });

        let r = Self {
            text: text.downgrade(),
            fallback,
            args,
            resource_handles,
            current_resource,
        };

        (r, text.read_only())
    }

    fn update(
        &mut self,
        langs: &Langs,
        file: &Txt,
        id: &Txt,
        attribute: &Txt,
        resources: &HashMap<(Lang, Txt), LangResourceWatcher>,
    ) -> bool {
        if let Some(txt) = self.text.upgrade() {
            for (i, h) in self.resource_handles.iter().enumerate() {
                if matches!(h.status().get(), LangResourceStatus::Loaded) {
                    let bundle = &resources.get(&(&langs[i], file) as &dyn KeyPair<_, _>).unwrap().bundle;
                    if bundle.with(|b| has_message(b, id, attribute)) {
                        //  found best
                        if self.current_resource != i || bundle.is_new() || self.args.iter().any(|a| a.1.is_new()) {
                            self.current_resource = i;

                            let t = bundle.with(|b| format_message(b, id, attribute, &self.args));
                            txt.set_ne(t)
                        }
                        return true;
                    }
                }
            }

            // fallback
            if self.current_resource != self.resource_handles.len() || self.args.iter().any(|a| a.1.is_new()) {
                self.current_resource = self.resource_handles.len();

                txt.set_ne(format_fallback(file, id, attribute, &self.fallback, &self.args));
            }

            true
        } else {
            false
        }
    }
}

fn format_fallback(file: &str, id: &str, attribute: &str, fallback: &Txt, args: &[(Txt, BoxedVar<L10nArgument>)]) -> Txt {
    let mut fallback_pattern = None;

    let entry = format!("k={fallback}");
    match fluent_syntax::parser::parse_runtime(entry.as_str()) {
        Ok(mut f) => {
            if let Some(fluent_syntax::ast::Entry::Message(m)) = f.body.pop() {
                if let Some(p) = m.value {
                    fallback_pattern = Some(p)
                }
            }
        }
        Err(e) => {
            let key = DisplayKey { file, id, attribute };
            tracing::error!("invalid fallback for `{key}`\n{}", FluentParserErrors(e.1));
        }
    }
    let fallback = match fallback_pattern {
        Some(f) => f,
        None => fluent_syntax::ast::Pattern {
            elements: vec![fluent_syntax::ast::PatternElement::TextElement { value: fallback.as_str() }],
        },
    };

    let values: Vec<_> = args.iter().map(|(_, v)| v.get()).collect();
    let args = if args.is_empty() {
        None
    } else {
        let mut r = fluent::FluentArgs::with_capacity(args.len());
        for ((key, _), value) in args.iter().zip(&values) {
            r.set(Cow::Borrowed(key.as_str()), value.fluent_value())
        }
        Some(r)
    };

    let mut errors = vec![];
    let blank = fluent::FluentBundle::<fluent::FluentResource>::new(vec![]);
    let txt = blank.format_pattern(&fallback, args.as_ref(), &mut errors);

    if !errors.is_empty() {
        let key = DisplayKey { file, id, attribute };
        tracing::error!("error formatting fallback `{key}`\n{}", FluentErrors(errors));
    }

    txt.to_text()
}

fn format_message(bundle: &ArcFluentBundle, id: &str, attribute: &str, args: &[(Txt, BoxedVar<L10nArgument>)]) -> Txt {
    let msg = bundle.get_message(id).unwrap();

    let values: Vec<_> = args.iter().map(|(_, v)| v.get()).collect();
    let args = if args.is_empty() {
        None
    } else {
        let mut r = fluent::FluentArgs::with_capacity(args.len());
        for ((key, _), value) in args.iter().zip(&values) {
            r.set(Cow::Borrowed(key.as_str()), value.fluent_value())
        }
        Some(r)
    };

    if attribute.is_empty() {
        if let Some(pattern) = msg.value() {
            let mut errors = vec![];
            let txt = bundle.format_pattern(pattern, args.as_ref(), &mut errors);

            if !errors.is_empty() {
                tracing::error!("error formatting `{}/{}`\n{}", &bundle.locales[0], id, FluentErrors(errors));
            }

            txt.to_text()
        } else {
            tracing::error!("found `{:?}/{id}`, but not value", &bundle.locales[0]);
            Txt::empty()
        }
    } else {
        match msg.get_attribute(attribute) {
            Some(attr) => {
                let mut errors = vec![];

                let txt = bundle.format_pattern(attr.value(), args.as_ref(), &mut errors);

                if !errors.is_empty() {
                    tracing::error!("error formatting `{}/{}`\n{}", &bundle.locales[0], id, FluentErrors(errors));
                }

                txt.to_text()
            }
            None => {
                tracing::error!("found `{:?}/{id}`, but not attribute `{attribute}`", &bundle.locales[0]);
                Txt::empty()
            }
        }
    }
}

struct FluentErrors(Vec<fluent::FluentError>);

impl fmt::Display for FluentErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for e in &self.0 {
            write!(f, "{sep}{e}")?;
            sep = "\n";
        }
        Ok(())
    }
}

fn has_message(bundle: &ArcFluentBundle, id: &str, attribute: &str) -> bool {
    if attribute.is_empty() {
        bundle.has_message(id)
    } else if let Some(msg) = bundle.get_message(id) {
        msg.get_attribute(attribute).is_some()
    } else {
        false
    }
}

struct DisplayKey<'a> {
    file: &'a str,
    id: &'a str,
    attribute: &'a str,
}
impl<'a> fmt::Display for DisplayKey<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.file.is_empty() {
            write!(f, "{}/", self.file)?
        }
        write!(f, "{}", self.id)?;
        if !self.attribute.is_empty() {
            write!(f, ".{}", self.attribute)?;
        }
        Ok(())
    }
}
