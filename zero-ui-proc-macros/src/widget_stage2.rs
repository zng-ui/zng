use proc_macro2::TokenStream;
use syn::{parse::*, punctuated::Punctuated, *};

/// `widget!` recursive inheritance.
///
/// ## In Stage 2:
///
/// 1 - Parse the inherits yet to include + the stage3 return macro.
/// 2 - If there still inherits to include, generate a call to the right-most
///     inherit left in the stack, this call is *recursive* to **Stage 2**.
/// 3 - Else if all inherits are included, call the stage3 return macro, this
///     call is the start of **Stage 3**.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // 1
    let WidgetInheriting {
        stage3_name,
        mut inherits,
        rest,
    } = parse_macro_input!(input as WidgetInheriting);

    let new_widget_span = stage3_name.span();

    if inherits.is_empty() {
        // 3 - go to widget_stage3.
        let r = quote_spanned! {new_widget_span=>
            #stage3_name! {
                #rest
            }
        };
        r.into()
    } else {
        // 2 - recursive to widget_stage2 again.
        let next_inherit = inherits.pop().unwrap().into_value();
        let r = quote_spanned! {new_widget_span=>
            #next_inherit! {
                -> inherit {
                    #stage3_name;
                    #next_inherit;
                    #next_inherit::widget_stage2;
                    #inherits
                }
                #rest
            }
        };
        r.into()
    }
}

struct WidgetInheriting {
    stage3_name: Ident,
    inherits: Punctuated<Path, Token![+]>,
    rest: TokenStream,
}
impl Parse for WidgetInheriting {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![=>]>().unwrap_or_else(|e| non_user_error!(e));

        let inner = non_user_braced!(input);

        let stage3_name = inner.parse().unwrap_or_else(|e| non_user_error!(e));
        inner.parse::<Token![;]>().unwrap_or_else(|e| non_user_error!(e));

        let inherits = Punctuated::parse_terminated(&inner).unwrap_or_else(|e| non_user_error!(e));

        let rest = input.parse().unwrap_or_else(|e| non_user_error!(e));
        Ok(WidgetInheriting {
            stage3_name,
            inherits,
            rest,
        })
    }
}
