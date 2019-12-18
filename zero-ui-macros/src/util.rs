// cannot macro-export so use include!("util.rs") to import.

/// `Ident` with call_site span.
#[allow(unused)]
fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Same as `parse_quote` but with an `expect` message.
#[allow(unused)]
macro_rules! dbg_parse_quote {
    ($msg:expr, $($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).expect($msg)
    };
}