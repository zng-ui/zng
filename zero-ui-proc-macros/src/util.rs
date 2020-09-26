use parse::{Parse, ParseStream};
use proc_macro2::*;
use punctuated::Punctuated;
use quote::ToTokens;
use syn::*;

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

/// returns `zero_ui` or the name used in `Cargo.toml` if the crate was
/// renamed.
pub fn zero_ui_crate_ident() -> Ident {
    use once_cell::sync::OnceCell;
    static CRATE: OnceCell<String> = OnceCell::new();

    let crate_ = CRATE.get_or_init(|| proc_macro_crate::crate_name("zero-ui").unwrap_or_else(|_| "zero_ui".to_owned()));

    Ident::new(crate_.as_str(), Span::call_site())
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
            $tokens.extend(quote!(#[doc=#doc_comment]));
        }
    };
    ($tokens:ident, $($tt:tt)*) => {
        {
            let doc_comment = format!($($tt)*);
            $tokens.extend(quote!(#[doc=#doc_comment]));
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

/// Does a `braced!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
pub fn non_user_braced(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        braced!(inner in input);
        Ok(inner)
    }
    inner(input).unwrap_or_else(|e| non_user_error!(e))
}

/// Does a `parenthesized!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
pub fn non_user_parenthesized(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        parenthesized!(inner in input);
        Ok(inner)
    }
    inner(input).unwrap_or_else(|e| non_user_error!(e))
}

pub fn uuid() -> impl std::fmt::Display {
    uuid::Uuid::new_v4().to_simple()
}

/// Parse a `Punctuated` from a `TokenStream`.
pub fn parse_terminated2<T: Parse, P: Parse>(tokens: TokenStream) -> parse::Result<Punctuated<T, P>> {
    parse2::<PunctParser<T, P>>(tokens).map(|p| p.0)
}
struct PunctParser<T, P>(Punctuated<T, P>);
impl<T: Parse, P: Parse> Parse for PunctParser<T, P> {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::<T, P>::parse_terminated(input).map(Self)
    }
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

    /*
    pub fn extend(&mut self, errors: Errors) {
        self.tokens.extend(errors.tokens)
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
    */
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
    pub others: Vec<Attribute>,
}

impl Attributes {
    pub fn new(attrs: Vec<Attribute>) -> Self {
        let mut docs = vec![];
        let mut inline = None;
        let mut cfg = None;
        let mut others = vec![];

        let doc_ident = ident!("doc");
        let inline_ident = ident!("inline");
        let cfg_ident = ident!("cfg");

        for attr in attrs {
            if let Some(ident) = attr.path.get_ident() {
                if ident == &doc_ident {
                    docs.push(attr);
                    continue;
                } else if ident == &inline_ident {
                    inline = Some(attr);
                } else if ident == &cfg_ident {
                    cfg = Some(attr);
                } else {
                    others.push(attr);
                }
            } else {
                others.push(attr);
            }
        }

        Attributes { docs, inline, cfg, others }
    }
}

pub fn docs_with_first_line_js(output: &mut TokenStream, docs: &[Attribute], js: &'static str) {
    if docs.is_empty() {
        doc_extend!(output, "{}", js);
    } else {
        let inner = docs[0].tokens.to_string();
        if inner.starts_with('=') {
            let doc = &inner[1..].trim_start().trim_start_matches('r').trim_start_matches('#');
            if doc.starts_with('"') {
                // is #[doc=".."] like attribute.
                // inject JS without breaking line so that it is included in the item summary.

                let doc = &doc[1..]; // remove \" start
                let doc = &doc[..doc.len() - 1]; // remove \" end

                doc_extend!(output, "{}{}\n\n", doc, js);
                return;
            }
        }

        for attr in docs.iter().skip(1) {
            attr.to_tokens(output);
        }
    }
}
