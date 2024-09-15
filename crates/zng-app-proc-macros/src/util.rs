use std::{borrow::Cow, env, fmt, fs, path::PathBuf};

use proc_macro2::*;
use quote::{quote_spanned, ToTokens};
use syn::{
    self,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, LitStr, Token,
};

use once_cell::sync::OnceCell;

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

pub fn parse_braces<'a>(input: &syn::parse::ParseBuffer<'a>) -> syn::Result<(syn::token::Brace, syn::parse::ParseBuffer<'a>)> {
    let r;
    let b = syn::braced!(r in input);
    Ok((b, r))
}

/// Returns `true` if the proc-macro is running in one of the rust-analyzer proc-macro servers.
#[expect(unexpected_cfgs)] // rust_analyzer exists: https://github.com/rust-lang/rust-analyzer/pull/15528
pub fn is_rust_analyzer() -> bool {
    cfg!(rust_analyzer)
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
    } else if let Ok(ident) = crate_name("zng-app") {
        // using the core crate only.
        match ident {
            FoundCrate::Name(name) => (name, ""),
            FoundCrate::Itself => ("zng_app".to_owned(), ""),
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
            if let State::Dependency(name) = state {
                if name == orig_name {
                    // finished `[*dependencies.<name>]` without finding a `package = "other"`
                    return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
                }
            }

            state = new_state;
            continue;
        }

        match state {
            State::Seeking => continue,
            // Check if it is the crate itself, or one of its tests.
            State::Package => {
                if line.starts_with("name ") || line.starts_with("name=") {
                    if let Some(name_start) = line.find('"') {
                        if let Some(name_end) = line.rfind('"') {
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
                if line.starts_with("package ") || line.starts_with("package=") {
                    if let Some(pkg_name_start) = line.find('"') {
                        if let Some(pkg_name_end) = line.rfind('"') {
                            let pkg_name = &line[pkg_name_start + 1..pkg_name_end];

                            if pkg_name == orig_name {
                                return Ok(FoundCrate::Name(name.replace('-', "_")));
                            }
                        }
                    }
                }
            }
        }
    }

    if let State::Dependency(name) = state {
        if name == orig_name {
            // finished `[*dependencies.<name>]` without finding a `package = "other"`
            return Ok(FoundCrate::Name(orig_name.replace('-', "_")));
        }
    }

    Err(())
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

macro_rules! non_user_group {
    ($group_kind:ident, $input:expr) => {
        {
            fn inner(input: syn::parse::ParseStream) -> syn::Result<syn::parse::ParseBuffer> {
                let inner;
                // this macro inserts a return Err(..) but we want to panic
                syn::$group_kind!(inner in input);
                Ok(inner)
            }
            inner($input).unwrap_or_else(|e| non_user_error!(e))
        }
    };
    ($group_kind:ident, $input:expr, $ident:expr) => {
        {
            let id: syn::Ident = $input.parse().unwrap_or_else(|e| non_user_error!(e));
            let ident = $ident;
            if id != ident {
                non_user_error!(format!("expected `{ident}`"));
            }
            non_user_group! { $group_kind, $input }
        }
    }
}
/// Does a `braced!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
macro_rules! non_user_braced {
    ($input:expr) => {
        non_user_group! { braced, $input }
    };
    ($input:expr, $ident:expr) => {
        non_user_group! { braced, $input, $ident }
    };
}

/// Does a `parenthesized!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
#[allow(unused)] // depends on cfg
macro_rules! non_user_parenthesized {
    ($input:expr) => {
        non_user_group! { parenthesized, $input }
    };
}

/// Does a `bracketed!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
macro_rules! non_user_bracketed {
    ($input:expr) => {
        non_user_group! { bracketed, $input }
    };
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

    /// Push all compile errors in `error`.
    pub fn push_syn(&mut self, error: syn::Error) {
        for error in error {
            let span = error.span();
            let msg = error.to_string();
            if msg != RECOVERABLE_TAG {
                self.push(error, span);
            }
        }
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

/// Separated attributes.
#[derive(Clone)]
pub struct Attributes {
    pub docs: Vec<Attribute>,
    pub inline: Option<Attribute>,
    pub cfg: Option<Attribute>,
    pub lints: Vec<Attribute>,
    pub others: Vec<Attribute>,
}
impl Attributes {
    pub fn new(attrs: Vec<Attribute>) -> Self {
        let mut docs = vec![];
        let mut inline = None;
        let mut cfg = None;
        let mut lints = vec![];
        let mut others = vec![];

        for attr in attrs {
            if let Some(ident) = attr.path().get_ident() {
                if ident == "doc" {
                    docs.push(attr);
                    continue;
                } else if ident == "inline" {
                    inline = Some(attr);
                } else if ident == "cfg" {
                    cfg = Some(attr);
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
            inline,
            cfg,
            lints,
            others,
        }
    }

    /// Insert a tag on the first doc line, does nothing if docs are missing (to cause a doc missing warning).
    pub fn tag_doc(&mut self, text: &str, help: &str) {
        let txt = format!("<strong title='{help}' data-tag='{text}'><code>{text}</code></strong> ");
        for first in self.docs.iter_mut() {
            match syn::parse2::<DocAttr>(first.tokens()) {
                Ok(doc) => {
                    let mut msg = doc.msg.value();
                    msg.insert_str(0, &txt);
                    *first = parse_quote_spanned! {first.span()=>
                        #[doc = #msg]
                    };

                    return;
                }
                Err(_) => continue,
            }
        }
    }

    pub(crate) fn cfg_and_lints(&self) -> TokenStream {
        let mut tts = self.cfg.to_token_stream();
        for l in &self.lints {
            l.to_tokens(&mut tts);
        }
        tts
    }
}
impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in self
            .docs
            .iter()
            .chain(&self.inline)
            .chain(&self.cfg)
            .chain(&self.lints)
            .chain(&self.others)
        {
            attr.to_tokens(tokens);
        }
    }
}

struct DocAttr {
    msg: LitStr,
}
impl Parse for DocAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![=]>()?;
        Ok(DocAttr { msg: input.parse()? })
    }
}

/// Convert a [`Path`] to a formatted [`String`].
pub fn display_path(path: &syn::Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
}

/// Gets a span that best represent the path.
pub fn path_span(path: &syn::Path) -> Span {
    path.segments.last().map(|s| s.span()).unwrap_or_else(|| path.span())
}

struct OuterAttr {
    pound_token: Token![#],
    style: syn::AttrStyle,
    bracket_token: syn::token::Bracket,
    path: syn::Path,
    tokens: TokenStream,
}
impl syn::parse::Parse for OuterAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;

        Ok(OuterAttr {
            pound_token: input.parse()?,
            style: if input.peek(Token![!]) {
                syn::AttrStyle::Inner(input.parse()?)
            } else {
                syn::AttrStyle::Outer
            },
            bracket_token: syn::bracketed!(inner in input),
            path: inner.parse()?,
            tokens: inner.parse()?,
        })
    }
}
impl From<OuterAttr> for Attribute {
    fn from(s: OuterAttr) -> Self {
        Attribute {
            pound_token: s.pound_token,
            style: s.style,
            bracket_token: s.bracket_token,
            meta: {
                let path = s.path;
                let tokens = s.tokens;
                parse_quote!(#path #tokens)
            },
        }
    }
}

/// Runs `rustfmt` in the `expr`.
pub fn format_rust_expr(value: String) -> String {
    // credits: https://github.com/rust-lang/rustfmt/issues/3257#issuecomment-523573838
    use std::io::Write;
    use std::process::{Command, Stdio};
    const PREFIX: &str = "const x:() = ";
    const SUFFIX: &str = ";\n";
    if let Ok(mut proc) = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--edition=2018")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        {
            let stdin = proc.stdin.as_mut().unwrap();
            stdin.write_all(PREFIX.as_bytes()).unwrap();
            stdin.write_all(value.as_bytes()).unwrap();
            stdin.write_all(SUFFIX.as_bytes()).unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            if output.status.success() {
                // slice between after the prefix and before the suffix
                // (currently 14 from the start and 2 before the end, respectively)
                let start = PREFIX.len() + 1;
                let end = output.stdout.len() - SUFFIX.len();
                return std::str::from_utf8(&output.stdout[start..end]).unwrap().to_owned();
            }
        }
    }
    value
}

/// Gets the span of the last item or the span_close if the last item is a group.
pub fn last_span(tts: TokenStream) -> Span {
    if let Some(tt) = tts.into_iter().last() {
        if let proc_macro2::TokenTree::Group(g) = tt {
            g.span_close()
        } else {
            tt.span()
        }
    } else {
        Span::call_site()
    }
}

/// A lint level.
///
/// NOTE: We add an underline `_` after the lint display name because rustc validates
/// custom tools even for lint attributes removed by proc-macros.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
    Forbid,
}
impl fmt::Display for LintLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LintLevel::Allow => write!(f, "allow_"),
            LintLevel::Warn => write!(f, "warn_"),
            LintLevel::Deny => write!(f, "deny_"),
            LintLevel::Forbid => write!(f, "forbid_"),
        }
    }
}

/// Takes lint attributes in the `zng::` namespace.
///
/// Pushes `errors` for unsupported `warn` and already attempt of setting
/// level of forbidden zng lints.
///
/// NOTE: We add an underline `_` after the lint ident because rustc validates
/// custom tools even for lint attributes removed by proc-macros.
pub fn take_zng_lints(
    attrs: &mut Vec<Attribute>,
    errors: &mut Errors,
    forbidden: &std::collections::HashSet<&Ident>,
) -> Vec<(Ident, LintLevel, Attribute)> {
    let mut r = vec![];
    let mut i = 0;
    while i < attrs.len() {
        if let Some(ident) = attrs[i].path().get_ident() {
            let level = if ident == "allow_" {
                LintLevel::Allow
            } else if ident == "warn_" {
                LintLevel::Warn
            } else if ident == "deny_" {
                LintLevel::Deny
            } else if ident == "forbid_" {
                LintLevel::Forbid
            } else {
                i += 1;
                continue;
            };
            if let Ok(path) = syn::parse2::<LintPath>(attrs[i].tokens()) {
                let path = path.path;
                if path.segments.len() == 2 && path.segments[0].ident == "zng" {
                    let attr = attrs.remove(i);
                    let lint_ident = path.segments[1].ident.clone();
                    match level {
                        LintLevel::Warn => errors.push(
                            "cannot set zng lints to warn because warning diagnostics are not stable",
                            attr.path().span(),
                        ),
                        LintLevel::Allow if forbidden.contains(&lint_ident) => {
                            errors.push(format_args!("lint `zng::{lint_ident}` is `forbid` in this context"), attr.span())
                        }
                        _ => {
                            r.push((lint_ident, level, attr));
                        }
                    }

                    continue; // same i new attribute
                }
            }
        }

        i += 1;
    }
    r
}
struct LintPath {
    _paren: syn::token::Paren,
    path: syn::Path,
}
impl syn::parse::Parse for LintPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        Ok(LintPath {
            _paren: syn::parenthesized!(inner in input),
            path: inner.parse()?,
        })
    }
}

pub fn span_is_call_site(a: proc_macro2::Span) -> bool {
    span_eq(a, proc_macro2::Span::call_site())
}

pub fn span_eq(a: proc_macro2::Span, b: proc_macro2::Span) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// Parses all outer attributes and stores any parsing errors in `errors`.
/// Note: If a malformed attribute is passed, only the attributes after that one will be returned.
pub fn parse_outer_attrs(input: ParseStream, errors: &mut Errors) -> Vec<Attribute> {
    let mut attrs;
    loop {
        let fork = input.fork();
        let mut parsed = true;

        attrs = Attribute::parse_outer(&fork).unwrap_or_else(|e| {
            parsed = false;
            errors.push_syn(e);
            vec![]
        });
        if parsed {
            input.advance_to(&fork);
            break;
        } else {
            let _ = input.parse::<Token![#]>();
            if input.peek(Token![!]) {
                let _ = input.parse::<Token![!]>();
            }
            let _ = non_user_bracketed!(input).parse::<TokenStream>();
        }
    }

    attrs
}

/// New [`syn::Error`] marked [recoverable](ErrorRecoverable).
pub fn recoverable_err(span: Span, msg: impl std::fmt::Display) -> syn::Error {
    syn::Error::new(span, msg).set_recoverable()
}

const RECOVERABLE_TAG: &str = "<recoverable>";
fn recoverable_tag() -> syn::Error {
    syn::Error::new(Span::call_site(), RECOVERABLE_TAG)
}

/// Extension to [`syn::Error`] that lets you mark an error as recoverable,
/// meaning that a sequence of the parse stream is not correct but the parser
/// manage to skip to the end of what was expected to be parsed.
pub trait ErrorRecoverable {
    /// Returns a new error that contains all the errors in `self` but is also marked recoverable.
    fn set_recoverable(self) -> Self;
    /// Returns if `self` is recoverable and all the errors in `self`.
    ///
    /// Note: An error is considered recoverable only if all inner errors are marked recoverable.
    fn recoverable(self) -> (bool, Self);
}
impl ErrorRecoverable for syn::Error {
    fn set_recoverable(self) -> Self {
        let mut errors = self.into_iter();
        let mut e = errors.next().unwrap();

        debug_assert!(e.to_string() != RECOVERABLE_TAG);

        e.combine(recoverable_tag());

        for error in errors {
            if e.to_string() != RECOVERABLE_TAG {
                e.combine(error);
                e.combine(recoverable_tag());
            }
        }

        e
    }
    fn recoverable(self) -> (bool, Self) {
        let mut errors = self.into_iter();
        let mut e = errors.next().unwrap();

        debug_assert!(e.to_string() != RECOVERABLE_TAG);

        let mut errors_count = 1;
        let mut tags_count = 0;

        for error in errors {
            if error.to_string() == RECOVERABLE_TAG {
                tags_count += 1;
            } else {
                errors_count += 1;
                e.combine(error);
            }
        }

        (errors_count == tags_count, e)
    }
}

// Debug tracing if it was enabled during run-time.
//
// This is useful for debugging say the widget macros but only for a widget.
//
// Use [`enable_trace!`] and [`trace!`].
#[allow(unused)] // depends on cfg
#[cfg(debug_assertions)]
pub mod debug_trace {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ENABLED: AtomicBool = AtomicBool::new(false);

    pub fn enable(enable: bool) {
        let prev = ENABLED.swap(enable, Ordering::SeqCst);
        if prev != enable {
            eprintln!("zng-proc-macros::debug_trace {}", if enable { "enabled" } else { "disabled" });
        }
    }

    pub fn display(msg: impl std::fmt::Display) {
        if ENABLED.load(Ordering::SeqCst) {
            eprintln!("{msg}");
        }
    }
}

#[allow(unused)] // depends on cfg
#[cfg(debug_assertions)]
macro_rules! enable_trace {
    () => {
        $crate::util::debug_trace::enable(true);
    };
    (if $bool_expr:expr) => {
        $crate::util::debug_trace::enable($bool_expr);
    };
}
#[allow(unused)] // depends on cfg
#[cfg(debug_assertions)]
macro_rules! trace {
    ($msg:tt) => {
        $crate::util::debug_trace::display($msg);
    };
    ($fmt:tt, $($args:tt)+) => {
        $crate::util::debug_trace::display(format_args!($fmt, $($args)+));
    };
}

/// `Punctuated::parse_terminated` from a `TokenStream`.
pub fn parse_punct_terminated2<T: Parse, P: syn::token::Token + Parse>(input: TokenStream) -> syn::Result<Punctuated<T, P>> {
    struct PunctTerm<T: Parse, P: syn::token::Token + Parse>(Punctuated<T, P>);

    impl<T: Parse, P: syn::token::Token + Parse> Parse for PunctTerm<T, P> {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(Self(Punctuated::parse_terminated(input)?))
        }
    }

    syn::parse2::<PunctTerm<T, P>>(input).map(|p| p.0)
}

/// Returns `true` if the stream has at least 3 more tokens.
pub fn peek_any3(stream: ParseStream) -> bool {
    let mut cursor = stream.cursor();

    if let Some(group) = stream.cursor().group(Delimiter::None) {
        cursor = group.0;
    }

    if let Some((_, cursor)) = cursor.token_tree() {
        if let Some((_, cursor)) = cursor.token_tree() {
            if let Some((_tt, _)) = cursor.token_tree() {
                return true;
            }
        }
    }

    false
}

/// Set the span for each token-tree in the stream.
pub fn set_stream_span(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|mut tt| {
            tt.set_span(span);
            tt
        })
        .collect()
}

pub trait AttributeExt {
    fn tokens(&self) -> TokenStream;
}
impl AttributeExt for Attribute {
    fn tokens(&self) -> TokenStream {
        match &self.meta {
            syn::Meta::Path(_) => quote!(),
            syn::Meta::List(m) => {
                let t = &m.tokens;
                match &m.delimiter {
                    syn::MacroDelimiter::Paren(p) => quote_spanned!(p.span.join()=> (#t)),
                    syn::MacroDelimiter::Brace(b) => quote_spanned!(b.span.join()=> {#t}),
                    syn::MacroDelimiter::Bracket(b) => quote_spanned!(b.span.join()=> [#t]),
                }
            }
            syn::Meta::NameValue(m) => {
                let eq = &m.eq_token;
                let tk = &m.value;
                quote!(#eq #tk)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_itself_1() {
        let toml = r#"
        [package]
        name = "crate-name"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Itself, r);
    }

    #[test]
    fn crate_name_itself_2() {
        let toml = r#"
        [package]
        version = "0.1.0"
        edition = "2021"
        name = "crate-name"
        license = "Apache-2.0"
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Itself, r);
    }

    #[test]
    fn crate_name_dependencies_1() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dependencies]
        bar = "1.0"
        crate-name = "*"

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("crate_name".to_owned()), r);
    }

    #[test]
    fn crate_name_dependencies_2() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dependencies]
        zum = "1.0"
        super-name = { version = "*", package = "crate-name" }

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("super_name".to_owned()), r);
    }

    #[test]
    fn crate_name_dependencies_3() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [target.'cfg(windows)'.dependencies]
        zum = "1.0"
        super-name = { version = "*", package = "crate-name" }

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("super_name".to_owned()), r);
    }

    #[test]
    fn crate_name_dependencies_4() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dev-dependencies]
        zum = "1.0"
        super-name = { version = "*", package = "crate-name" }

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("super_name".to_owned()), r);
    }

    #[test]
    fn crate_name_dependency_1() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dev-dependencies.super-foo]
        version = "*"
        package = "crate-name"

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("super_foo".to_owned()), r);
    }

    #[test]
    fn crate_name_dependency_2() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dependencies.super-foo]
        version = "*"
        package = "crate-name"

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("super_foo".to_owned()), r);
    }

    #[test]
    fn crate_name_dependency_3() {
        let toml = r#"
        [package]
        name = "foo"
        version = "0.1.0"
        edition = "2021"
        license = "Apache-2.0"

        [dependencies.crate-name]
        version = "*"

        [workspace]
        "#;

        let r = crate_name_impl("crate-name", toml).unwrap();
        assert_eq!(FoundCrate::Name("crate_name".to_owned()), r);
    }
}
