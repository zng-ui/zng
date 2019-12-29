use proc_macro2::{Span, TokenStream};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::*;

include!("util.rs");

pub(crate) fn gen_impl_ui_node(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
    crate_: TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemImpl);

    let mut in_node_impl = false;

    let ui_node = ident("UiNode");

    if let Some((_, path, _)) = input.trait_ {
        if let Some(seg) = path.segments.last() {
            in_node_impl = seg.ident == ui_node;
        }
        if !in_node_impl {
            abort!(
                path.span(),
                "expected inherent impl or UiNode trait impl, found `{}`",
                quote! {#path}
            )
        }
    }

    let mut node_items = vec![];
    let mut other_items = vec![];
    let mut node_item_names = HashSet::new();

    for mut item in input.items {
        let mut is_node = false;

        if let ImplItem::Method(m) = &mut item {
            if in_node_impl {
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            } else if let Some(index) = m.attrs.iter().position(|a| a.path.get_ident() == Some(&ui_node)) {
                m.attrs.remove(index);
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            }
        }

        if is_node {
            node_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let default_ui_items = match args {
        Args::Leaf => leaf_defaults(crate_.clone(), node_item_names),
        Args::Container { delegate, delegate_mut } => {
            container_defaults(crate_.clone(), node_item_names, delegate, delegate_mut)
        }
        Args::MultiContainer {
            delegate_iter,
            delegate_iter_mut,
        } => multi_container_defaults(crate_.clone(), node_item_names, delegate_iter, delegate_iter_mut),
    };

    
    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;
    
    let mut inline_all = InlineEverything::new();
    
    let mut impl_node = parse_quote! {
        impl #impl_generics #crate_::core2::UiNode for #self_ty #where_clause {
            #(#node_items)*
            #(#default_ui_items)*
        }
    };
    inline_all.visit_item_impl_mut(&mut impl_node);
    
    let result = if in_node_impl {
        quote! {
            #impl_node
        }
    } else {
        let input_attrs = input.attrs;
        quote! {
            #(#input_attrs)*
            impl #impl_generics #self_ty #where_clause {
                #(#other_items)*
            }

            #impl_node
        }
    };

    //let test = format!("{}", result);
    //if test.contains("FocusOnInit") {
    //    println!("{}", test);
    //}

    result.into()
}

/// Visitor that adds `#[inline]` in every `ImplItemMethod`.
struct InlineEverything {
    inline: Attribute,
}
impl InlineEverything {
    pub fn new() -> Self {
        InlineEverything {
            inline: parse_quote! {#[inline]},
        }
    }
}
impl VisitMut for InlineEverything {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        if i.attrs
            .iter()
            .all(|a| a.path.get_ident() != self.inline.path.get_ident())
        {
            i.attrs.push(self.inline.clone());
        }

        visit_mut::visit_impl_item_method_mut(self, i);
    }
}

fn leaf_defaults(crate_: TokenStream, user_mtds: HashSet<Ident>) -> Vec<ImplItem> {
    let mut absents = vec![];

    macro_rules! push_absent {
        (fn $ident:ident $($tt:tt)*) => {
            if !user_mtds.contains(&ident(stringify!($ident))) {
                absents.push(parse_quote!{
                   fn $($tt)*
                });
            }
        };
    }
    push_absent! {fn init(&mut self, ctx: &mut #crate_::core2::AppContext) {} }
    push_absent! {fn deinit(&mut self, ctx: &mut #crate_::core2::AppContext) {} }
    push_absent! {fn update(&mut self, ctx: &mut #crate_::core2::AppContext) {} }
    push_absent! {fn update_hp(&mut self, ctx: &mut #crate_::core2::AppContext) {} }
    push_absent! {fn arrange(&mut self, final_size: #crate_::core2::LayoutSize) {} }
    push_absent! {fn render(&self, f: &mut #crate_::core2::NextFrame) {} }
    push_absent! {fn measure(&mut self, available_size: #crate_::core2::LayoutSize) -> #crate_::core2::LayoutSize {
        let mut size = available_size;
        
        if size.width.is_infinite() {
            size.width = 0.0;
        }
        
        if size.height.is_infinite() {
            size.height = 0.0;
        }
        
        size
    }}

    absents
}

fn container_defaults(crate_: TokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    let mut absents = vec![];

    macro_rules! push_absent {
        (fn $ident:ident $($tt:tt)*) => {
            if !user_mtds.contains(&ident(stringify!($ident))) {
                absents.push(parse_quote!{
                   fn $ident $($tt)*
                });
            }
        };
    }
    push_absent! {fn init(&mut self, ctx: &mut #crate_::core2::AppContext) {
        let child = #borrow_mut;
        child.init(ctx)
    }}
    push_absent! {fn deinit(&mut self, ctx: &mut #crate_::core2::AppContext) {
        let child = #borrow_mut;
        child.deinit(ctx)
    }}
    push_absent! {fn update(&mut self, ctx: &mut #crate_::core2::AppContext) {
        let child = #borrow_mut;
        child.update(ctx)
    }}
    push_absent! {fn update_hp(&mut self, ctx: &mut #crate_::core2::AppContext) {
        let child = #borrow_mut;
        child.update_hp(ctx)
    }}
    push_absent! {fn arrange(&mut self, final_size: #crate_::core2::LayoutSize) {
        let child = #borrow_mut;
        child.arrange(final_size)
    }}
    push_absent! {fn render(&self, f: &mut #crate_::core2::NextFrame) {
        let child = #borrow;
        child.render(f)
    }}
    push_absent! {fn measure(&mut self, available_size: #crate_::core2::LayoutSize) -> #crate_::core2::LayoutSize {
        let child = #borrow_mut;            
        child.measure(available_size)
    }}

    absents
}

fn multi_container_defaults(
    crate_: TokenStream,
    user_mtds: HashSet<Ident>,
    iter: Expr,
    iter_mut: Expr,
) -> Vec<ImplItem> {
    let mut absents = vec![];

    macro_rules! push_absent {
        (fn $ident:ident $($tt:tt)*) => {
            if !user_mtds.contains(&ident(stringify!($ident))) {
                absents.push(parse_quote!{
                   fn $($tt)*
                });
            }
        };
    }
    push_absent! {fn init(&mut self, ctx: &mut #crate_::core2::AppContext) {
        for child in #iter_mut {            
            child.init(ctx)
        }
    }}
    push_absent! {fn deinit(&mut self, ctx: &mut #crate_::core2::AppContext) {
        for child in #iter_mut {            
            child.deinit(ctx)
        }
    }}
    push_absent! {fn update(&mut self, ctx: &mut #crate_::core2::AppContext) {
        for child in #iter_mut {
            child.update(ctx)
        }
    }}
    push_absent! {fn update_hp(&mut self, ctx: &mut #crate_::core2::AppContext) {
        for child in #iter_mut {
            child.update_hp(ctx)
        }
    }}
    push_absent! {fn arrange(&mut self, final_size: #crate_::core2::LayoutSize) {
        for child in #iter_mut {
            child.arrange(final_size)
        }
    }}
    push_absent! {fn render(&self, f: &mut #crate_::core2::NextFrame) {
        for child in #iter {
            child.render(f)
        }
    }}
    push_absent! {fn measure(&mut self, available_size: #crate_::core2::LayoutSize) -> #crate_::core2::LayoutSize {
        let mut size = Default::default();
        for child in #iter_mut {             
            size = child.measure(available_size).max(size);
        }
        size
    }}

    absents
}
/// Parsed macro arguments.
#[allow(clippy::large_enum_variant)]
enum Args {
    /// No arguments. Impl is for a leaf in the Ui tree.
    Leaf,
    /// `child` or `delegate=expr` and `delegate_mut=expr`. Impl is for
    /// an Ui that delegates each call to a single delegate.
    Container { delegate: Expr, delegate_mut: Expr },
    /// `children` or `delegate_iter=expr` and `delegate_iter_mut=expr`. Impl
    /// is for an Ui that delegates each call to multiple delegates.
    MultiContainer {
        delegate_iter: Expr,
        delegate_iter_mut: Expr,
    },
}

impl Parse for Args {
    fn parse(args: ParseStream) -> Result<Self> {
        let args = if args.is_empty() {
            Args::Leaf
        } else {
            let arg0 = args.parse::<Ident>()?;

            if arg0 == ident("child") {
                Args::Container {
                    delegate: parse_quote!(&self.child),
                    delegate_mut: parse_quote!(&mut self.child),
                }
            } else if arg0 == ident("children") {
                Args::MultiContainer {
                    delegate_iter: parse_quote!(self.children.iter()),
                    delegate_iter_mut: parse_quote!(self.children.iter_mut()),
                }
            } else if arg0 == ident("delegate") {
                // https://docs.rs/syn/1.0.5/syn/struct.ExprAssign.html
                args.parse::<Token![:]>()?;

                let delegate = args.parse::<Expr>()?;

                args.parse::<Token![,]>()?;

                let delegate_mut = args.parse::<Ident>()?;
                if delegate_mut != ident("delegate_mut") {
                    return Err(syn::parse::Error::new(delegate_mut.span(), "expected `delegate_mut`"));
                }

                args.parse::<Token![:]>()?;

                let delegate_mut = args.parse::<Expr>()?;

                Args::Container { delegate, delegate_mut }
            } else if arg0 == ident("delegate_iter") {
                args.parse::<Token![:]>()?;

                let delegate_iter = args.parse::<Expr>()?;

                args.parse::<Token![,]>()?;

                let delegate_iter_mut = args.parse::<Ident>()?;
                if delegate_iter_mut != ident("delegate_iter_mut") {
                    return Err(syn::parse::Error::new(
                        delegate_iter_mut.span(),
                        "expected `delegate_iter_mut`",
                    ));
                }

                args.parse::<Token![:]>()?;

                let delegate_iter_mut = args.parse::<Expr>()?;

                Args::MultiContainer {
                    delegate_iter,
                    delegate_iter_mut,
                }
            } else {
                return Err(syn::parse::Error::new(
                    arg0.span(),
                    "expected `child`, `children`, `delegate` or `delegate_iter`",
                ));
            }
        };

        Ok(args)
    }
}
