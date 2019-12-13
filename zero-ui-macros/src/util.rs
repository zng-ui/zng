// cannot macro-export so use include!("util.rs") to import.

/// `Ident` with call_site span.
#[allow(unused)]
fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Returns a `TokenStream` with a `compile_error` in the given span with
/// the given error message.
macro_rules! error {
    ($span: expr, $ ($ arg : tt) *) => {{
        let span = $span;
        let error = format!($($arg)*);
        let error = LitStr::new(&error, span);
        let error = quote_spanned! {
            span=>
            compile_error!(concat!("#[impl_ui] ", #error));
        };

        return error.into();
    }};
}

/// Same as `parse_quote` but with an `expect` message.
#[allow(unused)]
macro_rules! dbg_parse_quote {
    ($msg:expr, $($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).expect($msg)
    };
}