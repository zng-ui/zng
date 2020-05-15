use crate::util;
use crate::widget_new::BuiltPropertyKind;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, punctuated::Punctuated, *};
use uuid::Uuid;

/// `widget!` actual expansion, in stage3 we have all the inherited tokens to work with.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}