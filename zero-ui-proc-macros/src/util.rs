use parse::ParseStream;
use proc_macro2::*;
use std::fmt;
use syn::parse::discouraged::Speculative;
use syn::parse::{Error as ParseError, Parse};
use syn::punctuated::Punctuated;
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

/// generates `pub`
pub fn pub_vis() -> Visibility {
    Visibility::Public(VisPublic {
        pub_token: syn::token::Pub { span: Span::call_site() },
    })
}

///-> (docs, other_attributes)
pub fn split_doc_other(attrs: &mut Vec<Attribute>) -> (Vec<Attribute>, Vec<Attribute>) {
    let mut docs = vec![];
    let mut other_attrs = vec![];

    let doc_ident = ident!("doc");
    let inline_ident = ident!("inline");

    for attr in attrs.drain(..) {
        if let Some(ident) = attr.path.get_ident() {
            if ident == &doc_ident {
                docs.push(attr);
                continue;
            } else if ident == &inline_ident {
                continue;
            }
        }
        other_attrs.push(attr);
    }

    (docs, other_attrs)
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

/// Generates a `#[doc]` attribute.
macro_rules! doc {
    ($($tt:tt)*) => {{
        let doc_lit = LitStr::new(&format!($($tt)*), Span::call_site());
        let doc: Attribute = parse_quote!(#[doc=#doc_lit]);
        doc
    }};
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
pub const NON_USER_ERROR: &str = "invalid non-user input";

/// Does a `braced!` parse but panics with [`NON_USER_ERROR`](NON_USER_ERROR) if the parsing fails.
pub fn non_user_braced(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        braced!(inner in input);
        Ok(inner)
    }
    inner(input).expect(NON_USER_ERROR)
}

/// Does a `parenthesized!` parse but panics with [`NON_USER_ERROR`](NON_USER_ERROR) if the parsing fails.
pub fn non_user_parenthesized(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        parenthesized!(inner in input);
        Ok(inner)
    }
    inner(input).expect(NON_USER_ERROR)
}

// Wait for https://github.com/rust-lang/rust/issues/54140 ?
#[allow(unused)]
macro_rules! idea {()=>{
    /// Macro output builder.
#[derive(Default)]
pub struct MacroOutput {
    out: TokenStream,
    err: TokenStream,
}

impl MacroOutput {
    pub fn new() -> MacroOutput {
        MacroOutput::default()
    }

    pub fn is_empty(&self) -> bool {
        self.out.is_empty() && self.err.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.err.is_empty()
    }

    /// Adds an error.
    pub fn err(&mut self, span: Span, msg: String) {
        self.err.extend(quote_spanned!(span=> compile_error!{#msg}));
    }

    /// Add tokens to output.
    pub fn out(&mut self, t: TokenStream) {
        self.out.extend(t)
    }

    /// Add a doc attribute to output.
    pub fn doc(&mut self, doc: impl Into<DocAttr>) {
        self.out.extend(doc.into().emit())
    }

    /// Parse a value.
    ///
    /// # Recover
    /// If parsing fails `recover` is called. It receives the error and the `input` stream at the position before parsing was
    /// called. It has to advance the stream to the next potentially valid code and return a placeholder value for the failed parsing.
    pub fn parse<T: Parse>(&mut self, input: ParseStream, recover: impl FnOnce(ParseError, ParseStream) -> T) -> T {
        let try_input = input.fork();
        match try_input.parse::<T>() {
            Ok(r) => {
                input.advance_to(&try_input);
                r
            }
            Err(e) => {
                self.err(e.span(), e.to_string());
                recover(e, input)
            }
        }
    }

    /// Parse a value.
    ///
    /// # Recover
    /// If parsing fails `recover` is called. It receives the error and the `input` stream at the position before parsing was
    /// called. It has to advance the stream to the next potentially valid code and return a placeholder value for the failed parsing.
    pub fn parse_terminated<T, P: Parse>(
        &mut self,
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T>,
        recover: impl FnOnce(ParseError, ParseStream) -> Punctuated<T, P>,
    ) -> Punctuated<T, P> {
        let try_input = input.fork();
        match try_input.parse_terminated::<T, P>(parser) {
            Ok(r) => {
                input.advance_to(&try_input);
                r
            }
            Err(e) => {
                self.err(e.span(), e.to_string());
                recover(e, input)
            }
        }
    }

    /// Extends output and errors from `other`.
    pub fn extend(&mut self, other: Self) {
        self.out.extend(other.out);
        self.err.extend(other.err);
    }

    /// Emits final stream.
    pub fn emit(self) -> proc_macro::TokenStream {
        let mut out = self.err;
        out.extend(self.out);
        out.into()
    }
}

/// Documentation attribute builder.
///
/// Use the std `write!` macros to write to this doc.
#[derive(Default)]
pub struct DocAttr {
    doc: String,
    section_state: DocSectionState
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum DocSectionState {
    Open,
    Close
}

impl Default for DocSectionState {
    fn default() -> Self {
        // starts open of the rust-doc creates the item section.
        DocSectionState::Open
    }
}

use fmt::Write;

impl DocAttr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_str(&mut self, string: &str) {
        self.doc.push_str(string);
    }

    /// Opens a custom docs section, closes the section the previous section if it is open.
    pub fn push_section_open(&mut self, id: &str, title: &str) {
        if self.section_state == DocSectionState::Open {
            self.push_section_close();
        }
        self.section_state = DocSectionState::Open;
        writeln!(&mut self.doc,
            r##"\n<h2 id="{0}" class="small-section-header">{1}<a href="#{0}" class="anchor"></a></h2>
            <div class="methods" style="display: block;">"##,
            id,
            title
        ).unwrap();
    }

    pub fn push_section_close(&mut self) {
        assert!(self.section_state == DocSectionState::Open, "docs section already closed");
        self.section_state = DocSectionState::Close;
        writeln!(&mut self.doc, "\n</div>").unwrap();
    }

    pub fn push_js(&mut self, js: &str) {
        writeln!(&mut self.doc, "\n<script>{}</script>", js).unwrap();
    }

    pub fn push_css(&mut self, css: &str) {
        writeln!(&mut self.doc, "\n<script>{}</script>", css).unwrap();
    }

    pub fn emit(self) -> TokenStream {
        let doc = self.doc;
        quote!(#[doc=#doc])
    }
}

impl fmt::Write for DocAttr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.doc.push_str(s);
        Ok(())
    }
}

impl From<String> for DocAttr {
    fn from(doc: String) -> Self {
        DocAttr { doc, ..Self::new() }
    }
}

impl<'a> From<&'a str> for DocAttr {
    fn from(s: &'a str) -> Self {
        DocAttr { doc: s.to_owned(), ..Self::new() }
    }
}
}}