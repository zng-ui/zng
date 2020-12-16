use crate::util;
use proc_macro2::Span;
use std::collections::HashSet;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::*;

pub(crate) fn gen_impl_ui_node(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as Args);
    let mut input = parse_macro_input!(input as ItemImpl);

    let crate_ = util::zero_ui_crate_ident();

    let mut in_node_impl = false;

    let ui_node = ident!("UiNode");

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
    let mut node_items_allow_missing_del = vec![];
    let mut other_items = vec![];
    let mut node_item_names = HashSet::new();

    let mut validate_manual_del = take_allow_missing_deletate(&mut input.attrs).is_none();

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

            if is_node && validate_manual_del {
                node_items_allow_missing_del.push(take_allow_missing_deletate(&mut m.attrs).is_some());
            }
        }

        if is_node {
            node_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let default_ui_items = match args {
        Args::NoDelegate => {
            validate_manual_del = false;
            no_delegate_absents(crate_.clone(), node_item_names)
        }
        Args::Delegate { delegate, delegate_mut } => delegate_absents(crate_.clone(), node_item_names, delegate, delegate_mut),
        Args::DelegateList {
            delegate_list,
            delegate_list_mut,
        } => delegate_list_absents(crate_.clone(), node_item_names, delegate_list, delegate_list_mut),
        Args::DelegateIter {
            delegate_iter,
            delegate_iter_mut,
        } => delegate_iter_absents(crate_.clone(), node_item_names, delegate_iter, delegate_iter_mut),
    };

    if validate_manual_del {
        let skip = vec![ident!("render"), ident!("render_update"), ident!("boxed")];

        for (manual_impl, allow) in node_items.iter().zip(node_items_allow_missing_del.into_iter()) {
            let mut validator = DelegateValidator::new(manual_impl);

            if allow || skip.contains(&validator.ident) {
                continue;
            }

            validator.visit_impl_item(manual_impl);

            if !validator.delegates {
                let ident = validator.ident;
                abort!(
                    manual_impl.span(),
                    "auto impl delegates call to `{}` but this manual impl does not",
                    quote! {#ident},
                )
            }
        }
    }

    if !in_node_impl && node_items.is_empty() && !other_items.is_empty() {
        abort_call_site!("no UiNode method found, missing `UiNode for` in impl or `#[UiNode]` in methods")
    }

    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let mut inline_all = InlineEverything::new();

    let mut impl_node = parse_quote! {
        impl #impl_generics #crate_::core::UiNode for #self_ty #where_clause {
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
        if i.attrs.iter().all(|a| a.path.get_ident() != self.inline.path.get_ident()) {
            i.attrs.push(self.inline.clone());
        }

        visit_mut::visit_impl_item_method_mut(self, i);
    }
}

macro_rules! make_absents {
    ($user_mtds:ident $([fn $ident:ident $($tt:tt)*])+) => {{
        let mut absents = vec![];
        let user_mtds = $user_mtds;
        $(
        if !user_mtds.contains(&ident!(stringify!($ident))) {
            absents.push(parse_quote!{
               fn $ident $($tt)*
            });
        }
        )+
        absents
    }};
}

fn no_delegate_absents(crate_: Ident, user_mtds: HashSet<Ident>) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::core::context::WidgetContext) { }]

        [fn deinit(&mut self, ctx: &mut #crate_::core::context::WidgetContext) { }]

        [fn update(&mut self, ctx: &mut #crate_::core::context::WidgetContext) { }]

        [fn update_hp(&mut self, ctx: &mut #crate_::core::context::WidgetContext) { }]

        [fn render(&self, frame: &mut #crate_::core::render::FrameBuilder) { }]

        [fn render_update(&self, update: &mut #crate_::core::render::FrameUpdate) { }]

        [fn arrange(&mut self, final_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) { }]

        [fn measure(&mut self, available_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) -> #crate_::core::units::LayoutSize {
            let mut size = available_size;

            if #crate_::core::is_layout_any_size(size.width) {
                size.width = 0.0;
            }

            if #crate_::core::is_layout_any_size(size.height) {
                size.height = 0.0;
            }

            size
        }]
    }
}

fn delegate_absents(crate_: Ident, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let child = {#borrow_mut};
            child.init(ctx)
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let child = {#borrow_mut};
            child.deinit(ctx)
        }]

        [fn update(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let child = {#borrow_mut};
            child.update(ctx)
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let child = {#borrow_mut};
            child.update_hp(ctx)
        }]

        [fn render(&self, frame: &mut #crate_::core::render::FrameBuilder) {
            let child = {#borrow};
            child.render(frame)
        }]

        [fn render_update(&self, update: &mut #crate_::core::render::FrameUpdate) {
            let child = {#borrow};
            child.render_update(update)
        }]

        [fn arrange(&mut self, final_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) {
            let child = {#borrow_mut};
            child.arrange(final_size, ctx)
        }]

        [fn measure(&mut self, available_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) -> #crate_::core::units::LayoutSize {
            let child = {#borrow_mut};
            child.measure(available_size, ctx)
        }]
    }
}

fn delegate_list_absents(crate_: Ident, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::core::UiNodeList::init_all(children, ctx)
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::core::UiNodeList::deinit_all(children, ctx)
        }]

        [fn update(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::core::UiNodeList::update_all(children, ctx)
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::core::UiNodeList::update_hp_all(children, ctx)
        }]

        [fn render(&self, frame: &mut #crate_::core::render::FrameBuilder) {
            let children = {#borrow};
            #crate_::core::UiNodeList::render_all(children, |_|#crate_::core::units::LayoutPoint::zero(), frame)
        }]

        [fn render_update(&self, update: &mut #crate_::core::render::FrameUpdate) {
            let children = {#borrow};
            #crate_::core::UiNodeList::render_update_all(children, update)
        }]

        [fn arrange(&mut self, final_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) {
            let children = {#borrow_mut};
            #crate_::core::UiNodeList::arrange_all(children, |_, _|final_size, ctx)
        }]

        [fn measure(&mut self, available_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) -> #crate_::core::units::LayoutSize {
            let children = {#borrow_mut};
            let mut size = #crate_::core::units::LayoutSize::zero();
            #crate_::core::UiNodeList::measure_all(children, |_, _|available_size, |_, desired_size, _| {
                size = size.max(desired_size);
            }, ctx);
            size
        }]
    }
}

fn delegate_iter_absents(crate_: Ident, user_mtds: HashSet<Ident>, iter: Expr, iter_mut: Expr) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            for child in {#iter_mut} {
                child.init(ctx)
            }
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            for child in {#iter_mut} {
                child.deinit(ctx)
            }
        }]

        [fn update(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            for child in {#iter_mut} {
                child.update(ctx)
            }
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::core::context::WidgetContext) {
            for child in {#iter_mut} {
                child.update_hp(ctx)
            }
        }]

        [fn render(&self, frame: &mut #crate_::core::render::FrameBuilder) {
            for child in {#iter} {
                child.render(frame)
            }
        }]

        [fn render_update(&self, update: &mut #crate_::core::render::FrameUpdate) {
            for child in {#iter} {
                child.render_update(update)
            }
        }]

        [fn arrange(&mut self, final_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) {
            for child in {#iter_mut} {
                child.arrange(final_size, ctx)
            }
        }]

        [fn measure(&mut self, available_size: #crate_::core::units::LayoutSize, ctx: &mut #crate_::core::context::LayoutContext) -> #crate_::core::units::LayoutSize {
            let mut size = #crate_::core::units::LayoutSize::zero();
            for child in #iter_mut {
                size = child.measure(available_size, ctx).max(size);
            }
            size
        }]
    }
}
/// Parsed macro arguments.
#[allow(clippy::large_enum_variant)]
enum Args {
    /// No arguments. Impl is for a leaf in the Ui tree.
    NoDelegate,
    /// `child` or `delegate=expr` and `delegate_mut=expr`. Impl is for
    /// an Ui that delegates each call to a single delegate.
    Delegate { delegate: Expr, delegate_mut: Expr },
    /// `children` or `delegate_list=expr` and `delegate_list_mut=expr`. Impl
    /// is for an Ui that delegates each call to multiple delegates.
    DelegateList { delegate_list: Expr, delegate_list_mut: Expr },
    /// `children_iter` or `delegate_iter=expr` and `delegate_iter_mut=expr`. Impl
    /// is for an Ui that delegates each call to multiple delegates.
    DelegateIter { delegate_iter: Expr, delegate_iter_mut: Expr },
}

impl Parse for Args {
    fn parse(args: ParseStream) -> Result<Self> {
        if args.peek(Ident) {
            let arg0 = args.parse::<Ident>()?;

            let args = if arg0 == ident!("child") {
                Args::Delegate {
                    delegate: parse_quote!(&self.child),
                    delegate_mut: parse_quote!(&mut self.child),
                }
            } else if arg0 == ident!("children") {
                Args::DelegateList {
                    delegate_list: parse_quote!(&self.children),
                    delegate_list_mut: parse_quote!(&mut self.children),
                }
            } else if arg0 == ident!("children_iter") {
                Args::DelegateIter {
                    delegate_iter: parse_quote!(self.children.iter()),
                    delegate_iter_mut: parse_quote!(self.children.iter_mut()),
                }
            } else if arg0 == ident!("none") {
                Args::NoDelegate
            } else {
                let delegate = ident!("delegate");
                let delegate_mut = ident!("delegate_mut");

                if arg0 == delegate || arg0 == delegate_mut {
                    let (delegate, delegate_mut) = parse_pair(args, arg0, delegate, delegate_mut)?;
                    Args::Delegate { delegate, delegate_mut }
                } else {
                    let delegate_iter = ident!("delegate_iter");
                    let delegate_iter_mut = ident!("delegate_iter_mut");

                    if arg0 == delegate_iter || arg0 == delegate_iter_mut {
                        let (delegate_iter, delegate_iter_mut) = parse_pair(args, arg0, delegate_iter, delegate_iter_mut)?;
                        Args::DelegateIter {
                            delegate_iter,
                            delegate_iter_mut,
                        }
                    } else {
                        let delegate_list = ident!("delegate_list");
                        let delegate_list_mut = ident!("delegate_list_mut");

                        if arg0 == delegate_list || arg0 == delegate_list_mut {
                            let (delegate_list, delegate_list_mut) = parse_pair(args, arg0, delegate_list, delegate_list_mut)?;
                            Args::DelegateList {
                                delegate_list,
                                delegate_list_mut,
                            }
                        } else {
                            return Err(Error::new(
                                arg0.span(),
                                "expected `child`, `children`, `children_iter`, `delegate`, `delegate_list` or `delegate_iter`",
                            ));
                        }
                    }
                }
            };

            Ok(args)
        } else {
            Err(Error::new(
                Span::call_site(),
                "missing macro argument, expected `none`, `child`, `children`, `children_iter`, `delegate`, `delegate_list` or `delegate_iter`",
            ))
        }
    }
}

fn parse_pair(args: ParseStream, arg0: Ident, ident: Ident, ident_mut: Ident) -> Result<(Expr, Expr)> {
    args.parse::<Token![:]>()?;
    let expr1: Expr = args.parse()?;
    args.parse::<Token![,]>()?;

    let ident2: Ident = args.parse()?;

    args.parse::<Token![:]>()?;
    let expr2: Expr = args.parse()?;
    if args.peek(Token![,]) {
        args.parse::<Token![,]>()?;
    }

    if arg0 == ident {
        if ident2 == ident_mut {
            Ok((expr1, expr2))
        } else {
            Err(Error::new(ident2.span(), format!("expected `{}`", ident_mut)))
        }
    } else {
        debug_assert_eq!(arg0, ident_mut);
        if ident2 == ident {
            Ok((expr2, expr1))
        } else {
            Err(Error::new(ident2.span(), format!("expected `{}`", ident)))
        }
    }
}

struct DelegateValidator<'a> {
    pub ident: &'a Ident,
    pub list_variant: Ident,
    pub attrs: &'a [Attribute],
    args_count: u8,
    pub delegates: bool,
}

impl<'a> DelegateValidator<'a> {
    fn new(manual_impl: &'a ImplItem) -> Self {
        if let ImplItem::Method(m) = manual_impl {
            DelegateValidator {
                ident: &m.sig.ident,
                list_variant: ident!("{}_all", m.sig.ident),
                attrs: &m.attrs,
                args_count: (m.sig.inputs.len() - 1) as u8,
                delegates: false,
            }
        } else {
            panic!("")
        }
    }
}

impl<'a, 'ast> Visit<'ast> for DelegateValidator<'a> {
    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        if (&i.method == self.ident && i.args.len() as u8 == self.args_count) || i.method == self.list_variant {
            self.delegates = true;
        }
        visit::visit_expr_method_call(self, i)
    }
}

fn take_allow_missing_deletate(attrs: &mut Vec<Attribute>) -> Option<Attribute> {
    let allow = ident!("allow_missing_delegate");

    if let Some(i) = attrs.iter().position(|a| a.path.get_ident() == Some(&allow)) {
        Some(attrs.remove(i))
    } else {
        None
    }
}
