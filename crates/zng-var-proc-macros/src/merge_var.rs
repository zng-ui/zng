use syn::{
    Expr, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { vars_mod, mut inputs, .. } = parse_macro_input!(input as Input);

    if inputs.len() < 3 {
        abort_call_site!("expected at least two input vars and one merge closure")
    }

    let merge = inputs.pop().unwrap();

    let type_idents: Vec<_> = (0..inputs.len()).map(|i| ident!("T{i}")).collect();
    let var_idents: Vec<_> = (0..inputs.len()).map(|i| ident!("V{i}")).collect();
    let input_idents: Vec<_> = (0..inputs.len()).map(|i| ident!("var{i}")).collect();
    let idx: Vec<_> = (0..inputs.len())
        .map(|i| syn::Index {
            index: i as _,
            span: proc_macro2::Span::call_site(),
        })
        .collect();

    let out = quote! {
        {
            #[inline(always)]
            fn merge_var__<
                #(#type_idents: #vars_mod::VarValue,)*
                #(#var_idents: #vars_mod::Var<#type_idents>,)*
                O: #vars_mod::VarValue,
                F: FnMut(
                    #(&#type_idents,)*
                ) -> O + Send + 'static
            >(
                #(#input_idents: #var_idents,)*
                mut merge: F
            ) -> #vars_mod::BoxedVar<O> {
                let input_types = (
                    #(#vars_mod::types::ArcMergeVarInput::new(&#input_idents)),*
                );
                #vars_mod::types::ArcMergeVar::new(
                    Box::new([
                        #(Box::new(#input_idents),)*
                    ]),
                    move |inputs| {
                        merge(
                            #(input_types.#idx.get(&inputs[#idx])),*
                        )
                    }
                )
            }
            merge_var__(#inputs #merge)
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
