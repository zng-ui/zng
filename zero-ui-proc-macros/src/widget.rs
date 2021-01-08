use quote::ToTokens;
use syn::{ItemMod, Path, parse_macro_input};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // a `$crate` path to the widget module.
    let crate_mod = parse_macro_input!(args as Path);
    // the widget mod declaration. 
    let mod_ = parse_macro_input!(input as ItemMod);

    let tokens = mod_.to_token_stream();
    tokens.into()
}
