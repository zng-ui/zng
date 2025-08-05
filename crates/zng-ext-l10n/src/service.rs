use std::{
    borrow::Cow,
    collections::{HashMap, hash_map},
    fmt, ops,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use parking_lot::Mutex;
use zng_app_context::app_local;
use zng_txt::Txt;
use zng_var::{ArcEq, MergeVarBuilder, Var, WeakVar, const_var, merge_var, var};
use zng_view_api::config::LocaleConfig;

use crate::{
    FluentParserErrors, L10nArgument, L10nSource, Lang, LangFilePath, LangMap, LangResource, LangResourceStatus, Langs, SwapL10nSource,
};

pub(super) struct L10nService {
    source: Mutex<SwapL10nSource>, // Mutex for `Sync` only.
    sys_lang: Var<Langs>,
    app_lang: Var<Langs>,

    perm_res: Vec<Var<Option<ArcEq<fluent::FluentResource>>>>,
    bundles: HashMap<(Langs, LangFilePath), WeakVar<ArcFluentBundle>>,
}
impl L10nService {
    pub fn new() -> Self {
        let sys_lang = var(Langs::default());
        Self {
            source: Mutex::new(SwapL10nSource::new()),
            app_lang: sys_lang.cow(),
            sys_lang,
            perm_res: vec![],
            bundles: HashMap::new(),
        }
    }

    pub fn load(&mut self, source: impl L10nSource) {
        self.source.get_mut().load(source);
    }

    pub fn available_langs(&mut self) -> Var<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        self.source.get_mut().available_langs()
    }

    pub fn available_langs_status(&mut self) -> Var<LangResourceStatus> {
        self.source.get_mut().available_langs_status()
    }

    pub fn sys_lang(&self) -> Var<Langs> {
        self.sys_lang.read_only()
    }

    pub fn app_lang(&self) -> Var<Langs> {
        self.app_lang.clone()
    }

    pub fn localized_message(
        &mut self,
        langs: Langs,
        file: LangFilePath,
        id: Txt,
        attribute: Txt,
        fallback: Txt,
        mut args: Vec<(Txt, Var<L10nArgument>)>,
    ) -> Var<Txt> {
        if langs.is_empty() {
            return if args.is_empty() {
                // no lang, no args
                const_var(fallback)
            } else {
                // no lang, but args can change
                fluent_args_var(args).map(move |args| {
                    let args = args.lock();
                    format_fallback(&file.file, id.as_str(), attribute.as_str(), &fallback, Some(&*args))
                })
            };
        }

        let bundle = self.resource_bundle(langs, file.clone());

        if args.is_empty() {
            // no args, but message can change
            bundle.map(move |b| {
                if let Some(msg) = b.get_message(&id) {
                    let value = if attribute.is_empty() {
                        msg.value()
                    } else {
                        msg.get_attribute(&attribute).map(|attr| attr.value())
                    };
                    if let Some(pattern) = value {
                        let mut errors = vec![];
                        let r = b.format_pattern(pattern, None, &mut errors);
                        if !errors.is_empty() {
                            let e = FluentErrors(errors);
                            if attribute.is_empty() {
                                tracing::error!("error formatting {id}\n{e}");
                            } else {
                                tracing::error!("error formatting {id}.{attribute}\n{e}");
                            }
                        }
                        return Txt::from_str(r.as_ref());
                    }
                }
                fallback.clone()
            })
        } else if args.len() == 1 {
            // one arg and message can change
            let (name, arg) = args.remove(0);

            merge_var!(bundle, arg, move |b, arg| {
                let mut args = fluent::FluentArgs::with_capacity(1);
                args.set(Cow::Borrowed(name.as_str()), arg.fluent_value());

                if let Some(msg) = b.get_message(&id) {
                    let value = if attribute.is_empty() {
                        msg.value()
                    } else {
                        msg.get_attribute(&attribute).map(|attr| attr.value())
                    };
                    if let Some(pattern) = value {
                        let mut errors = vec![];

                        let r = b.format_pattern(pattern, Some(&args), &mut errors);
                        if !errors.is_empty() {
                            let e = FluentErrors(errors);
                            let key = DisplayKey {
                                file: &file.file,
                                id: id.as_str(),
                                attribute: attribute.as_str(),
                            };
                            tracing::error!("error formatting {key}\n{e}");
                        }
                        return Txt::from_str(r.as_ref());
                    }
                }

                format_fallback(&file.file, id.as_str(), attribute.as_str(), &fallback, Some(&args))
            })
        } else {
            // many args and message can change
            merge_var!(bundle, fluent_args_var(args), move |b, args| {
                if let Some(msg) = b.get_message(&id) {
                    let value = if attribute.is_empty() {
                        msg.value()
                    } else {
                        msg.get_attribute(&attribute).map(|attr| attr.value())
                    };
                    if let Some(pattern) = value {
                        let mut errors = vec![];

                        let args = args.lock();
                        let r = b.format_pattern(pattern, Some(&*args), &mut errors);
                        if !errors.is_empty() {
                            let e = FluentErrors(errors);
                            let key = DisplayKey {
                                file: &file.file,
                                id: id.as_str(),
                                attribute: attribute.as_str(),
                            };
                            tracing::error!("error formatting {key}\n{e}");
                        }
                        return Txt::from_str(r.as_ref());
                    }
                }

                let args = args.lock();
                format_fallback(&file.file, id.as_str(), attribute.as_str(), &fallback, Some(&*args))
            })
        }
    }

    fn resource_bundle(&mut self, langs: Langs, file: LangFilePath) -> Var<ArcFluentBundle> {
        match self.bundles.entry((langs, file)) {
            hash_map::Entry::Occupied(mut e) => {
                if let Some(r) = e.get().upgrade() {
                    return r;
                }
                let (langs, file) = e.key();
                let r = Self::new_resource_bundle(self.source.get_mut(), langs, file);
                e.insert(r.downgrade());
                r
            }
            hash_map::Entry::Vacant(e) => {
                let (langs, file) = e.key();
                let r = Self::new_resource_bundle(self.source.get_mut(), langs, file);
                e.insert(r.downgrade());
                r
            }
        }
    }
    fn new_resource_bundle(source: &mut SwapL10nSource, langs: &Langs, file: &LangFilePath) -> Var<ArcFluentBundle> {
        if langs.len() == 1 {
            let lang = langs[0].clone();
            let res = source.lang_resource(lang.clone(), file.clone());
            res.map(move |r| {
                let mut bundle = ConcurrentFluentBundle::new_concurrent(vec![lang.0.clone()]);
                if let Some(r) = r {
                    bundle.add_resource_overriding(r.0.clone());
                }
                ArcFluentBundle(Arc::new(bundle))
            })
        } else {
            debug_assert!(langs.len() > 1);

            let langs = langs.0.clone();

            let mut res = MergeVarBuilder::new();
            for l in langs.iter().rev() {
                res.push(source.lang_resource(l.clone(), file.clone()));
            }
            res.build(move |res| {
                let mut bundle = ConcurrentFluentBundle::new_concurrent(langs.iter().map(|l| l.0.clone()).collect());
                for r in res.iter().flatten() {
                    bundle.add_resource_overriding(r.0.clone());
                }
                ArcFluentBundle(Arc::new(bundle))
            })
        }
    }

    pub fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> LangResource {
        LangResource {
            res: self.source.get_mut().lang_resource(lang.clone(), file.clone()),
            status: self.source.get_mut().lang_resource_status(lang, file),
        }
    }

    pub fn set_sys_langs(&self, cfg: &LocaleConfig) {
        let langs = cfg
            .langs
            .iter()
            .filter_map(|l| match Lang::from_str(l) {
                Ok(l) => Some(l),
                Err(e) => {
                    tracing::error!("invalid lang {l:?}, {e}");
                    None
                }
            })
            .collect();
        self.sys_lang.set(Langs(langs));
    }

    pub fn push_perm_resource(&mut self, r: LangResource) {
        if !self.perm_res.iter().any(|rr| rr.var_eq(&r.res)) {
            self.perm_res.push(r.res);
        }
    }
}
app_local! {
    pub(super) static L10N_SV: L10nService = L10nService::new();
}

type ConcurrentFluentBundle = fluent::bundle::FluentBundle<Arc<fluent::FluentResource>, intl_memoizer::concurrent::IntlLangMemoizer>;

#[derive(Clone)]
struct ArcFluentBundle(Arc<ConcurrentFluentBundle>);
impl fmt::Debug for ArcFluentBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArcFluentBundle")
    }
}
impl PartialEq for ArcFluentBundle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl ops::Deref for ArcFluentBundle {
    type Target = ConcurrentFluentBundle;

    fn deref(&self) -> &Self::Target {
        &self.0
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

fn format_fallback(file: &str, id: &str, attribute: &str, fallback: &Txt, args: Option<&fluent::FluentArgs>) -> Txt {
    let mut fallback_pattern = None;

    let mut entry = "k = ".to_owned();
    let mut prefix = "";
    for line in fallback.lines() {
        entry.push_str(prefix);
        entry.push_str(line);
        prefix = "\n   ";
    }
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

    let mut errors = vec![];
    let blank = fluent::FluentBundle::<fluent::FluentResource>::new(vec![]);
    let txt = blank.format_pattern(&fallback, args, &mut errors);

    if !errors.is_empty() {
        let key = DisplayKey { file, id, attribute };
        tracing::error!("error formatting fallback `{key}`\n{}", FluentErrors(errors));
    }

    Txt::from_str(txt.as_ref())
}

fn fluent_args_var(args: Vec<(Txt, Var<L10nArgument>)>) -> Var<ArcEq<Mutex<fluent::FluentArgs<'static>>>> {
    let mut fluent_args = MergeVarBuilder::new();
    let mut names = Vec::with_capacity(args.len());
    for (name, arg) in args {
        names.push(name);
        fluent_args.push(arg);
    }
    fluent_args.build(move |values| {
        // review after https://github.com/projectfluent/fluent-rs/issues/319
        let mut args = fluent::FluentArgs::with_capacity(values.len());
        for (name, value) in names.iter().zip(values.iter()) {
            args.set(Cow::Owned(name.to_string()), value.to_fluent_value());
        }

        // Mutex because ValueType is not Sync
        ArcEq::new(Mutex::new(args))
    })
}

struct DisplayKey<'a> {
    file: &'a str,
    id: &'a str,
    attribute: &'a str,
}
impl fmt::Display for DisplayKey<'_> {
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
