extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::__rt::{Span, TokenStream as QTokenStream};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse_macro_input, Attribute, Block, Expr, Ident, ImplItem, ImplItemMethod, ItemImpl, PatType, Token, Type};

macro_rules! error {
    ($span: expr, $msg: expr) => {{
        let error = quote_spanned! {
            $span=>
            compile_error!(concat!("#[impl_ui] ", $msg));
        };

        return TokenStream::from(error);
    }};
}

macro_rules! parse_quote {
    ($($tt:tt)*) => {
        syn::parse(quote!{$($tt)*}.into()).unwrap()
    };
}

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

struct CrateUiEverything {
    crate_: QTokenStream,
}

impl CrateUiEverything {
    pub fn new(crate_: QTokenStream) -> Self {
        CrateUiEverything { crate_ }
    }
}

impl VisitMut for CrateUiEverything {
    fn visit_pat_type_mut(&mut self, i: &mut PatType) {
        match i.ty.as_mut() {
            Type::Path(p) => {
                let path = &mut p.path;
                if let Some(ident) = path.get_ident().clone() {
                    let crate_ = self.crate_.clone();
                    *path = parse_quote! { #crate_::ui::#ident };
                }
            }
            _ => {}
        }

        visit_mut::visit_pat_type_mut(self, i);
    }
}

fn ui_defaults(
    crate_: QTokenStream,
    user_mtds: HashSet<Ident>,
    measure_default: impl Fn(Ident, Vec<Ident>) -> Block,
    render_default: impl Fn(Ident, Vec<Ident>) -> Block,
    point_over_default: impl Fn(Ident, Vec<Ident>) -> Block,
    other_mtds: impl Fn(Ident, Vec<Ident>) -> Block,
) -> Vec<ImplItem> {
    let ui: ItemImpl = parse_quote! {
        impl Ui for Dummy {
            fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn measure(&mut self, available_size: LayoutSize) -> LayoutSize { LayoutSize::default() }
            fn arrange(&mut self, final_size: LayoutSize) { }
            fn render(&self, f: &mut NextFrame) { }
            fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) { }
            fn focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> { None }
            fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
            fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) { }
        }
    };

    unimplemented!()
}

fn ui_leaf_defaults(crate_: QTokenStream, user_mtds: HashSet<Ident>) {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        |_, args| {
            parse_quote! {{
                let mut size = #(#args),*;

                if size.width.is_infinite() {
                    size.width = 0.0;
                }

                if size.height.is_infinite() {
                    size.height = 0.0;
                }

                size
            }}
        },
        /* render */
        |_, _| parse_quote! {{}},
        /* point_over */
        |_, _| parse_quote! {{ None }},
        /* other_mtds */
        |_, _| parse_quote! {{}},
    );
}

fn ui_container_defaults(crate_: QTokenStream, user_mtds: HashSet<Ident>, borrow: Expr, borrow_mut: Expr) {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        |_, args| {
            parse_quote! {{
                let d = #borrow_mut;
                d.measure(#(#args),*)
            }}
        },
        /* render */
        |_, args| parse_quote! {{
            let d = #borrow;
            d.render(#(#args),*)
        }},
        /* point_over */
        |_, args| parse_quote! {{
            let d = #borrow_mut;
            d.point_over(#(#args),*)
         }},
        /* other_mtds */
        |mtd, args| parse_quote! {{
            let d = #borrow_mut;
            d.#mtd(#(#args),*);
        }},
    );
}

fn ui_multi_container_defaults(crate_: QTokenStream, user_mtds: HashSet<Ident>, iter: Expr, iter_mut: Expr) {
    ui_defaults(
        crate_,
        user_mtds,
        /* measure */
        |_, args| {
            parse_quote! {{
                let mut size = Default::default();
                for d in #iter_mut {
                   size = d.measure(#(#args),*).max(size);
                }
                size
            }}
        },
        /* render */
        |_, args| parse_quote! {{
            for d in #iter {
                d.render(#(#args),*);
            }
        }},
        /* point_over */
        |_, args| parse_quote! {{
            for d in #iter {
                if let Some(pt) = d.point_over(#(#args),*) {
                    return Some(pt);
                }
            }
            None
         }},
        /* other_mtds */
        |mtd, args| parse_quote! {{
            for d in #iter_mut {
                d.#mtd(#(#args),*);
            }
        }},
    );
}

fn ui_leaf(crate_: QTokenStream, user_mtds: HashSet<Ident>, ui_mtds: &mut Vec<ImplItem>) {
    let n = quote!(#crate_::ui);

    macro_rules! mtd {
        ($mtd_name:ident $($mtd:tt)+) => {
            let mtd_name = ident(stringify!($mtd_name));
            if !user_mtds.contains(&mtd_name) {
                ui_mtds.push(
                    syn::parse(quote! {
                        #[inline]
                        fn #mtd_name $($mtd)+
                    }.into())
                    .unwrap(),
                );
            }
        };
    }

    mtd!(init(&mut self, v: &mut #n::UiValues, u : &mut #n::NextUpdate) {});
    mtd!(measure(&mut self, available_size: #n::LayoutSize) -> #n::LayoutSize {
        let mut size = available_size;

        if size.width.is_infinite() {
            size.width = 0.0;
        }

        if size.height.is_infinite() {
            size.height = 0.0;
        }

        size
    });
    mtd!(arrange(&mut self, _: #n::LayoutSize) {});
    mtd!(render(&self, _: &mut #n::NextFrame) {});
    mtd!(keyboard_input(&mut self, _: &#n::KeyboardInput, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(focused(&mut self, _: bool, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(mouse_input(&mut self, _: &#n::MouseInput, _: &#n::Hits, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(mouse_move(&mut self, _: &#n::UiMouseMove, _: &#n::Hits, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(mouse_entered(&mut self, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(mouse_left(&mut self, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(close_request(&mut self, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(point_over(&self, _: &#n::Hits) -> Option<#n::LayoutPoint> {None});
    mtd!(value_changed(&mut self, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
    mtd!(parent_value_changed(&mut self, _: &mut #n::UiValues, _: &mut #n::NextUpdate) {});
}

fn ui_container(
    crate_: QTokenStream,
    borrow: Expr,
    borrow_mut: Expr,
    user_mtds: HashSet<Ident>,
    ui_mtds: &mut Vec<ImplItem>,
) {
    let n = quote!(#crate_::ui);

    macro_rules! mtd {
        ($mtd_name:ident $($mtd:tt)+) => {
            let mtd_name = ident(stringify!($mtd_name));
            if !user_mtds.contains(&mtd_name) {
                ui_mtds.push(
                    syn::parse(quote! {
                        #[inline]
                        fn #mtd_name $($mtd)+
                    }.into())
                    .unwrap(),
                );
            }
        };
    }

    mtd!(init(&mut self, v: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.init(v, u);
    });
    mtd!(measure(&mut self, available_size: #n::LayoutSize) -> #n::LayoutSize {
        let d = #borrow_mut;
        d.measure(available_size)
    });
    mtd!(arrange(&mut self, final_size: #n::LayoutSize) {
        let d = #borrow_mut;
        d.arrange(final_size);
    });
    mtd!(render(&self, f: &mut #n::NextFrame) {
        let d = #borrow;
        d.render(f);
    });
    mtd!(keyboard_input(&mut self, k: &#n::KeyboardInput, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.keyboard_input(k, uv, u);
    });
    mtd!(focused(&mut self, f: bool, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.focused(f, uv, u);
    });
    mtd!(mouse_input(&mut self, mi: &#n::MouseInput, h: &#n::Hits, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.mouse_input(mi, h, uv, u);
    });
    mtd!(mouse_move(&mut self, mv: &#n::UiMouseMove, h: &#n::Hits, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.mouse_move(mv, h, uv, u);
    });
    mtd!(mouse_entered(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.mouse_entered(uv, u);
    });
    mtd!(mouse_left(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.mouse_left(uv, u);
    });
    mtd!(close_request(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.close_request(uv, u);
    });
    mtd!(point_over(&self, h: &#n::Hits) -> Option<#n::LayoutPoint> {
        let d = #borrow;
        d.point_over(h)
    });
    mtd!(value_changed(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.value_changed(uv, u);
    });
    mtd!(parent_value_changed(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        let d = #borrow_mut;
        d.parent_value_changed(uv, u);
    });
}

fn ui_multi_container(
    crate_: QTokenStream,
    iter: Expr,
    iter_mut: Expr,
    user_mtds: HashSet<Ident>,
    ui_mtds: &mut Vec<ImplItem>,
) {
    let n = quote!(#crate_::ui);

    macro_rules! mtd {
        ($mtd_name:ident $($mtd:tt)+) => {
            let mtd_name = ident(stringify!($mtd_name));
            if !user_mtds.contains(&mtd_name) {
                ui_mtds.push(
                    syn::parse(quote! {
                        #[inline]
                        fn #mtd_name $($mtd)+
                    }.into())
                    .unwrap(),
                );
            }
        };
    }

    mtd!(init(&mut self, v: &mut #n::UiValues, u : &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.init(v, u);
        }
    });
    mtd!(measure(&mut self, available_size: #n::LayoutSize) -> #n::LayoutSize {
         let mut size = #n::LayoutSize::default();
        for d in #iter_mut {
            size = d.measure(available_size).max(size);
        }
        size
    });
    mtd!(arrange(&mut self, final_size: #n::LayoutSize) {
        for d in #iter_mut {
            d.arrange(final_size);
        }
    });
    mtd!(render(&self, f: &mut #n::NextFrame) {
        for d in #iter {
            d.render(f);
        }
    });
    mtd!(keyboard_input(&mut self, k: &#n::KeyboardInput, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.keyboard_input(k, uv, u);
        }
    });
    mtd!(focused(&mut self, f: bool, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.focused(f, uv, u);
        }
    });
    mtd!(mouse_input(&mut self, mi: &#n::MouseInput, h: &#n::Hits, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.mouse_input(mi, h, uv, u);
        }
    });
    mtd!(mouse_move(&mut self, mv: &#n::UiMouseMove, h: &#n::Hits, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.mouse_move(mv, h, uv, u);
        }
    });
    mtd!(mouse_entered(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.mouse_entered(uv, u);
        }
    });
    mtd!(mouse_left(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.mouse_left(uv, u);
        }
    });
    mtd!(close_request(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.close_request(uv, u);
        }
    });
    mtd!(point_over(&self, h: &#n::Hits) -> Option<#n::LayoutPoint> {
        for d in #iter {
            if let Some(pt) = d.point_over(h) {
                return Some(pt);
            }
        }
        None
    });
    mtd!(value_changed(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.value_changed(uv, u);
        }
    });
    mtd!(parent_value_changed(&mut self, uv: &mut #n::UiValues, u: &mut #n::NextUpdate) {
        for d in #iter_mut {
            d.parent_value_changed(uv, u);
        }
    });
}

enum Args {
    Leaf,
    Container {
        delegate: Expr,
        delegate_mut: Expr,
    },
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
                    delegate: syn::parse(quote!(&self.child).into()).unwrap(),
                    delegate_mut: syn::parse(quote!(&mut self.child).into()).unwrap(),
                }
            } else if arg0 == ident("children") {
                Args::MultiContainer {
                    delegate_iter: syn::parse(quote!(self.children.iter()).into()).unwrap(),
                    delegate_iter_mut: syn::parse(quote!(self.children.iter()).into()).unwrap(),
                }
            } else if arg0 == ident("delegate") {
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

fn impl_ui_impl(args: TokenStream, input: TokenStream, crate_: QTokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemImpl);

    if let Some((_, trait_, _)) = input.trait_ {
        error!(trait_.span(), "expected type impl found trait")
    }

    let ui_marker = ident("Ui");

    let mut ui_items = vec![];
    let mut other_items = vec![];
    let mut ui_item_names = HashSet::new();

    for mut item in input.items {
        let mut is_ui = false;

        if let ImplItem::Method(m) = &mut item {
            if let Some(index) = m.attrs.iter().position(|a| a.path.get_ident() == Some(&ui_marker)) {
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

    match args {
        Args::Leaf => ui_leaf(crate_.clone(), ui_item_names, &mut ui_items),
        Args::Container { delegate, delegate_mut } => {
            ui_container(crate_.clone(), delegate, delegate_mut, ui_item_names, &mut ui_items)
        }
        Args::MultiContainer {
            delegate_iter,
            delegate_iter_mut,
        } => ui_multi_container(
            crate_.clone(),
            delegate_iter,
            delegate_iter_mut,
            ui_item_names,
            &mut ui_items,
        ),
    }

    let impl_ui = ident("impl_ui");
    let mut impl_attrs = input.attrs;
    impl_attrs.retain(|a| a.path.get_ident() != Some(&impl_ui));

    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let result = quote! {
        #(#impl_attrs)*
        impl #impl_generics #self_ty #ty_generics #where_clause {
            #(#other_items)*
        }

        impl #impl_generics #crate_::ui::Ui for #self_ty #ty_generics #where_clause {
            #(#ui_items)*
        }
    };

    TokenStream::from(result)
}

/// Same as `impl_ui` but with type paths using the keyword `crate::` instead of `zero_ui::`.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn impl_ui_crate(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {crate})
}

/// Helper macro for implementing [Ui](zero_ui::ui::Ui). You implement only Ui the
/// methods you need and the macro generates default implementations based on configuration.
///
/// # Usage
///
/// ## `#[impl_ui]`
///
/// Generates blank implementations for events, layout fills finite spaces and collapses in
/// infinite spaces. This should only be used for Uis that don't have descendents.
///
/// ```rust
/// # use zero_ui::ui::{Value, NextFrame, ColorF, LayoutSize, UiValues, NextUpdate};
/// # pub struct FillColor<C: Value<ColorF>>(C);
///
/// #[impl_ui]
/// impl<C: Value<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
///
///     #[Ui]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///
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
///     pub fn new(color: ColorF) -> Self {
///         FillColor(color)
///     }
/// }
///
/// impl<C: Value<ColorF>> zero_ui::ui::Ui for FillColor<C> {
///
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), self.color, None);
///     }
///
///     //TODO list all defaults here
/// }
/// ```
///
/// ## `#[impl_ui(child)]`
///
/// Shorthand for `#[impl_ui(delegate: &self.child, delegate_mut: &mut self.child)]`.
///
/// ## `#[impl_ui(children)]`
///
/// Shorthand for `#[impl_ui(delegate_iter: self.children.iter(), delegate_iter_mut: mut self.children.iter_mut())]`.
///
/// ## Delegate
///
/// Generates implementations for all missing `Ui` methods by delegating to a single descendent.
///
/// ```rust
/// #[impl_ui(delegate: self.0.borrow(), delegate_mut: self.0.borrow_mut())]
/// // TODO
/// ```
///
/// ## Delegate Iter
///
/// Generates implementations for all missing `Ui` methods by delegating to multiple descendents. The default
/// behavior is the same as `z_stack`.
///
/// ```rust
/// #[impl_ui(delegate_iter: self.0.iter(), delegate_iter_mut: self.0.iter_mut())]
/// // TODO
/// ```
#[proc_macro_attribute]
pub fn impl_ui(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {zero_ui})
}

fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}
