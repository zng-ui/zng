use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::*;

use crate::util::Errors;

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Input);
    let message = input.message.value();

    let mut errors = Errors::default();

    let message_params = parse_validate_id(input.message_id, &mut errors);

    let fluent_msg = format!("id = {message}");
    let mut variables = HashSet::new();

    if message.is_empty() {
        errors.push("message cannot be empty", input.message.span());
    } else {
        match fluent_syntax::parser::parse_runtime(fluent_msg.as_str()) {
            Ok(ast) => {
                let span = input.message.span();
                if ast.body.len() > 1 {
                    match &ast.body[1] {
                        fluent_syntax::ast::Entry::Message(m) => {
                            errors.push(format!("unescaped fluent message `{}..`", m.id.name), span);
                        }
                        fluent_syntax::ast::Entry::Term(t) => {
                            errors.push(format!("unescaped fluent term `-{}..`", t.id.name), span);
                        }
                        fluent_syntax::ast::Entry::Comment(_c)
                        | fluent_syntax::ast::Entry::GroupComment(_c)
                        | fluent_syntax::ast::Entry::ResourceComment(_c) => {
                            errors.push("unescaped fluent comment `#..`", span);
                        }
                        fluent_syntax::ast::Entry::Junk { content } => {
                            errors.push(format!("unexpected `{content}`"), span);
                        }
                    }
                } else {
                    match &ast.body[0] {
                        fluent_syntax::ast::Entry::Message(m) => {
                            if m.id.name != "id" {
                                non_user_error!("")
                            }
                            if m.comment.is_some() {
                                non_user_error!("")
                            }

                            if let Some(m) = &m.value {
                                collect_vars_pattern(&mut errors, &mut variables, m);
                            }
                            if !m.attributes.is_empty() {
                                errors.push(format!("unescaped fluent attribute `.{}..`", m.attributes[0].id.name), span);
                            }
                        }
                        fluent_syntax::ast::Entry::Term(t) => {
                            errors.push(format!("unescaped fluent term `-{}..`", t.id.name), span);
                        }
                        fluent_syntax::ast::Entry::Comment(_c)
                        | fluent_syntax::ast::Entry::GroupComment(_c)
                        | fluent_syntax::ast::Entry::ResourceComment(_c) => {
                            errors.push("unescaped fluent comment `#..`", span);
                        }
                        fluent_syntax::ast::Entry::Junk { content } => {
                            errors.push(format!("unexpected `{content}`"), span);
                        }
                    }
                }
            }
            Err((_, e)) => {
                for e in e {
                    errors.push(e, input.message.span());
                }
            }
        }
    }

    if errors.is_empty() {
        let l10n_path = &input.l10n_path;
        let message = &input.message;
        let span = input.message.span();

        let mut build = quote_spanned! {span=>
            #l10n_path::L10N.l10n_message(env!("CARGO_PKG_NAME"), #message_params, #message)
        };
        for var in variables {
            let var_ident = ident_spanned!(span=> "{}", var);
            build.extend(quote_spanned! {span=>
                .l10n_arg(#var, {
                    use #l10n_path::IntoL10nVar;
                    (&mut &mut #l10n_path::L10nSpecialize(Some(#var_ident))).to_l10n_var()
                })
            });
        }
        build.extend(quote! {
            .build()
        });

        build.into()
    } else {
        quote! {
            #errors
        }
        .into()
    }
}

fn collect_vars_pattern<'s>(errors: &mut Errors, vars: &mut HashSet<&'s str>, pattern: &fluent_syntax::ast::Pattern<&'s str>) {
    for el in &pattern.elements {
        match el {
            fluent_syntax::ast::PatternElement::TextElement { .. } => continue,
            fluent_syntax::ast::PatternElement::Placeable { expression } => collect_vars_expr(errors, vars, expression),
        }
    }
}
fn collect_vars_expr<'s>(errors: &mut Errors, vars: &mut HashSet<&'s str>, expression: &fluent_syntax::ast::Expression<&'s str>) {
    match expression {
        fluent_syntax::ast::Expression::Select { selector, variants } => {
            collect_vars_inline_expr(errors, vars, selector);
            for v in variants {
                collect_vars_pattern(errors, vars, &v.value);
            }
        }
        fluent_syntax::ast::Expression::Inline(expr) => collect_vars_inline_expr(errors, vars, expr),
    }
}
fn collect_vars_inline_expr<'s>(errors: &mut Errors, vars: &mut HashSet<&'s str>, inline: &fluent_syntax::ast::InlineExpression<&'s str>) {
    match inline {
        fluent_syntax::ast::InlineExpression::FunctionReference { arguments, .. } => {
            for arg in &arguments.positional {
                collect_vars_inline_expr(errors, vars, arg);
            }
            for arg in &arguments.named {
                collect_vars_inline_expr(errors, vars, &arg.value);
            }
        }
        fluent_syntax::ast::InlineExpression::VariableReference { id } => {
            vars.insert(id.name);
        }
        fluent_syntax::ast::InlineExpression::Placeable { expression } => collect_vars_expr(errors, vars, expression),
        _ => {}
    }
}

struct Input {
    l10n_path: TokenStream,
    message_id: LitStr,
    message: LitStr,
}
impl parse::Parse for Input {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Input {
            l10n_path: non_user_braced!(input, "l10n_path").parse().unwrap(),
            message_id: non_user_braced!(input, "message_id").parse()?,
            message: non_user_braced!(input, "message").parse()?,
        })
    }
}

// Returns "file", "id", "attribute"
fn parse_validate_id(message_id: LitStr, errors: &mut Errors) -> TokenStream {
    let s = message_id.value();
    let span = message_id.span();

    let mut id = s.as_str();
    let mut file = "";
    let mut attribute = "";
    if let Some((f, rest)) = id.rsplit_once('/') {
        file = f;
        id = rest;
    }
    if let Some((i, a)) = id.rsplit_once('.') {
        id = i;
        attribute = a;
    }

    // file
    if !file.is_empty() {
        let mut first = true;
        let mut valid = true;
        let path: &std::path::Path = file.as_ref();
        for c in path.components() {
            if !first || !matches!(c, std::path::Component::Normal(_)) {
                valid = false;
                break;
            }
            first = false;
        }
        if !valid {
            errors.push(format!("invalid file {file:?}, must be a single file name"), span);
            file = "";
        }
    }

    // https://github.com/projectfluent/fluent/blob/master/spec/fluent.ebnf
    // Identifier ::= [a-zA-Z] [a-zA-Z0-9_-]*
    fn validate(value: &str) -> bool {
        let mut first = true;
        if !value.is_empty() {
            for c in value.chars() {
                if !first && (c == '_' || c == '-' || c.is_ascii_digit()) {
                    continue;
                }
                if !c.is_ascii_lowercase() && !c.is_ascii_uppercase() {
                    return false;
                }

                first = false;
            }
        } else {
            return false;
        }
        true
    }
    if !validate(id) {
        errors.push(
            format!("invalid id {id:?}, must start with letter, followed by any letters, digits, `_` or `-`"),
            span,
        );
        id = "invalid__";
    }
    if !attribute.is_empty() && !validate(attribute) {
        errors.push(
            format!("invalid attribute {attribute:?}, must start with letter, followed by any letters, digits, `_` or `-`"),
            span,
        );
        attribute = "";
    }

    if !attribute.is_empty() {
        if let Err((_, e)) = fluent_syntax::parser::parse_runtime(format!("{id} = \n .{attribute} = m")) {
            for e in e {
                errors.push(e, span);
            }
        }
    } else if let Err((_, e)) = fluent_syntax::parser::parse_runtime(format!("{id} = m")) {
        for e in e {
            errors.push(e, span);
        }
    }

    quote_spanned!(span=> #file, #id, #attribute)
}
