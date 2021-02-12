use std::{env, path::PathBuf};

use proc_macro2::*;
use quote::{quote_spanned, ToTokens};
use regex::Regex;
use syn::{
    self,
    parse::{Parse, ParseStream},
    parse_quote,
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

/// Return `$crate::core` where `$crate` is the zero-ui
/// crate name in the crate using our proc-macros. Or, returns `$crate` where `$crate`
/// is the zero-ui-core crate if the crate using our proc-macros does not use the main zero-ui crate.
pub fn crate_core() -> TokenStream {
    use once_cell::sync::OnceCell;
    use proc_macro_crate::crate_name;
    static CRATE: OnceCell<(String, bool)> = OnceCell::new();

    let (ident, core) = CRATE.get_or_init(|| {
        if let Ok(ident) = crate_name("zero-ui") {
            // using the main crate.
            (ident, true)
        } else if let Ok(ident) = crate_name("zero-ui-core") {
            // using the core crate only.
            (ident, false)
        } else if let Ok(true) = in_crate_core() {
            // using in the zero-ui-core crate, it re-exports self as zero_ui_core to work in examples.
            ("zero_ui_core".to_owned(), false)
        } else {
            // using in the zero-ui-core crate, it re-exports self as zero_ui to work in examples.
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
fn in_crate_core() -> std::result::Result<bool, ()> {
    use std::io::Read;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| ())?;

    let cargo_toml_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    let mut content = String::new();
    std::fs::File::open(cargo_toml_path)
        .map_err(|_| ())?
        .read_to_string(&mut content)
        .map_err(|_| ())?;

    Ok(content.contains(r#"name = "zero-ui-core""#))
}

/// Same as `parse_quote` but with an `expect` message.
#[allow(unused)]
macro_rules! dbg_parse_quote {
    ($msg:expr, $($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).expect($msg)
    };
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
                $tokens.extend(quote!(#[doc=#line]));
            }
        }
    };
    ($tokens:ident, $($tt:tt)*) => {
        {
            let doc_comment = format!($($tt)*);
            for line in doc_comment.lines() {
                $tokens.extend(quote!(#[doc=#line]));
            }
        }
    }
}

/// Generates a string with the code of `input` parse stream. The stream is not modified.
#[allow(unused)]
macro_rules! dump_parse {
    ($input:ident) => {{
      let input = $input.fork();
      let tokens: TokenStream = input.parse().unwrap();
      format!("{}", quote!(#tokens))
    }};
}

/// Input error not caused by the user.
macro_rules! non_user_error {
    ($e:expr) => {
        panic!("[{}:{}] invalid non-user input: {}", file!(), line!(), $e)
    };
}

/// Include minified JS string from the "src/js" dir.
macro_rules! js {
    ($file_name:tt) => {
        include_str!(concat!(env!("OUT_DIR"), "\\js_min\\", $file_name))
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

pub fn parse_all<T: Parse>(input: syn::parse::ParseStream) -> syn::Result<Vec<T>> {
    let mut result = vec![];

    while !input.is_empty() {
        result.push(input.parse()?)
    }

    Ok(result)
}

pub fn uuid() -> impl std::fmt::Display {
    // could also be format!("{:?}", Span::call_site()).splitn(2, ' ').next().unwrap()[1..].to_string();
    uuid::Uuid::new_v4().to_simple()
}

struct PunctParser<T, P>(Punctuated<T, P>);
impl<T: Parse, P: Parse> Parse for PunctParser<T, P> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Punctuated::<T, P>::parse_terminated(input).map(Self)
    }
}

/// [`Punctuated::parse_terminated`] from a token stream.
pub fn parse2_punctuated<T: Parse, P: Parse>(input: TokenStream) -> syn::Result<Punctuated<T, P>> {
    syn::parse2::<PunctParser<T, P>>(input).map(|r| r.0)
}

/// Collection of compile errors.
#[derive(Default)]
pub struct Errors {
    tokens: TokenStream,
}
impl Errors {
    pub fn push(&mut self, error: impl ToString, span: Span) {
        let error = error.to_string();
        self.tokens.extend(quote_spanned! {span=>
            compile_error!{#error}
        })
    }

    pub fn push_syn(&mut self, error: syn::Error) {
        let span = error.span();
        self.push(error, span)
    }

    pub fn extend(&mut self, errors: Errors) {
        self.tokens.extend(errors.tokens)
    }

    // pub fn is_empty(&self) -> bool {
    //     self.tokens.is_empty()
    // }
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

                doc_extend!(output, "{}{}\n\n", doc, js);
                skip = 1;
            }
        }

        for attr in docs.iter().skip(skip) {
            attr.to_tokens(output);
        }
    }
}

/// Split docs with line breaks into different doc attributes.
#[allow(unused)]
pub fn normalize_docs(docs: &[Attribute]) -> Vec<Attribute> {
    let mut r = Vec::with_capacity(docs.len());
    for a in docs {
        if let syn::AttrStyle::Inner(_) = a.style {
            r.push(a.clone());
        } else {
            let doc: DocArgs = syn::parse2(a.tokens.clone()).unwrap();
            for line in doc.str_.value().lines() {
                r.push(parse_quote!( #[doc=#line] ));
            }
        }
    }
    r
}

struct DocArgs {
    _eq: Token![=],
    str_: syn::LitStr,
}
impl Parse for DocArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(DocArgs {
            _eq: input.parse()?,
            str_: input.parse()?,
        })
    }
}

/// Inserts extra `super::` in paths that start with super that reference
/// out of the implied mod visited.
pub struct PatchSuperPath {
    super_ident: Ident,
    new_depth: usize,
    mod_depth: usize,
}
impl PatchSuperPath {
    /// `new_depth` is the number of `super::` to insert the paths.
    pub fn new(new_depth: usize) -> Self {
        PatchSuperPath {
            super_ident: ident!("super"),
            new_depth,
            mod_depth: 0,
        }
    }
}
impl syn::visit_mut::VisitMut for PatchSuperPath {
    fn visit_path_mut(&mut self, i: &mut syn::Path) {
        syn::visit_mut::visit_path_mut(self, i);

        // if the path does not start with ::
        if i.leading_colon.is_none() {
            // count super::(super::)?.
            let mut super_count = 0;
            for seg in i.segments.iter() {
                if seg.ident == self.super_ident {
                    super_count += 1;
                } else {
                    break;
                }
            }

            // if the path super:: prefixes reference out of the outer mod visited.
            if super_count > 0 && super_count > self.mod_depth {
                let first = i.segments[0].clone();

                // insert the `new_depth` count of supers in the `0` index.
                for _ in 1..self.new_depth {
                    i.segments.insert(0, first.clone());
                }
                i.segments.insert(0, first);
            }
        }
    }

    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        self.mod_depth += 1;
        syn::visit_mut::visit_item_mod_mut(self, i);
        self.mod_depth -= 1;
    }
}

/// Convert a [`Path`] to a formatted [`String`].
pub fn display_path(path: &syn::Path) -> String {
    path.to_token_stream().to_string().replace(" ", "")
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`TokenStream`].
pub fn tokens_to_ident_str(tokens: &TokenStream) -> String {
    let tokens = tokens.to_string();
    let max = tokens.len().min(20);
    let tokens = tokens[..max]
        .replace(".", " ")
        .replace(":", " ")
        .replace("!", "not")
        .replace("&&", "and")
        .replace("||", "or")
        .replace("(", "p")
        .replace(")", "b")
        .replace("==", "eq");

    let tokens = Regex::new(r"\s+").unwrap().replace_all(&tokens, "_"); // space sequences to `_`
    let tokens = Regex::new(r"\W").unwrap().replace_all(&tokens, ""); // remove non-word chars

    tokens.to_string()
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`Path`].
pub fn path_to_ident_str(path: &syn::Path) -> String {
    tokens_to_ident_str(&path.to_token_stream())
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
                non_user_error!("not an cfg attribute")
            } else {
                let span = cfg.span();
                let condition = cfg.tokens;
                quote_spanned! {span=>
                    #[cfg(not(#condition))]
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
impl Into<Attribute> for OuterAttr {
    fn into(self) -> Attribute {
        Attribute {
            pound_token: self.pound_token,
            style: self.style,
            bracket_token: self.bracket_token,
            path: self.path,
            tokens: self.tokens,
        }
    }
}
