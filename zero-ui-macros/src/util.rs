// cannot macro-export so use include!("util.rs") to import.

/// `Ident` with call_site span.
#[allow(unused)]
macro_rules! ident {
    ($name:expr) => {
        proc_macro2::Ident::new($name, proc_macro2::Span::call_site())
    };
    ($($format_name:tt)+) => {
        proc_macro2::Ident::new(&format!($($format_name)+), proc_macro2::Span::call_site())
    };
}

/// generates `pub`
#[allow(unused)]
fn pub_vis() -> Visibility {
    Visibility::Public(VisPublic {
        pub_token: syn::token::Pub {
            span: Span::call_site(),
        },
    })
}

///-> (docs, other_attrs)
#[allow(unused)]
fn split_doc_other(attrs: &mut Vec<Attribute>) -> (Vec<Attribute>, Vec<Attribute>) {
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
#[allow(unused)]
fn zero_ui_crate_ident() -> Ident {
    proc_macro_crate::crate_name("zero-ui").map(|n|ident!(&n)).unwrap_or_else(|_|ident!("zero_ui"))
}

/// Same as `parse_quote` but with an `expect` message.
#[allow(unused)]
macro_rules! dbg_parse_quote {
    ($msg:expr, $($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).expect($msg)
    };
}

#[allow(unused)]
macro_rules! abort {
    ($span:expr, $($tt:tt)*) => {{
        let error = format!($($tt)*);
        let error = LitStr::new(&error, Span::call_site());

        return quote_spanned!($span=> compile_error!{#error}).into();
    }};
}

#[allow(unused)]
macro_rules! abort_call_site {
    ($($tt:tt)*) => {
        abort!(Span::call_site(), $($tt)*)
    };
}

#[allow(unused)]
macro_rules! doc {
    ($($tt:tt)*) => {{
        let doc_lit = LitStr::new(&format!($($tt)*), Span::call_site());
        let doc: Attribute = parse_quote!(#[doc=#doc_lit]);
        doc
    }};
}

/// Input error not caused by the user.
const NON_USER_ERROR: &str = "invalid non-user input";

#[allow(unused)]
fn non_user_braced(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        braced!(inner in input);
        Ok(inner)
    }
    inner(input).expect(NON_USER_ERROR)
}

#[allow(unused)]
fn non_user_parenthesized(input: syn::parse::ParseStream) -> syn::parse::ParseBuffer {
    fn inner(input: syn::parse::ParseStream) -> Result<syn::parse::ParseBuffer> {
        let inner;
        // this macro inserts a return Err(..) but we want to panic
        parenthesized!(inner in input);
        Ok(inner)
    }
    inner(input).expect(NON_USER_ERROR)
}