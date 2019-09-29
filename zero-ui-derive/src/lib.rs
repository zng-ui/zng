extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::__rt::{Span, TokenStream as QTokenStream};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::*;

/// Generates default implementations of [Ui](zero_ui::ui::Ui) methods.
///
/// # Usage
///
/// In a inherent impl, anotate the impl block with `#[impl_ui]` and custom `Ui` methods with `#[Ui]`.
///
/// ```rust
/// #[impl_ui]
/// impl<C: Value<ColorF>> FillColor<C> {
///     #[Ui]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
/// }
/// ```
///
/// In a `Ui` trait impl block, anotate the impl block with `#[impl_ui]` and only implement custom methods.
///
/// ```rust
/// #[impl_ui]
/// impl<C: Value<ColorF>> Ui for FillColor<C> {
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
/// }
/// ```
///
/// The generated defaults can be configurated in the macro.
///
/// ## `#[impl_ui]`
///
/// Generates defaults for UI components without descendents.
///
/// ### Defaults
/// * Events: Does nothing, blank implementation.
/// * Layout: Normal fill behavior, fills finite spaces, collapses in infinite spaces.
/// * Render: Does nothing, blank implementation.
/// * Hit-test: Not hit-testable, `point_over` is always `None`.
///
/// ```rust
/// # use zero_ui::ui::{Value, NextFrame, ColorF, LayoutSize, UiValues, NextUpdate};
/// # pub struct FillColor<C: Value<ColorF>>(C);
/// #
/// #[impl_ui]
/// impl<C: Value<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
///     /// Custom impl
///     #[Ui]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///     /// Custom impl
///     #[Ui]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
/// }
/// ```
/// ### Expands to
///
/// ```rust
/// impl<C: Value<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
/// }
///
/// impl<C: Value<ColorF>> zero_ui::ui::Ui for FillColor<C> {
///     /// Custom impl
///     #[inline]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(
///             LayoutRect::from_size(f.final_size()),
///             *self.0,
///             None,
///         );
///     }
///     /// Custom impl
///     #[inline]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::ui::LayoutSize) -> zero_ui::ui::LayoutSize {
///         let mut size = available_size;
///         if size.width.is_infinite() {
///             size.width = 0.0;
///         }
///         if size.height.is_infinite() {
///             size.height = 0.0;
///         }
///         size
///     }
///     #[inline]
///     fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
///         None
///     }
///     #[inline]
///     fn init(&mut self, values: &mut zero_ui::ui::UiValues, update: &mut zero_ui::ui::NextUpdate) {}
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::ui::LayoutSize) {}
///     #[inline]
///     fn keyboard_input(
///         &mut self,
///         input: &zero_ui::ui::KeyboardInput,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn focused(
///         &mut self,
///         focused: bool,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_input(
///         &mut self,
///         input: &zero_ui::ui::MouseInput,
///         hits: &zero_ui::ui::Hits,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_move(
///         &mut self,
///         input: &zero_ui::ui::UiMouseMove,
///         hits: &zero_ui::ui::Hits,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_entered(
///         &mut self,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_left(
///         &mut self,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn close_request(
///         &mut self,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn parent_value_changed(
///         &mut self,
///         values: &mut zero_ui::ui::UiValues,
///         update: &mut zero_ui::ui::NextUpdate,
///     ) {
///     }
/// }
/// ```
///
/// ## `#[impl_ui(child)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui(
///     delegate: &self.child,
///     delegate_mut: &mut self.child
/// )]
/// ```
///
/// ## `#[impl_ui(children)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui(
///     delegate_iter: self.children.iter(),
///     delegate_iter_mut: mut self.children.iter_mut()
/// )]
/// ```
///
/// ## `#[impl_ui(delegate: expr, delegate_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// a reference of another Ui component.
///
/// Both arguments are required and in order.
///
/// ```rust
/// #[impl_ui(delegate: self.0.borrow(), delegate_mut: self.0.borrow_mut())]
/// // TODO
/// ```
///
/// ## `#[impl_ui(delegate_iter: expr, delegate_iter_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// all Ui component references provided by the iterators.
///
/// ### Defaults
/// * Events: Calls the same event method for each `Ui` delegate provided by the iterator.
/// * Layout: Measure all delegates the largest size is returned. Arranges all delegates with the default top-left alignment.
/// * Render: Renders all delegates on top of each other in the iterator order.
/// * Hit-test: Returns the first delegate hit or `None` if none hit.
///
/// ```rust
/// #[impl_ui(delegate_iter: self.0.iter(), delegate_iter_mut: self.0.iter_mut())]
/// // TODO
/// ```
#[proc_macro_attribute]
pub fn impl_ui(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {zero_ui})
}

/// Same as `impl_ui` but with type paths using the keyword `crate::` instead of `zero_ui::`.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn impl_ui_crate(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {crate})
}

/// `Ident` with call_site span.
fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Returns a `TokenStream` with a `compile_error` in the given span with
/// the given error message.
macro_rules! error {
    ($span: expr, $msg: expr) => {{
        let error = quote_spanned! {
            $span=>
            compile_error!(concat!("#[impl_ui] ", $msg));
        };

        return TokenStream::from(error);
    }};
}

/// `syn::parse` `quote`
macro_rules! parse_quote {
    ($($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).unwrap()
    };
}

// /// Same as `parse_quote` but with an `expect` message.
// macro_rules! dbg_parse_quote {
//     ($msg:expr, $($tt:tt)*) => {
//         syn::parse(quote!{$($tt)*}.into()).expect($msg)
//     };
// }

fn impl_ui_impl(args: TokenStream, input: TokenStream, crate_: QTokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemImpl);

    let mut in_ui_impl = false;
    if let Some((_, trait_, _)) = input.trait_ {
        if let Some(seg) = trait_.segments.last() {
            in_ui_impl = seg.ident == ident("Ui");
        }
        if !in_ui_impl {
            error!(
                trait_.span(),
                "expected inherent impl or Ui trait impl, found another trait"
            )
        }
    }

    let ui_marker = ident("Ui");

    let mut ui_items = vec![];
    let mut other_items = vec![];
    let mut ui_item_names = HashSet::new();

    for mut item in input.items {
        let mut is_ui = false;

        if let ImplItem::Method(m) = &mut item {
            if in_ui_impl {
                is_ui = true;
                ui_item_names.insert(m.sig.ident.clone());
            } else if let Some(index) = m.attrs.iter().position(|a| a.path.get_ident() == Some(&ui_marker)) {
                m.attrs.remove(index);
                is_ui = true;
                ui_item_names.insert(m.sig.ident.clone());
            }
        }

        if is_ui {
            ui_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let default_ui_items = match args {
        Args::Leaf => ui_leaf_defaults(crate_.clone(), ui_item_names),
        Args::Container { delegate, delegate_mut } => {
            ui_container_defaults(crate_.clone(), ui_item_names, delegate, delegate_mut)
        }
        Args::MultiContainer {
            delegate_iter,
            delegate_iter_mut,
        } => ui_multi_container_defaults(crate_.clone(), ui_item_names, delegate_iter, delegate_iter_mut),
    };

    let impl_ui = ident("impl_ui");
    let mut impl_attrs = input.attrs;
    impl_attrs.retain(|a| a.path.get_ident() != Some(&impl_ui));

    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let mut inline_all = InlineEverything::new();

    let mut impl_ui = parse_quote! {
        impl #impl_generics #crate_::ui::Ui for #self_ty #where_clause {
            #(#ui_items)*
            #(#default_ui_items)*
        }
    };
    inline_all.visit_item_impl_mut(&mut impl_ui);

    let result = if in_ui_impl {
        quote! {
            #impl_ui
        }
    } else {
        quote! {
            #(#impl_attrs)*
            impl #impl_generics #self_ty #where_clause {
                #(#other_items)*
            }

            #impl_ui
        }
    };

    //let test = format!("{}", result);
    //if test.contains("FillColor") {
    //    println!("{}", test);
    //}

    TokenStream::from(result)
}

/// Parsed macro arguments.
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

/// Visitor that adds `#[inline]` in every `ImplItemMethod`.
struct InlineEverything {
    inline: Attribute,
}
impl InlineEverything {
    pub fn new() -> Self {
        let mut dummy: ImplItemMethod = parse_quote! {
            #[inline]
            fn dummy(&self) {}
        };

        InlineEverything {
            inline: dummy.attrs.remove(0),
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

/// Visitor that prefixes every `PatType` with `#crate::ui::`.
struct CrateUiEverything {
    crate_: QTokenStream,
}

impl CrateUiEverything {
    pub fn new(crate_: QTokenStream) -> Self {
        CrateUiEverything { crate_ }
    }
}

impl VisitMut for CrateUiEverything {
    fn visit_type_mut(&mut self, i: &mut Type) {
        match i {
            Type::Path(p) => {
                let path = &mut p.path;
                if let Some(tident) = path.get_ident() {
                    if tident != &ident("bool") {
                        let crate_ = self.crate_.clone();
                        *path = parse_quote! { #crate_::ui::#tident };
                    }
                }
            }
            _ => {}
        }

        visit_mut::visit_type_mut(self, i);
    }
}

/// Visitor that replaces the block of every `Ui` method found with
/// the specified defaults OR removes the method if the user already defined then.
struct MakeDefaults {
    /// Set of methods the user already defined.
    user_mtds: HashSet<Ident>,
    /// Default block for `measure` method.
    measure_default: Option<Block>,
    /// Default block for `render` method.
    render_default: Option<Block>,
    /// Default block for `point_over` method.
    point_over_default: Option<Block>,
    /// Function that generated default blocks for all other `Ui` methods.
    /// The first argument is the method ident, the secound is a vec of method
    /// argument idents.
    other_mtds: Box<dyn Fn(Ident, Vec<Ident>) -> Block>,
}

impl VisitMut for MakeDefaults {
    fn visit_impl_item_mut(&mut self, i: &mut ImplItem) {
        let mut rmv = false;
        if let ImplItem::Method(m) = i {
            if self.user_mtds.remove(&m.sig.ident) {
                rmv = true;
            } else {
                if m.sig.ident == ident("measure") {
                    m.block = self.measure_default.take().unwrap();
                } else if m.sig.ident == ident("render") {
                    m.block = self.render_default.take().unwrap();
                } else if m.sig.ident == ident("point_over") {
                    m.block = self.point_over_default.take().unwrap();
                } else {
                    m.block = (self.other_mtds)(
                        m.sig.ident.clone(),
                        m.sig
                            .inputs
                            .iter()
                            .filter_map(|a| {
                                if let FnArg::Typed(t) = a {
                                    if let Pat::Ident(i) = t.pat.as_ref() {
                                        return Some(i.ident.clone());
                                    }
                                }
                                None
                            })
                            .collect(),
                    );
                }
            }
        }

        if rmv {
            *i = ImplItem::Verbatim(QTokenStream::new());
        }

        visit_mut::visit_impl_item_mut(self, i);
    }
}

fn ui_defaults(
    crate_: QTokenStream,
    user_mtds: HashSet<Ident>,
    measure_default: Block,
    render_default: Block,
    point_over_default: Block,
    other_mtds: impl Fn(Ident, Vec<Ident>) -> Block + 'static,
) -> Vec<ImplItem> {
    let mut ui: ItemImpl = parse_quote! {
        impl Ui for Dummy {
            fn measure(&mut self, available_size: LayoutSize) -> LayoutSize { LayoutSize::default() }
            fn render(&self, f: &mut NextFrame) { }
            fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> { None }

            fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn arrange(&mut self, final_size: LayoutSize) { }
            fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) { }
            fn focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
        }
    };

    let mut visitor = MakeDefaults {
        user_mtds: user_mtds,
        measure_default: Some(measure_default),
        render_default: Some(render_default),
        point_over_default: Some(point_over_default),
        other_mtds: Box::new(other_mtds),
    };
    visitor.visit_item_impl_mut(&mut ui);

    let mut visitor = CrateUiEverything::new(crate_);
    visitor.visit_item_impl_mut(&mut ui);

    ui.items
        .into_iter()
        .filter(|i| match &i {
            ImplItem::Method(_) => true,
            _ => false,
        })
        .collect()
}

fn ui_leaf_defaults(crate_: QTokenStream, user_mtds: HashSet<Ident>) -> Vec<ImplItem> {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        parse_quote! {{
            let mut size = available_size;

            if size.width.is_infinite() {
                size.width = 0.0;
            }

            if size.height.is_infinite() {
                size.height = 0.0;
            }

            size
        }},
        /* render */
        parse_quote! {{}},
        /* point_over */
        parse_quote! {{ None }},
        /* other_mtds */
        |_, _| parse_quote! {{}},
    )
}

fn ui_container_defaults(
    crate_: QTokenStream,
    user_mtds: HashSet<Ident>,
    borrow: Expr,
    borrow_mut: Expr,
) -> Vec<ImplItem> {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        parse_quote! {{
            let delegate = #borrow_mut;
            delegate.measure(available_size)
        }},
        /* render */
        parse_quote! {{
            let delegate = #borrow;
            delegate.render(f);
        }},
        /* point_over */
        parse_quote! {{
           let delegate = #borrow;
           delegate.point_over(hits)
        }},
        /* other_mtds */
        move |mtd, args| {
            parse_quote! {{
                let delegate = #borrow_mut;
                delegate.#mtd(#(#args),*);
            }}
        },
    )
}

fn ui_multi_container_defaults(
    crate_: QTokenStream,
    user_mtds: HashSet<Ident>,
    iter: Expr,
    iter_mut: Expr,
) -> Vec<ImplItem> {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        parse_quote! {{
            let mut size = Default::default();
            for d in #iter_mut {
               size = d.measure(available_size).max(size);
            }
            size
        }},
        /* render */
        parse_quote! {{
            for d in #iter {
                d.render(f);
            }
        }},
        /* point_over */
        parse_quote! {{
           for d in #iter {
               if let Some(pt) = d.point_over(hits) {
                   return Some(pt);
               }
           }
           None
        }},
        /* other_mtds */
        move |mtd, args| {
            parse_quote! {{
                for d in #iter_mut {
                    d.#mtd(#(#args),*);
                }
            }}
        },
    )
}
