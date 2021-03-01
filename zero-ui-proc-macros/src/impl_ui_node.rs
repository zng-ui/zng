use crate::util;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashSet;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::*;

pub(crate) fn gen_impl_ui_node(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = match syn::parse::<ItemImpl>(input.clone()) {
        Ok(i) => i,
        Err(e) => {
            // in case of major parsing error, like item not being an impl block. We give the args error
            // but do not remove the function.
            let mut r = proc_macro::TokenStream::from(e.to_compile_error());
            r.extend(input);
            return r;
        }
    };
    // TODO in case of error and input is not an `impl UiNode for`
    // remove #[UiNode] methods and custom lints attributes.
    let args = parse_macro_input!(args as Args);

    let crate_ = util::crate_core();

    let mut ui_node_path = None;

    let ui_node = ident!("UiNode");

    // if the impl block has a trait path.
    if let Some((_, path, _)) = input.trait_ {
        // if the trait name is UiNode
        let seg = path.segments.last().unwrap();
        if seg.ident == ui_node {
            ui_node_path = Some(path);
        }
        // if the impl block is for a trait not named UiNode
        else {
            abort!(
                path.span(),
                "expected inherent impl or `UiNode` trait impl, found `{}`",
                quote! {#path}
            )
        }
    }

    let mut node_items = vec![];
    let mut node_items_missing_del_level = vec![];
    let mut other_items = vec![];
    let mut node_item_names = HashSet::new();

    let mut errors = util::Errors::default();

    // impl scope custom lints:
    //
    // we only have one lint, `zero_ui::missing_delegate`.
    let missing_delegate_level =
        take_missing_deletate_level(&mut input.attrs, &mut errors, &HashSet::new()).unwrap_or(util::LintLevel::Deny);
    let missing_delegate_ident = ident!("missing_delegate");
    let mut forbidden_lints = HashSet::new();
    if let util::LintLevel::Forbid = missing_delegate_level {
        forbidden_lints.insert(&missing_delegate_ident);
    }
    let forbidden_lints = forbidden_lints;

    for mut item in input.items {
        let mut is_node = false;

        if let ImplItem::Method(m) = &mut item {
            // if we are in an UiNode impl
            if ui_node_path.is_some() {
                // assume the item is a method defined in the UiNode trait.
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            }
            // if we are not in an UiNode impl but a method is annotated with `#[UiNode]`.
            else if let Some(index) = m.attrs.iter().position(|a| a.path.get_ident() == Some(&ui_node)) {
                // remove the marker attribute..
                m.attrs.remove(index);
                // ..and assume the item is a method defined in the UiNode trait.
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            }

            if is_node {
                let item_level = take_missing_deletate_level(&mut m.attrs, &mut errors, &forbidden_lints).unwrap_or(missing_delegate_level);
                node_items_missing_del_level.push(item_level);
            }
        }

        if is_node {
            node_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let mut validate_manual_delegate = true;

    // collect default methods needed.
    let default_ui_items = match args {
        Args::NoDelegate => {
            validate_manual_delegate = false;
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

    if validate_manual_delegate {
        let skip = vec![ident!("render"), ident!("render_update"), ident!("boxed")];

        // validate that manually implemented UiNode methods call the expected method in the struct child or children.

        for (manual_impl, level) in node_items.iter().zip(node_items_missing_del_level.into_iter()) {
            let mut validator = DelegateValidator::new(manual_impl);

            if level == util::LintLevel::Allow || skip.contains(&validator.ident) {
                continue;
            }

            validator.visit_impl_item(manual_impl);

            if !validator.delegates {
                let ident = validator.ident;
                errors.push(
                    format_args!(
                        "auto impl delegates call to `{}` but this manual impl does not\n `#[{}(zero_ui::missing_delegate)]` is on",
                        ident, missing_delegate_level
                    ),
                    {
                        match manual_impl {
                            ImplItem::Method(mtd) => mtd.block.span(),
                            _ => non_user_error!("expected a method"),
                        }
                    },
                );
            }
        }
    }

    // if we are not in a `UiNode` impl and no method was tagged `#[UiNode]`. TODO remove this?
    if ui_node_path.is_none() && node_items.is_empty() && !other_items.is_empty() {
        abort_call_site!("no `UiNode` method found, missing `UiNode for` in impl or `#[UiNode]` in methods")
    }

    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let mut inline_all = InlineEverything::new();

    let in_node_impl = ui_node_path.is_some();

    // we use the UiNode path provided by the user if possible
    // to avoid an unused import warning.
    let ui_node_path = ui_node_path
        .map(|p| p.to_token_stream())
        .unwrap_or_else(|| quote! { #crate_::UiNode });

    let mut impl_node = parse_quote! {
        impl #impl_generics #ui_node_path for #self_ty #where_clause {
            #(#node_items)*
            #(#default_ui_items)*
        }
    };
    inline_all.visit_item_impl_mut(&mut impl_node);

    let result = if in_node_impl {
        quote! {
            #errors
            #impl_node
        }
    } else {
        let input_attrs = input.attrs;
        quote! {
            #errors
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

fn no_delegate_absents(crate_: TokenStream, user_mtds: HashSet<Ident>) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn update_hp(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) { }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) { }]

        [fn arrange(&mut self, final_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) { }]

        [fn measure(&mut self, available_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) -> #crate_::units::LayoutSize {
            let mut size = available_size;

            if #crate_::is_layout_any_size(size.width) {
                size.width = 0.0;
            }

            if #crate_::is_layout_any_size(size.height) {
                size.height = 0.0;
            }

            size
        }]
    }
}

fn delegate_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    let borrow = quote_spanned! {borrow.span()=>
        #crate_::ui_node_asserts::delegate(#borrow)
    };
    let borrow_mut = quote_spanned! {borrow_mut.span()=>
        #crate_::ui_node_asserts::delegate_mut(#borrow_mut)
    };
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let child = {#borrow_mut};
            child.init(ctx)
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let child = {#borrow_mut};
            child.deinit(ctx)
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let child = {#borrow_mut};
            child.update(ctx)
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let child = {#borrow_mut};
            child.update_hp(ctx)
        }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) {
            let child = {#borrow};
            child.render(frame)
        }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) {
            let child = {#borrow};
            child.render_update(update)
        }]

        [fn arrange(&mut self, final_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) {
            let child = {#borrow_mut};
            child.arrange(final_size, ctx)
        }]

        [fn measure(&mut self, available_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) -> #crate_::units::LayoutSize {
            let child = {#borrow_mut};
            child.measure(available_size, ctx)
        }]
    }
}

fn delegate_list_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    let borrow = quote_spanned! {borrow.span()=>
        #crate_::ui_node_asserts::delegate_list(#borrow)
    };
    let borrow_mut = quote_spanned! {borrow_mut.span()=>
        #crate_::ui_node_asserts::delegate_list_mut(#borrow_mut)
    };
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::UiNodeList::init_all(children, ctx)
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::UiNodeList::deinit_all(children, ctx)
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::UiNodeList::update_all(children, ctx)
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let children = {#borrow_mut};
            #crate_::UiNodeList::update_hp_all(children, ctx)
        }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) {
            let children = {#borrow};
            #crate_::UiNodeList::render_all(children, |_|#crate_::units::LayoutPoint::zero(), frame)
        }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) {
            let children = {#borrow};
            #crate_::UiNodeList::render_update_all(children, update)
        }]

        [fn arrange(&mut self, final_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) {
            let children = {#borrow_mut};
            #crate_::UiNodeList::arrange_all(children, |_, _|final_size, ctx)
        }]

        [fn measure(&mut self, available_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) -> #crate_::units::LayoutSize {
            let children = {#borrow_mut};
            let mut size = #crate_::units::LayoutSize::zero();
            #crate_::UiNodeList::measure_all(children, |_, _|available_size, |_, desired_size, _| {
                size = size.max(desired_size);
            }, ctx);
            size
        }]
    }
}

fn delegate_iter_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, iter: Expr, iter_mut: Expr) -> Vec<ImplItem> {
    make_absents! { user_mtds

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            for child in {#iter_mut} {
                child.init(ctx)
            }
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            for child in {#iter_mut} {
                child.deinit(ctx)
            }
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            for child in {#iter_mut} {
                child.update(ctx)
            }
        }]

        [fn update_hp(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            for child in {#iter_mut} {
                child.update_hp(ctx)
            }
        }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) {
            for child in {#iter} {
                child.render(frame)
            }
        }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) {
            for child in {#iter} {
                child.render_update(update)
            }
        }]

        [fn arrange(&mut self, final_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) {
            for child in {#iter_mut} {
                child.arrange(final_size, ctx)
            }
        }]

        [fn measure(&mut self, available_size: #crate_::units::LayoutSize, ctx: &mut #crate_::context::LayoutContext) -> #crate_::units::LayoutSize {
            let mut size = #crate_::units::LayoutSize::zero();
            for child in {#iter_mut} {
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
                    delegate: parse_quote_spanned!(arg0.span()=> &self.child),
                    delegate_mut: parse_quote_spanned!(arg0.span()=> &mut self.child),
                }
            } else if arg0 == ident!("children") {
                Args::DelegateList {
                    delegate_list: parse_quote_spanned!(arg0.span()=> &self.children),
                    delegate_list_mut: parse_quote_spanned!(arg0.span()=> &mut self.children),
                }
            } else if arg0 == ident!("children_iter") {
                Args::DelegateIter {
                    delegate_iter: parse_quote_spanned!(arg0.span()=> self.children.iter()),
                    delegate_iter_mut: parse_quote_spanned!(arg0.span()=> self.children.iter_mut()),
                }
            } else if arg0 == ident!("none") {
                Args::NoDelegate
            } else {
                let delegate = ident!("delegate");
                let delegate_mut = ident!("delegate_mut");

                if arg0 == delegate || arg0 == delegate_mut {
                    let (delegate, delegate_mut) = parse_delegate_pair(args, arg0, delegate, delegate_mut)?;
                    Args::Delegate { delegate, delegate_mut }
                } else {
                    let delegate_iter = ident!("delegate_iter");
                    let delegate_iter_mut = ident!("delegate_iter_mut");

                    if arg0 == delegate_iter || arg0 == delegate_iter_mut {
                        let (delegate_iter, delegate_iter_mut) = parse_delegate_pair(args, arg0, delegate_iter, delegate_iter_mut)?;
                        Args::DelegateIter {
                            delegate_iter,
                            delegate_iter_mut,
                        }
                    } else {
                        let delegate_list = ident!("delegate_list");
                        let delegate_list_mut = ident!("delegate_list_mut");

                        if arg0 == delegate_list || arg0 == delegate_list_mut {
                            let (delegate_list, delegate_list_mut) = parse_delegate_pair(args, arg0, delegate_list, delegate_list_mut)?;
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

/// After parsing one of the delegate idents, parse the value and the other delegate.
///
/// Returns (immutable_expr, mutable_expr) independently of the order the delegates where written.
fn parse_delegate_pair(args: ParseStream, arg0: Ident, ident: Ident, ident_mut: Ident) -> Result<(Expr, Expr)> {
    // parse arg0 ": <expr>"
    let expr0 = parse_delegate_expr(args, &arg0)?;

    // get what ident is the second one, delegate pairs can be defined in any order.
    let expected_arg1 = if arg0 == ident { &ident_mut } else { &ident };

    // delegate pair are separated by comma (,)
    let comma = args
        .parse::<Token![,]>()
        .map_err(|_| Error::new(util::after_span(&expr0), format!("expected `, {}: <expr>`", expected_arg1)))?;
    // delegate idents require a pair.
    let arg1: Ident = args
        .parse()
        .map_err(|_| Error::new(comma.span(), format!("expected `{}: <expr>`", expected_arg1)))?;

    // second ident is not the expected pair.
    if &arg1 != expected_arg1 {
        return Err(Error::new(arg1.span(), format!("expected `{}`", ident_mut)));
    }

    // parse arg1 ": <expr>"
    let expr1 = parse_delegate_expr(args, &arg1)?;

    // result is (immutable_expr, mutable_expr)
    if arg0 == ident {
        Ok((expr0, expr1))
    } else {
        Ok((expr1, expr0))
    }
}
fn parse_delegate_expr(args: ParseStream, ident: &Ident) -> Result<Expr> {
    let colon = args
        .parse::<Token![:]>()
        .map_err(|_| Error::new(ident.span(), format!("expected `{}: <expr>`", ident)))?;
    let expr: Expr = args
        .parse()
        .map_err(|_| Error::new(colon.span(), format!("expected `{}: <expr>`", ident)))?;

    Ok(expr)
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

/// Removes and returns the `zero_ui::missing_delegate` level.
fn take_missing_deletate_level(
    attrs: &mut Vec<Attribute>,
    errors: &mut util::Errors,
    forbidden: &HashSet<&Ident>,
) -> Option<util::LintLevel> {
    let mut r = None;
    for (lint_ident, level, _) in util::take_zero_ui_lints(attrs, errors, forbidden) {
        if lint_ident == "missing_delegate" {
            r = Some(level);
        }
    }
    r
}
