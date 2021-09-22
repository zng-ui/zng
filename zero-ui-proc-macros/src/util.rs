use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use proc_macro2::*;
use quote::{quote_spanned, ToTokens};
use syn::{
    self,
    parse::{discouraged::Speculative, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Token,
};

/// `Ident` with custom span.
macro_rules! ident_spanned {
    ($span:expr=> $name:expr) => {
        proc_macro2::Ident::new($name, $span)
    };
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

/// `quote_spanned!` + `parse_quote!` combo.
macro_rules! parse_quote_spanned {
    ( $span:expr => $($tt:tt)+ ) => {{
        let quoted = quote_spanned!( $span => $($tt)+ );
        parse_quote!( #quoted )
    }};
}

/// Return `$crate::core` where `$crate` is the zero-ui
/// crate name in the crate using our proc-macros. Or, returns `$crate` where `$crate`
/// is the zero-ui-core crate if the crate using our proc-macros does not use the main zero-ui crate.
pub fn crate_core() -> TokenStream {
    use once_cell::sync::OnceCell;
    static CRATE: OnceCell<(String, bool)> = OnceCell::new();

    let (ident, core) = CRATE.get_or_init(|| {
        use proc_macro_crate::{crate_name, FoundCrate};
        if let Ok(ident) = crate_name("zero-ui") {
            // using the main crate.
            match ident {
                FoundCrate::Name(name) => (name, true),
                FoundCrate::Itself => ("zero_ui".to_owned(), true),
            }
        } else if let Ok(ident) = crate_name("zero-ui-core") {
            // using the core crate only.
            match ident {
                FoundCrate::Name(name) => (name, false),
                FoundCrate::Itself => ("zero_ui_core".to_owned(), false),
            }
        } else {
            // proc_macro_crate failed?
            ("zero_ui".to_owned(), true)
        }
    });

    let ident = Ident::new(ident, Span::call_site());
    if *core {
        quote! { #ident::core }
    } else {
        ident.to_token_stream()
    }
}

/// Generates a return of a compile_error message in the given span.
macro_rules! abort {
    ($span:expr, $($tt:tt)*) => {{
        let error = format!($($tt)*);
        let error = LitStr::new(&error, Span::call_site());

        return quote_spanned!($span=> compile_error!{#error}).into();
    }};
}

/// Generates a return of a compile_error message in the call_site span.
macro_rules! abort_call_site {
    ($($tt:tt)*) => {
        abort!(Span::call_site(), $($tt)*)
    };
}

/// Extend a TokenStream with a `#[doc]` attribute.
macro_rules! doc_extend {
    ($tokens:ident, $str:expr) => {
        {
            let doc_comment = $str;
            for line in doc_comment.lines() {
                $tokens.extend(quote_spanned!(proc_macro2::Span::call_site()=> #[doc=#line]));
            }
        }
    };
    ($tokens:ident, $($tt:tt)*) => {
        {
            let doc_comment = format!($($tt)*);
            for line in doc_comment.lines() {
                $tokens.extend(quote_spanned!(proc_macro2::Span::call_site()=> #[doc=#line]));
            }
        }
    }
}

/// Input error not caused by the user.
macro_rules! non_user_error {
    ($e:expr) => {
        panic!("[{}:{}] invalid non-user input: {}", file!(), line!(), $e)
    };
    ($fmt:tt, $($args:tt)+) => {
        non_user_error! { format_args!($fmt, $($args)+) }
    }
}

/// Include minified JS string from the "src/js" dir.
macro_rules! js {
    ($file_name:tt) => {
        include_str!(concat!(env!("OUT_DIR"), "/js_min/", $file_name))
    };
}

/// Like [`js!`] but quoted with `<script>..</script>` tag.
macro_rules! js_tag {
    ($file_name:tt) => {
        concat!("<script>", js!($file_name), "</script>")
    };
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
                non_user_error!(format!("expected `{}`", ident));
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

/// Parse items of a type until the end of the parse stream or the first error.
pub fn parse_all<T: Parse>(input: syn::parse::ParseStream) -> syn::Result<Vec<T>> {
    let mut result = vec![];

    while !input.is_empty() {
        result.push(input.parse()?)
    }

    Ok(result)
}

/// Unique id of the current [`Span::call_site()`] or a random unique id if the call_site is not distinct.
pub fn uuid() -> String {
    let call_site = format!("{:?}", Span::call_site());
    if call_site == "Span" {
        static ID: AtomicU64 = AtomicU64::new(0);
        let mut id = ID.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
            ID.store(id, Ordering::Relaxed);
        }
        format!("sp_{:x}", id)
    } else {
        call_site.splitn(2, ' ').next().unwrap().replace('#', "u")
    }
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

    /// Push all compile errors in `errors`.
    pub fn extend(&mut self, errors: Errors) {
        self.tokens.extend(errors.tokens)
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
impl ToTokens for Errors {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.tokens.clone().into_iter())
    }
    fn to_token_stream(&self) -> TokenStream {
        self.tokens.clone()
    }
    fn into_token_stream(self) -> TokenStream {
        self.tokens
    }
}

/// Separated attributes.
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
            if let Some(ident) = attr.path.get_ident() {
                if ident == "doc" {
                    docs.push(attr);
                    continue;
                } else if ident == "inline" {
                    inline = Some(attr);
                } else if ident == "cfg" {
                    cfg = Some(attr);
                } else if ident == "allow" || ident == "warn" || ident == "deny" || ident == "forbid" {
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
}

/// Gets if any of the attributes is #[doc(hidden)].
pub fn is_doc_hidden(docs: &[Attribute]) -> bool {
    let expected = quote! { (hidden) };
    docs.iter()
        .any(|a| token_stream_eq(a.tokens.clone(), expected.clone()) && a.path.get_ident().map(|id| id == "doc").unwrap_or_default())
}

/// Gets if any of the attributes in the unparsed stream is #[doc(hidden)].
pub fn is_doc_hidden_tt(docs: TokenStream) -> bool {
    let attrs = syn::parse2::<OuterAttrs>(docs).unwrap().attrs;
    is_doc_hidden(&attrs)
}

/// Insert `<script>{js}</script>` ad the end of the first `docs` line escaped to activate in the item
/// parent module summary list page.
pub fn docs_with_first_line_js(output: &mut TokenStream, docs: &[Attribute], js: &'static str) {
    if docs.is_empty() {
        doc_extend!(output, "{}", js);
    } else {
        let inner = docs[0].tokens.to_string();
        let mut skip = 0;
        if let Some(doc) = inner.strip_prefix('=') {
            let doc = doc.trim_start().trim_start_matches('r').trim_start_matches('#');
            if let Some(doc) = doc.strip_prefix('"') {
                // is #[doc=".."] like attribute.
                // inject JS without breaking line so that it is included in the item summary.

                let doc = &doc[..doc.len() - 1]; // remove \" end

                // replace characters `rustdoc` incorrectly changes.
                doc_extend!(output, "{}<script>{}</script>\n\n", doc, js.replace("'", "&#39;"));
                skip = 1;
            }
        }

        for attr in docs.iter().skip(skip) {
            attr.to_tokens(output);
        }
    }
}

/// Convert a [`Path`] to a formatted [`String`].
pub fn display_path(path: &syn::Path) -> String {
    path.to_token_stream().to_string().replace(" ", "")
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`TokenStream`].
pub fn tokens_to_ident_str(tokens: &TokenStream) -> String {
    let tokens = tokens.to_string();
    let max = tokens.len().min(40);
    let mut tokens = tokens[(tokens.len() - max)..]
        .replace(&['.', ':', ' '][..], "_")
        .replace("!", "not")
        .replace("&&", "and")
        .replace("||", "or")
        .replace("(", "p")
        .replace(")", "b")
        .replace("==", "eq");

    tokens.retain(|c| c == '_' || c.is_alphanumeric());

    tokens
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`Path`].
pub fn path_to_ident_str(path: &syn::Path) -> String {
    tokens_to_ident_str(&path.to_token_stream())
}

/// Gets a span that best represent the path.
pub fn path_span(path: &syn::Path) -> Span {
    path.segments.last().map(|s| s.span()).unwrap_or_else(|| path.span())
}

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

/// Merges both `#[cfg]` attributes so that only if both conditions are true the item is compiled.
pub fn cfg_attr_and(a: Option<Attribute>, b: Option<Attribute>) -> Option<TokenStream> {
    match (a, b) {
        (None, None) => None,
        (None, Some(b)) => Some(b.to_token_stream()),
        (Some(a), None) => Some(a.to_token_stream()),
        (Some(a), Some(b)) => match (syn::parse2::<CfgCondition>(a.tokens), syn::parse2::<CfgCondition>(b.tokens)) {
            (Ok(a), Ok(b)) => {
                if token_stream_eq(a.tokens.clone(), b.tokens.clone()) {
                    Some(quote! { #[cfg(#a)] })
                } else {
                    Some(quote! { #[cfg(all(#a, #b))] })
                }
            }
            (Ok(a), Err(_)) => Some(quote! { #[cfg(#a)] }),
            (Err(_), Ok(b)) => Some(quote! { #[cfg(#b)] }),
            (Err(_), Err(_)) => None,
        },
    }
}

/// Merges both `#[cfg]` attributes so that if any of the two conditions are true the item is compiled.
pub fn cfg_attr_or(a: Option<Attribute>, b: Option<Attribute>) -> Option<TokenStream> {
    match (a, b) {
        (None, _) => None,
        (_, None) => None,
        (Some(a), Some(b)) => match (syn::parse2::<CfgCondition>(a.tokens), syn::parse2::<CfgCondition>(b.tokens)) {
            (Err(_), _) => None,
            (_, Err(_)) => None,
            (Ok(a), Ok(b)) => {
                if token_stream_eq(a.tokens.clone(), b.tokens.clone()) {
                    Some(quote! { #[cfg(#a)] })
                } else {
                    Some(quote! { #[cfg(any(all(#a), all(#b)))] })
                }
            }
        },
    }
}

/// Returns an `#[cfg(..)]` stream that is the inverse of the `cfg` condition.
///
/// It the `cfg` is `None` returns a cfg match to a feature that is never set.
pub fn cfg_attr_not(cfg: Option<Attribute>) -> TokenStream {
    match cfg {
        Some(cfg) => {
            if !cfg.path.get_ident().map(|id| id == "cfg").unwrap_or_default() {
                non_user_error!("not a cfg attribute")
            } else {
                let span = cfg.span();
                let condition = cfg.tokens; // note: already includes the parenthesis
                quote_spanned! {span=>
                    #[cfg(not#condition)]
                }
            }
        }
        None => quote! {
            #[cfg(zero_ui_never_set)]
        },
    }
}
struct CfgCondition {
    tokens: TokenStream,
}
impl Parse for CfgCondition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        syn::parenthesized!(inner in input);
        Ok(CfgCondition { tokens: inner.parse()? })
    }
}
impl ToTokens for CfgCondition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.tokens.to_tokens(tokens);
    }
}

/// Parse an attribute.
pub fn parse_attr(input: TokenStream) -> Result<Attribute, syn::Error> {
    syn::parse2::<OuterAttr>(input).map(|a| a.into())
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

        #[allow(clippy::eval_order_dependence)] // see https://github.com/rust-lang/rust-clippy/issues/4637
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
            path: s.path,
            tokens: s.tokens,
        }
    }
}

struct OuterAttrs {
    attrs: Vec<Attribute>,
}
impl syn::parse::Parse for OuterAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(OuterAttrs {
            attrs: Attribute::parse_outer(input)?,
        })
    }
}

/// Convert a #[cfg(..)] attribute token stream to a string that can be displayed in a HTML element title attribute.
pub fn html_title_cfg(cfg: TokenStream) -> String {
    cfg.to_string().replace(" ", "").replace(",", ", ").replace("\"", "&quot;")
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

/// Gets a span that indicates a position after the item.
pub fn after_span<T: ToTokens>(tt: &T) -> Span {
    last_span(tt.to_token_stream())
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

/// Takes lint attributes in the `zero_ui::` namespace.
///
/// Pushes `errors` for unsupported `warn` and already attempt of setting
/// level of forbidden zero_ui lints.
///
/// NOTE: We add an underline `_` after the lint ident because rustc validates
/// custom tools even for lint attributes removed by proc-macros.
pub fn take_zero_ui_lints(
    attrs: &mut Vec<Attribute>,
    errors: &mut Errors,
    forbidden: &std::collections::HashSet<&Ident>,
) -> Vec<(Ident, LintLevel, Attribute)> {
    let mut r = vec![];
    let mut i = 0;
    while i < attrs.len() {
        if let Some(ident) = attrs[i].path.get_ident() {
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
            if let Ok(path) = syn::parse2::<LintPath>(attrs[i].tokens.clone()) {
                let path = path.path;
                if path.segments.len() == 2 && path.segments[0].ident == "zero_ui" {
                    let attr = attrs.remove(i);
                    let lint_ident = path.segments[1].ident.clone();
                    match level {
                        LintLevel::Warn => errors.push(
                            "cannot set zero_ui lints to warn because warning diagnostics are not stable",
                            attr.path.span(),
                        ),
                        LintLevel::Allow if forbidden.contains(&lint_ident) => errors.push(
                            format_args!("lint `zero_ui::{}` is `forbid` in this context", lint_ident),
                            attr.span(),
                        ),
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
        #[allow(clippy::eval_order_dependence)] // see https://github.com/rust-lang/rust-clippy/issues/4637
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
    format!("{:?}", a) == format!("{:?}", b)
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

/// Set `span` in all tokens of `token_stream`.
pub fn set_span(token_stream: &mut TokenStream, span: Span) {
    let mut r = TokenStream::default();
    for mut tt in token_stream.clone() {
        if let TokenTree::Group(g) = tt {
            let mut inner = g.stream();
            set_span(&mut inner, span);
            let mut g = proc_macro2::Group::new(g.delimiter(), inner);
            g.set_span(span);
            g.to_tokens(&mut r);
        } else {
            tt.set_span(span);
            tt.to_tokens(&mut r);
        }
    }
    *token_stream = r;
}

// Debug tracing if it was enabled during run-time.
//
// This is useful for debugging say the widget macros but only for a widget.
//
// Use [`enable_trace!`] and [`trace!`].
#[allow(unused)]
#[cfg(debug_assertions)]
pub mod debug_trace {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ENABLED: AtomicBool = AtomicBool::new(false);

    pub fn enable(enable: bool) {
        let prev = ENABLED.swap(enable, Ordering::SeqCst);
        if prev != enable {
            eprintln!("zero-ui-proc-macros::debug_trace {}", if enable { "enabled" } else { "disabled" });
        }
    }

    pub fn display(msg: impl std::fmt::Display) {
        if ENABLED.load(Ordering::SeqCst) {
            eprintln!("{}", msg);
        }
    }
}

#[allow(unused)]
#[cfg(debug_assertions)]
macro_rules! enable_trace {
    () => {
        $crate::util::debug_trace::enable(true);
    };
    (if $bool_expr:expr) => {
        $crate::util::debug_trace::enable($bool_expr);
    };
}
#[allow(unused)]
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

/// Convert CamelCase to snake_case.
pub fn snake_case(camel: &str) -> String {
    let mut r = String::new();
    let mut prev = '_';
    for ch in camel.chars() {
        if ch.is_uppercase() && prev != '_' {
            r.push('_');
        }
        r.push(ch);
        prev = ch;
    }
    r.to_lowercase()
}
