use std::collections::{HashMap, HashSet};

use crate::util::{parse_all, Errors};
use proc_macro2::TokenStream;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
    FieldValue, Ident, Member, Path, Token,
};

// Validate and expand the named fields property assign syntax.
//
// # Why Not Use `struct` Init?
//
// We could use `macro_rules!` to expand to to a `property::ArgsImpl { $($tt)* }` and let
// rust sort and validate the fields. Unfortunately when ArgsImpl has generic fields the `rustc`
// error highlights the `ArgsImpl` ident instead of the value that is causing the error. Rust also
// initializes the invalid generic type and continue given errors in every usage of the args instance
// that asserts the generic bounds, this can very easy cause a `Span::call_site()` error with a cryptic message.
//
// The downside is that we need to reimplement the other errors like missing fields here too.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { property_data, user_input } = parse_macro_input!(input as Input);

    let path = property_data.property_path;
    let args_ident_from_wgt = property_data.args_impl_spanned;

    let mut errors = Errors::default();
    let mut args = vec![None; property_data.arg_idents.len()];

    let members: HashMap<_, _> = property_data.arg_idents.iter().enumerate().map(|(v, k)| (k, v)).collect();
    let mut already_set = HashSet::new();

    for field in user_input.fields {
        match field.member {
            Member::Named(ident) => {
                if !already_set.insert(ident.clone()) {
                    errors.push(format!("field `{}` already set", ident), ident.span());
                } else if let Some(i) = members.get(&ident) {
                    args[*i] = Some(field.expr);
                } else {
                    errors.push(format!("unknown field `{}`", ident), ident.span());
                }
            }
            Member::Unnamed(n) => {
                errors.push("expected identifier", n.span());
            }
        }
    }
    let mut missing_fields = String::new();
    let mut missing_count = 0;

    for (i, a) in args.iter_mut().enumerate() {
        if a.is_none() {
            use std::fmt::Write;
            missing_count += 1;
            write!(missing_fields, "`{}`, ", property_data.arg_idents[i]).unwrap();
            *a = Some(parse_quote!(std::unreachable!()));
        }
    }

    if !missing_fields.is_empty() {
        let missing_fields = missing_fields.trim_end_matches(", ");
        let span = user_input.brace_token.span;
        if missing_count == 1 {
            errors.push(format_args!("missing field {} in property initializer", missing_fields), span);
        } else {
            errors.push(format_args!("missing fields {} in property initializer", missing_fields), span);
        }
    }

    let allow_unreachable = if missing_fields.is_empty() {
        TokenStream::default()
    } else {
        quote_spanned! {args_ident_from_wgt.span()=>  #[allow(unreachable_code)] }
    };

    let r = quote_spanned! {args_ident_from_wgt.span()=>
        #allow_unreachable {
            #errors
            use #path::ArgsImpl as #args_ident_from_wgt;
            #args_ident_from_wgt::new(#(#args),*)
        }
    };
    r.into()
}

struct Input {
    property_data: PropertyData,
    user_input: UserInput,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            property_data: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            user_input: input.parse()?,
        })
    }
}

struct PropertyData {
    property_path: Path,
    args_impl_spanned: Ident,
    arg_idents: Vec<Ident>,
}
impl Parse for PropertyData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(PropertyData {
            property_path: non_user_braced!(input, "property_path")
                .parse()
                .unwrap_or_else(|e| non_user_error!(e)),
            args_impl_spanned: non_user_braced!(input, "args_impl_spanned")
                .parse()
                .unwrap_or_else(|e| non_user_error!(e)),
            arg_idents: parse_all(&non_user_braced!(input, "arg_idents")).unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

struct UserInput {
    brace_token: Brace,
    fields: Punctuated<FieldValue, Token![,]>,
}
impl Parse for UserInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        let brace_token = braced!(inner in input);
        let fields = Punctuated::parse_terminated(&inner)?;
        Ok(UserInput { brace_token, fields })
    }
}
