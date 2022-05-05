//! This macro can almost be implemented using macro_rules!, we only need a proc-macro
//! to get the tuple member index for for each input and split off the last expression as the merge
//! closure, instead of another input.

use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Expr, Path, Token,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { vars_mod, mut inputs, .. } = parse_macro_input!(input as Input);

    if inputs.len() < 3 {
        abort_call_site!("expected at least two input vars and one merge closure")
    }

    let merge = inputs.pop().unwrap();
    let idx: Vec<_> = (0..inputs.len()).map(|i| syn::Index {
        index: i as u32,
        span: proc_macro2::Span::call_site(),
    }).collect();

    let out = quote! {
        {
            let inputs__ = (#inputs);
            let input_types__ = (#(#vars_mod::types::RcMergeVarInput::new(&inputs__.#idx)),*);
            let mut merge__ = #merge;
            #vars_mod::types::RcMergeVar::new(
                Box::new([#(Box::new(inputs__.#idx)),*]),
                Box::new(move |vars, inputs| {
                    merge__(#(input_types__.#idx.get(&inputs[#idx])),*)
                })
            )
        }
    };

    out.into()
}

struct Input {
    vars_mod: Path,
    _comma: Token![,],
    inputs: Punctuated<Expr, Token![,]>,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            vars_mod: input.parse()?,
            _comma: input.parse()?,
            inputs: Punctuated::parse_terminated(input)?,
        })
    }
}
