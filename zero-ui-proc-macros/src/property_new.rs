use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Brace,
    FieldValue, Ident, Token,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}

struct Input {
    property_data: PropertyData,
    user_input: UserInput,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

struct PropertyData {
    member_idents: Vec<Ident>,
}

struct UserInput {
    brace_token: Brace,
    fields: Punctuated<FieldValue, Token![,]>,
}
