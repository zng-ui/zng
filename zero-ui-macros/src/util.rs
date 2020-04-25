use proc_macro2::*;
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
