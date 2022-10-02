use syn::{parse::*, *};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { stmts } = parse_macro_input!(input as Input);
    todo!("{stmts:#?}")
}

struct Input {
    pub stmts: Vec<Stmt>,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            stmts: Block::parse_within(input)?
        })
    }
}