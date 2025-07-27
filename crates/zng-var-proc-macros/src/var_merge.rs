use syn::{
    Expr, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/*
Example generated for 2 inputs:

fn var_merge<I0, I1, O>(input0: impl MergeInputVar<I0>, input1: impl MergeInputVar<I1>, mut merge: impl FnMut(&I0, &I1) -> O + Send + 'static) -> Var<O>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
{
    var_merge(Box::new([input0, input1]), move |inputs| {
        let mut output = None;
        var_merge_with(&inputs[0], &mut |v0| {
            var_merge_with(&inputs[1], &mut |v1| {
                output = Some(var_merge_output(merge(v0.downcast_ref().unwrap(), v1.downcast_ref().unwrap())));
            })
        });
        output.unwrap()
    });
}

var_merge(input0_expr, input1_expr)
*/

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { vars_mod, mut inputs, .. } = parse_macro_input!(input as Input);

    if inputs.len() < 3 {
        abort_call_site!("expected at least two input vars and one merge closure")
    }

    let merge = inputs.pop().unwrap();

    let type_idents: Vec<_> = (0..inputs.len()).map(|i| ident!("T{i}")).collect();
    let input_idents: Vec<_> = (0..inputs.len()).map(|i| ident!("var{i}")).collect();
    let idx: Vec<_> = (0..inputs.len())
        .map(|i| syn::Index {
            index: i as _,
            span: proc_macro2::Span::call_site(),
        })
        .collect();

    let mut merge_with = quote! {
        output = Some(#vars_mod::var_merge_output(
            merge(
                #(#input_idents.downcast_ref().unwrap(),)*
            )
        ));
    };
    for (idx, input_ident) in idx.iter().zip(&input_idents).rev() {
        merge_with = quote! {
            #vars_mod::var_merge_with(&inputs[#idx], &mut |#input_ident| {
                #merge_with
            });
        }
    }

    let out = quote! {
        {
            #[inline(always)]
            fn merge_var__<
                #(#type_idents: #vars_mod::VarValue,)*
                O: #vars_mod::VarValue,
                F: FnMut(
                    #(&#type_idents,)*
                ) -> O + Send + 'static
            >(
                #(#input_idents: impl #vars_mod::MergeInput<#type_idents>,)*
                mut merge: F
            ) -> #vars_mod::Var<O> {
                #vars_mod::var_merge(Box::new([
                    #(#vars_mod::var_merge_input(#input_idents),)*
                ]), move |inputs| {
                    let mut output = None;
                    #merge_with
                    output.unwrap()
                })
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
