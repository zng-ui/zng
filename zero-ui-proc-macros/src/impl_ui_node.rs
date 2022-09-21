use crate::util;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashSet;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
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

    // validate layout/measure pair
    if let Some(layout) = &node_item_names.get(&ident!("layout")) {
        if !node_item_names.contains(&ident!("measure")) {
            errors.push(
                "`layout` is manual impl, but `measure` is auto impl, both must be auto or manual",
                layout.span(),
            );
        }
    } else if let Some(measure) = &node_item_names.get(&ident!("measure")) {
        if !node_item_names.contains(&ident!("measure")) {
            errors.push(
                "`measure` is manual impl, but `layout` is auto impl, both must be auto or manual",
                measure.span(),
            );
        }
    }

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
        let skip = vec![ident!("boxed"), ident!("measure")];

        // validate that manually implemented UiNode methods call the expected method in the struct child or children.

        for (manual_impl, level) in node_items.iter().zip(node_items_missing_del_level.into_iter()) {
            let mut validator = DelegateValidator::new(manual_impl);

            if level == util::LintLevel::Allow || skip.contains(validator.ident) {
                continue;
            }

            validator.visit_impl_item(manual_impl);

            if !validator.delegates {
                let ident = validator.ident;
                errors.push(
                    format_args!(
                        "auto impl delegates call to `{ident}`, but this manual impl does not\n `#[{missing_delegate_level}(zero_ui::missing_delegate)]` is on",
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

    // if we are not in a `UiNode` impl and no method was tagged `#[UiNode]`.
    if ui_node_path.is_none() && node_items.is_empty() && !other_items.is_empty() {
        abort_call_site!("no `UiNode` method found, missing `UiNode for` in impl or `#[UiNode]` in methods")
    }

    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let in_node_impl = ui_node_path.is_some();

    // we use the UiNode path provided by the user if possible
    // to avoid an unused import warning.
    let ui_node_path = ui_node_path
        .map(|p| p.to_token_stream())
        .unwrap_or_else(|| quote! { #crate_::UiNode });

    let result = if in_node_impl {
        quote! {
            #errors

            impl #impl_generics #ui_node_path for #self_ty #where_clause {
                #(#node_items)*
                #(#default_ui_items)*
                #(#other_items)*
            }
        }
    } else {
        let input_attrs = input.attrs;
        quote! {
            #errors
            #(#input_attrs)*
            impl #impl_generics #self_ty #where_clause {
                #(#other_items)*
            }

            impl #impl_generics #ui_node_path for #self_ty #where_clause {
                #(#node_items)*
                #(#default_ui_items)*
            }
        }
    };

    //let test = format!("{result}");
    //if test.contains("FocusOnInit") {
    //    println!("{test}");
    //}

    result.into()
}

macro_rules! make_absents {
    ($user_mtds:ident $([fn $ident:ident $($tt:tt)*])+) => {{
        let mut absents = vec![];
        let user_mtds = $user_mtds;
        $(
        if !user_mtds.contains(&ident!(stringify!($ident))) {
            absents.push(parse_quote! {

                #[allow(clippy::borrow_deref_ref)]
                fn $ident $($tt)*
            });
        }
        )+
        absents
    }};
}

fn no_delegate_absents(crate_: TokenStream, user_mtds: HashSet<Ident>) -> Vec<ImplItem> {
    make_absents! { user_mtds
        [fn info(&self, ctx: &mut #crate_::context::InfoContext, info: &mut #crate_::widget_info::WidgetInfoBuilder) { }]

        [fn subscriptions(&self, ctx: &mut #crate_::context::InfoContext, subs: &mut #crate_::widget_info::WidgetSubscriptions) { }]

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) { }]

        [fn event(&mut self, ctx: &mut #crate_::context::WidgetContext, update: &mut #crate_::event::EventUpdate) { }]

        [fn measure(&self, ctx: &mut #crate_::context::MeasureContext) -> #crate_::units::PxSize {
            ctx.metrics.constrains().fill_size()
        }]

        [fn layout(&mut self, ctx: &mut #crate_::context::LayoutContext, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            ctx.metrics.constrains().fill_size()
        }]
        [fn render(&self, ctx: &mut #crate_::context::RenderContext, frame: &mut #crate_::render::FrameBuilder) { }]

        [fn render_update(&self, ctx: &mut #crate_::context::RenderContext, update: &mut #crate_::render::FrameUpdate) { }]
    }
}

fn delegate_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    let child = ident_spanned!(borrow.span()=> "child");
    let child_mut = ident_spanned!(borrow_mut.span()=> "child");

    let deref = quote_spanned! {borrow.span()=>
        &*#child
    };
    let deref_mut = quote_spanned! {borrow_mut.span()=>
        &mut *#child_mut
    };

    make_absents! { user_mtds
        [fn info(&self, ctx: &mut #crate_::context::InfoContext, info: &mut #crate_::widget_info::WidgetInfoBuilder) {
            let #child = {#borrow};
            #crate_::UiNode::info(#deref, ctx, info);
        }]

        [fn subscriptions(&self, ctx: &mut #crate_::context::InfoContext, subs: &mut #crate_::widget_info::WidgetSubscriptions) {
            let #child = {#borrow};
            #crate_::UiNode::subscriptions(#deref, ctx, subs);
        }]

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let mut #child_mut = {#borrow_mut};
            #crate_::UiNode::init(#deref_mut, ctx);
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let mut #child_mut = {#borrow_mut};
            #crate_::UiNode::deinit(#deref_mut, ctx);
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let mut #child_mut = {#borrow_mut};
            #crate_::UiNode::update(#deref_mut, ctx);
        }]

        [fn event(&mut self, ctx: &mut #crate_::context::WidgetContext, update: &mut #crate_::event::EventUpdate) {
            let mut #child_mut = {#borrow_mut};
            #crate_::UiNode::event(#deref_mut, ctx, update);
        }]

        [fn measure(&self, ctx: &mut #crate_::context::MeasureContext) -> #crate_::units::PxSize {
            let mut #child = {#borrow};
            #crate_::UiNode::measure(#deref, ctx)
        }]

        [fn layout(&mut self, ctx: &mut #crate_::context::LayoutContext, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            let mut #child_mut = {#borrow_mut};
            #crate_::UiNode::layout(#deref_mut, ctx, wl)
        }]

        [fn render(&self, ctx: &mut #crate_::context::RenderContext, frame: &mut #crate_::render::FrameBuilder) {
            let #child = {#borrow};
            #crate_::UiNode::render(#deref, ctx, frame);
        }]

        [fn render_update(&self, ctx: &mut #crate_::context::RenderContext, update: &mut #crate_::render::FrameUpdate) {
            let #child = {#borrow};
            #crate_::UiNode::render_update(#deref, ctx, update);
        }]
    }
}

fn delegate_list_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) -> Vec<ImplItem> {
    let children = ident_spanned!(borrow.span()=> "children");
    let children_mut = ident_spanned!(borrow_mut.span()=> "children");
    let deref = quote_spanned! {borrow.span()=>
        &*#children
    };
    let deref_mut = quote_spanned! {borrow_mut.span()=>
        &mut *#children_mut
    };
    make_absents! { user_mtds
        [fn info(&self, ctx: &mut #crate_::context::InfoContext, info: &mut #crate_::widget_info::WidgetInfoBuilder) {
            let #children = {#borrow};
            #crate_::UiNodeList::info_all(#deref, ctx, info);
        }]

        [fn subscriptions(&self, ctx: &mut #crate_::context::InfoContext, subs: &mut #crate_::widget_info::WidgetSubscriptions) {
            let #children = {#borrow};
            #crate_::UiNodeList::subscriptions_all(#deref, ctx, subs);
        }]

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#borrow_mut};
            #crate_::UiNodeList::init_all(#deref_mut, ctx)
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#borrow_mut};
            #crate_::UiNodeList::deinit_all(#deref_mut, ctx)
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#borrow_mut};
            let mut changed = false;
            #crate_::UiNodeList::update_all(#deref_mut, ctx, &mut changed);
            if changed {
                ctx.updates.layout_and_render();
            }
        }]

        [fn event(&mut self, ctx: &mut #crate_::context::WidgetContext, update: &mut #crate_::event::EventUpdate) {
            let #children_mut = {#borrow_mut};
            #crate_::UiNodeList::event_all(#deref_mut, ctx, update);
        }]

        [fn measure(&self, ctx: &mut #crate_::context::MeasureContext) -> #crate_::units::PxSize {
            let #children = {#borrow};
            let mut size = #crate_::units::PxSize::zero();
            #crate_::UiNodeList::measure_all(#deref, ctx, |ctx, _|{}, |_, args| {
                size = size.max(args.size);
            });
            size
        }]

        [fn layout(&mut self, ctx: &mut #crate_::context::LayoutContext, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            let #children_mut = {#borrow_mut};
            let mut size = #crate_::units::PxSize::zero();
            #crate_::UiNodeList::layout_all(#deref_mut, ctx, wl, |ctx, _, _|{}, |_, _, args| {
                size = size.max(args.size);
            });
            size
        }]

        [fn render(&self, ctx: &mut #crate_::context::RenderContext, frame: &mut #crate_::render::FrameBuilder) {
            let #children = {#borrow};
            #crate_::UiNodeList::render_all(#deref, ctx, frame)
        }]

        [fn render_update(&self, ctx: &mut #crate_::context::RenderContext, update: &mut #crate_::render::FrameUpdate) {
            let #children = {#borrow};
            #crate_::UiNodeList::render_update_all(#deref, ctx, update)
        }]
    }
}

fn delegate_iter_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, iter: Expr, iter_mut: Expr) -> Vec<ImplItem> {
    let children = ident_spanned!(iter.span()=> "children");
    let children_mut = ident_spanned!(iter_mut.span()=> "children");

    let iter = quote_spanned! {iter.span()=>
        #crate_::impl_ui_node_util::delegate_iter(#iter)
    };
    let iter_mut = quote_spanned! {iter_mut.span()=>
        #crate_::impl_ui_node_util::delegate_iter_mut(#iter_mut)
    };

    make_absents! { user_mtds
        [fn info(&self, ctx: &mut #crate_::context::InfoContext, info: &mut #crate_::widget_info::WidgetInfoBuilder) {
            let #children = {#iter};
            #crate_::impl_ui_node_util::IterImpl::info_all(#children, ctx, info);
        }]

        [fn subscriptions(&self, ctx: &mut #crate_::context::InfoContext, subs: &mut #crate_::widget_info::WidgetSubscriptions) {
            let #children = {#iter};
            #crate_::impl_ui_node_util::IterImpl::subscriptions_all(#children, ctx, subs);
        }]

        [fn init(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#iter_mut};
            #crate_::impl_ui_node_util::IterMutImpl::init_all(#children_mut, ctx);
        }]

        [fn deinit(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#iter_mut};
            #crate_::impl_ui_node_util::IterMutImpl::deinit_all(#children_mut, ctx);
        }]

        [fn update(&mut self, ctx: &mut #crate_::context::WidgetContext) {
            let #children_mut = {#iter_mut};
            #crate_::impl_ui_node_util::IterMutImpl::update_all(#children_mut, ctx);
        }]

        [fn event(&mut self, ctx: &mut #crate_::context::WidgetContext, update: &mut #crate_::event::EventUpdate) {
            let #children_mut = {#iter_mut};
            #crate_::impl_ui_node_util::IterMutImpl::event_all(#children_mut, ctx, update);
        }]

        [fn measure(&self, ctx: &mut #crate_::context::MeasureContext) -> #crate_::units::PxSize {
            let #children = {#iter};
            #crate_::impl_ui_node_util::IterImpl::measure_all(#children, ctx)
        }]

        [fn layout(&mut self, ctx: &mut #crate_::context::LayoutContext, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize  {
            let #children_mut = {#iter_mut};
            #crate_::impl_ui_node_util::IterMutImpl::layout_all(#children_mut, ctx, wl)
        }]

        [fn render(&self, ctx: &mut #crate_::context::RenderContext, frame: &mut #crate_::render::FrameBuilder) {
            let #children = {#iter};
            #crate_::impl_ui_node_util::IterImpl::render_all(#children, ctx, frame);
        }]

        [fn render_update(&self, ctx: &mut #crate_::context::RenderContext, update: &mut #crate_::render::FrameUpdate) {
            let #children = {#iter};
            #crate_::impl_ui_node_util::IterImpl::render_update_all(#children, ctx, update);
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
    // parse arg0 " = <expr>"
    let expr0 = parse_delegate_expr(args, &arg0)?;

    // get what ident is the second one, delegate pairs can be defined in any order.
    let expected_arg1 = if arg0 == ident { &ident_mut } else { &ident };

    // delegate pair are separated by comma (,)
    let comma = args
        .parse::<Token![,]>()
        .map_err(|_| Error::new(util::after_span(&expr0), format!("expected `, {expected_arg1} = <expr>`")))?;
    // delegate idents require a pair.
    let arg1: Ident = args
        .parse()
        .map_err(|_| Error::new(comma.span(), format!("expected `{expected_arg1} = <expr>`")))?;

    // second ident is not the expected pair.
    if &arg1 != expected_arg1 {
        return Err(Error::new(arg1.span(), format!("expected `{ident_mut}`")));
    }

    // parse arg1 " = <expr>"
    let expr1 = parse_delegate_expr(args, &arg1)?;

    // trailing comma.
    if args.peek(Token![,]) {
        args.parse::<Token![,]>().ok();
    }

    // result is (immutable_expr, mutable_expr)
    if arg0 == ident {
        Ok((expr0, expr1))
    } else {
        Ok((expr1, expr0))
    }
}
fn parse_delegate_expr(args: ParseStream, ident: &Ident) -> Result<Expr> {
    let colon = args
        .parse::<Token![=]>()
        .map_err(|_| Error::new(ident.span(), format!("expected `{ident} = <expr>`")))?;
    let expr: Expr = args
        .parse()
        .map_err(|_| Error::new(colon.span(), format!("expected `{ident} = <expr>`")))?;

    Ok(expr)
}

struct DelegateValidator<'a> {
    pub ident: &'a Ident,
    pub list_variant: Ident,
    pub list_specific_variant: Ident,
    args_count: u8,
    pub delegates: bool,
}
impl<'a> DelegateValidator<'a> {
    fn new(manual_impl: &'a ImplItem) -> Self {
        if let ImplItem::Method(m) = manual_impl {
            DelegateValidator {
                ident: &m.sig.ident,
                list_variant: ident!("{}_all", m.sig.ident),
                list_specific_variant: ident!("item_{}", m.sig.ident),
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
        if (&i.method == self.ident && i.args.len() as u8 == self.args_count)
            || i.method == self.list_variant
            || i.method == self.list_specific_variant
        {
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
