use once_cell::sync::OnceCell;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use std::{borrow::Cow, env, fs, path::PathBuf};
use syn::{Attribute, Ident};

/// Returns `true` if `a` and `b` have the same tokens in the same order (ignoring span).
pub fn token_stream_eq(a: TokenStream, b: TokenStream) -> bool {
    let mut a = a.into_iter();
    let mut b = b.into_iter();
    use TokenTree::*;
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => match (a, b) {
                (Group(a), Group(b)) if a.delimiter() == b.delimiter() && token_stream_eq(a.stream(), b.stream()) => continue,
                (Ident(a), Ident(b)) if a == b => continue,
                (Punct(a), Punct(b)) if a.as_char() == b.as_char() && a.spacing() == b.spacing() => continue,
                (Literal(a), Literal(b)) if a.to_string() == b.to_string() => continue,
                _ => return false,
            },
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`TokenStream`].
pub fn tokens_to_ident_str(tokens: &TokenStream) -> String {
    let tokens = tokens.to_string();
    let max = tokens.len().min(40);
    let mut tokens = tokens[(tokens.len() - max)..]
        .replace(&['.', ':', ' '][..], "_")
        .replace('!', "not")
        .replace("&&", "and")
        .replace("||", "or")
        .replace('(', "p")
        .replace(')', "b")
        .replace("==", "eq");

    tokens.retain(|c| c == '_' || c.is_alphanumeric());

    tokens
}

/// Collection of compile errors.
#[derive(Default)]
pub struct Errors {
    tokens: TokenStream,
}
impl Errors {
    /// Push a compile error.
    pub fn push(&mut self, error: impl ToString, span: Span) {
        let error = error.to_string();
        self.tokens.extend(quote_spanned! {span=>
            compile_error!{#error}
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
impl ToTokens for Errors {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.tokens.clone())
    }
    fn to_token_stream(&self) -> TokenStream {
        self.tokens.clone()
    }
    fn into_token_stream(self) -> TokenStream {
        self.tokens
    }
}

/// Generates a return of a compile_error message in the given span.
macro_rules! abort {
    ($span:expr, $($tt:tt)*) => {{
        let error = format!($($tt)*);
        let error = syn::LitStr::new(&error, proc_macro2::Span::call_site());

        return quote_spanned!($span=> compile_error!{#error}).into();
    }};
}

/// Generates a return of a compile_error message in the call_site span.
macro_rules! abort_call_site {
    ($($tt:tt)*) => {
        abort!(proc_macro2::Span::call_site(), $($tt)*)
    };
}

/// Input error not caused by the user.
macro_rules! non_user_error {
    ($e:expr) => {
        panic!("[{}:{}] invalid non-user input: {}", file!(), line!(), $e)
    };
    ($fmt:tt, $($args:tt)+) => {
        non_user_error! {
            format_args!($fmt, $($args)+)
        }
    }
}

/// `Ident` with custom span.
macro_rules! ident_spanned {
    ($span:expr=> $($format_name:tt)+) => {
        proc_macro2::Ident::new(&format!($($format_name)+), $span)
    };
}

/// `Ident` with call_site span.
macro_rules! ident {
    ($($tt:tt)*) => {
        ident_spanned!(proc_macro2::Span::call_site()=> $($tt)*)
    };
}

/// Separated attributes.
#[derive(Clone)]
pub struct Attributes {
    pub docs: Vec<Attribute>,
    pub cfg: Option<Attribute>,
    pub deprecated: Option<Attribute>,
    pub lints: Vec<Attribute>,
    pub others: Vec<Attribute>,
}
impl Attributes {
    pub fn new(attrs: Vec<Attribute>) -> Self {
        let mut docs = vec![];
        let mut cfg = None;
        let mut deprecated = None;
        let mut lints = vec![];
        let mut others = vec![];

        for attr in attrs {
            if let Some(ident) = attr.path().get_ident() {
                if ident == "doc" {
                    docs.push(attr);
                    continue;
                } else if ident == "cfg" {
                    cfg = Some(attr);
                } else if ident == "deprecated" {
                    deprecated = Some(attr);
                } else if ident == "allow" || ident == "expect" || ident == "warn" || ident == "deny" || ident == "forbid" {
                    lints.push(attr);
                } else {
                    others.push(attr);
                }
            } else {
                others.push(attr);
            }
        }

        Attributes {
            docs,
            cfg,
            deprecated,
            lints,
            others,
        }
    }
}
impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in self
            .docs
            .iter()
            .chain(&self.cfg)
            .chain(&self.deprecated)
            .chain(&self.lints)
            .chain(&self.others)
        {
            attr.to_tokens(tokens);
        }
    }
}

/// Return the equivalent of `$crate`.
pub fn crate_core() -> TokenStream {
    let (ident, module) = if is_rust_analyzer() {
        // rust-analyzer gets the wrong crate sometimes if we cache, maybe they use the same server instance
        // for the entire workspace?
        let (ident, module) = crate_core_parts();
        (Cow::Owned(ident), module)
    } else {
        static CRATE: OnceCell<(String, &'static str)> = OnceCell::new();

        let (ident, module) = CRATE.get_or_init(crate_core_parts);
        (Cow::Borrowed(ident.as_str()), *module)
    };

    let ident = Ident::new(&ident, Span::call_site());
    if !module.is_empty() {
        let module = Ident::new(module, Span::call_site());
        quote! { #ident::#module }
    } else {
        ident.to_token_stream()
    }
}
fn crate_core_parts() -> (String, &'static str) {
    if let Ok(ident) = crate_name("zng") {
        // using the main crate.
        match ident {
            FoundCrate::Name(name) => (name, "__proc_macro_util"),
            FoundCrate::Itself => ("zng".to_owned(), "__proc_macro_util"),
        }
    } else if let Ok(ident) = crate_name("zng-wgt") {
        // using the wgt crate.
        match ident {
            FoundCrate::Name(name) => (name, "__proc_macro_util"),
            FoundCrate::Itself => ("zng_wgt".to_owned(), "__proc_macro_util"),
        }
    } else if let Ok(ident) = crate_name("zng-var") {
        // using the core crate only.
        match ident {
            FoundCrate::Name(name) => (name, ""),
            FoundCrate::Itself => ("zng_var".to_owned(), ""),
        }
    } else {
        // failed, at least shows "zng" in the compile error.
        ("zng".to_owned(), "__proc_macro_util")
    }
}

#[derive(PartialEq, Debug)]
enum FoundCrate {
    Name(String),
    Itself,
}

/// Gets the module name of a given crate name (same behavior as $crate).
fn crate_name(orig_name: &str) -> Result<FoundCrate, ()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|_| ())?);

    let toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).map_err(|_| ())?;

    crate_name_impl(orig_name, &toml)
}
fn crate_name_impl(orig_name: &str, toml: &str) -> Result<FoundCrate, ()> {
    // some of this code is based on the crate `proc-macro-crate` code, we
    // don't depend on that crate to speedup compile time.
    enum State<'a> {
        Seeking,
        Package,
        Dependencies,
        Dependency(&'a str),
    }

    let mut state = State::Seeking;

    for line in toml.lines() {
        let line = line.trim();

        let new_state = if line == "[package]" {
            Some(State::Package)
        } else if line.contains("dependencies.") && line.ends_with(']') {
            let name_start = line.rfind('.').unwrap();
            let name = line[name_start + 1..].trim_end_matches(']');
            Some(State::Dependency(name))
        } else if line.ends_with("dependencies]") {
            Some(State::Dependencies)
        } else if line.starts_with('[') {
            Some(State::Seeking)
        } else {
            None
        };

        if let Some(new_state) = new_state {
            if let State::Dependency(name) = state
                && name == orig_name
            {
                // finished `[*dependencies.<name>]` without finding a `package = "other"`
                return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
            }

            state = new_state;
            continue;
        }

        match state {
            State::Seeking => continue,
            // Check if it is the crate itself, or one of its tests.
            State::Package => {
                if (line.starts_with("name ") || line.starts_with("name="))
                    && let Some(name_start) = line.find('"')
                    && let Some(name_end) = line.rfind('"')
                {
                    let name = &line[name_start + 1..name_end];

                    if name == orig_name {
                        return Ok(if env::var_os("CARGO_TARGET_TMPDIR").is_none() {
                            FoundCrate::Itself
                        } else {
                            FoundCrate::Name(orig_name.replace('-', "_"))
                        });
                    }
                }
            }
            // Check dependencies, dev-dependencies, target.`..`.dependencies
            State::Dependencies => {
                if let Some(eq) = line.find('=') {
                    let name = line[..eq].trim();
                    let value = line[eq + 1..].trim();

                    if value.starts_with('"') {
                        if name == orig_name {
                            return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
                        }
                    } else if value.starts_with('{') {
                        let value = value.replace(' ', "");
                        if let Some(pkg) = value.find("package=\"") {
                            let pkg = &value[pkg + "package=\"".len()..];
                            if let Some(pkg_name_end) = pkg.find('"') {
                                let pkg_name = &pkg[..pkg_name_end];
                                if pkg_name == orig_name {
                                    return Ok(FoundCrate::Name(name.replace('-', "_")));
                                }
                            }
                        } else if name == orig_name {
                            return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
                        }
                    }
                }
            }
            // Check a dependency in the style [dependency.foo]
            State::Dependency(name) => {
                if (line.starts_with("package ") || line.starts_with("package="))
                    && let Some(pkg_name_start) = line.find('"')
                    && let Some(pkg_name_end) = line.rfind('"')
                {
                    let pkg_name = &line[pkg_name_start + 1..pkg_name_end];

                    if pkg_name == orig_name {
                        return Ok(FoundCrate::Name(name.replace('-', "_")));
                    }
                }
            }
        }
    }

    if let State::Dependency(name) = state
        && name == orig_name
    {
        // finished `[*dependencies.<name>]` without finding a `package = "other"`
        return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
    }

    Err(())
}

/// Returns `true` if the proc-macro is running in one of the rust-analyzer proc-macro servers.
#[expect(unexpected_cfgs)] // rust_analyzer exists: https://github.com/rust-lang/rust-analyzer/pull/15528
pub fn is_rust_analyzer() -> bool {
    cfg!(rust_analyzer)
}
