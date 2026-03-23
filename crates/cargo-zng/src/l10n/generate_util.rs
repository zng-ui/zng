use std::{borrow::Cow, fmt::Write as _, fs, path::Path};

use fluent_syntax::ast::{Attribute, CallArguments, Entry, Expression, Identifier, InlineExpression, Pattern, PatternElement, VariantKey};

use crate::util;

pub fn generate(dir: &str, to_name: &str, file_header: &str, transform: &impl Fn(&str) -> Cow<str>, check: bool, verbose: bool) {
    let dir_path = Path::new(dir);
    let pattern = dir_path.join("**/*.ftl");
    let to_dir = dir_path.with_file_name(to_name);
    for entry in glob::glob(&pattern.display().to_string()).unwrap_or_else(|e| fatal!("cannot read `{dir}`, {e}")) {
        let entry = entry.unwrap_or_else(|e| fatal!("cannot read `{dir}` entry, {e}"));
        let relative_entry = entry.strip_prefix(dir_path).unwrap();
        let to_file = to_dir.join(relative_entry);
        let _ = util::check_or_create_dir_all(check, to_file.parent().unwrap());
        let display_to = to_file.strip_prefix(to_dir.parent().unwrap()).unwrap();
        generate_file(&entry, &to_file, display_to, file_header, transform, check, verbose);
    }
}

fn generate_file(
    from: &Path,
    to: &Path,
    display_to: &Path,
    file_header: &str,
    transform: &impl Fn(&str) -> Cow<str>,
    check: bool,
    verbose: bool,
) {
    let source = match fs::read_to_string(from) {
        Ok(s) => s,
        Err(e) => {
            error!("cannot read `{}`, {e}", from.display());
            return;
        }
    };

    let source = match fluent_syntax::parser::parse(source) {
        Ok(s) => s,
        Err((s, e)) => {
            error!(
                "cannot parse `{}`\n{}",
                from.display(),
                e.into_iter().map(|e| format!("    {e}")).collect::<Vec<_>>().join("\n")
            );
            s
        }
    };

    let mut output = file_header.to_owned();

    for entry in source.body {
        match entry {
            Entry::Message(m) => write_entry(&mut output, false, &m.id, m.value.as_ref(), &m.attributes, transform),
            Entry::Term(t) => write_entry(&mut output, true, &t.id, Some(&t.value), &t.attributes, transform),
            Entry::Comment(_) | Entry::GroupComment(_) | Entry::ResourceComment(_) | Entry::Junk { .. } => {}
        }
    }

    if let Err(e) = util::check_or_write(check, to, output.as_bytes(), verbose) {
        error!("cannot write `{}`, {e}", to.display());
    } else {
        println!("  generated {}", display_to.display());
    }
}

fn write_entry(
    output: &mut String,
    is_term: bool,
    id: &Identifier<String>,
    value: Option<&Pattern<String>>,
    attributes: &[Attribute<String>],
    transform: &impl Fn(&str) -> Cow<str>,
) {
    write!(output, "\n\n{}{} = ", if is_term { "-" } else { "" }, id.name).unwrap();
    if let Some(value) = value {
        write_pattern(output, value, transform, 1);
    }
    for attr in attributes {
        write!(output, "\n    .{} = ", attr.id.name).unwrap();
        write_pattern(output, &attr.value, transform, 2);
    }
}

fn write_pattern(output: &mut String, pattern: &Pattern<String>, transform: &impl Fn(&str) -> Cow<str>, depth: usize) {
    for el in &pattern.elements {
        match el {
            PatternElement::TextElement { value } => {
                let mut prefix = String::new();
                for line in value.split('\n') {
                    // not .lines() because is consumes trailing empty lines
                    write!(output, "{prefix}{}", transform(line)).unwrap();
                    prefix = format!("\n{}", " ".repeat(depth * 4));
                }
            }
            PatternElement::Placeable { expression } => write_expression(output, expression, transform, depth),
        }
    }
}

fn write_expression(output: &mut String, expr: &Expression<String>, transform: &impl Fn(&str) -> Cow<str>, depth: usize) {
    match expr {
        Expression::Select { selector, variants } => {
            write!(output, "{{").unwrap();
            write_inline_expression_inner(output, selector, transform, depth);
            writeln!(output, " ->").unwrap();

            for v in variants {
                write!(output, "{}", " ".repeat((depth + 1) * 4)).unwrap();
                if v.default {
                    write!(output, "*").unwrap();
                }
                let key = match &v.key {
                    VariantKey::Identifier { name } => name,
                    VariantKey::NumberLiteral { value } => value,
                };
                write!(output, "[{key}] ").unwrap();

                write_pattern(output, &v.value, transform, depth + 2);
                writeln!(output).unwrap();
            }

            writeln!(output, "}}").unwrap();
        }
        Expression::Inline(e) => write_inline_expression(output, e, transform, depth),
    }
}
fn write_inline_expression(output: &mut String, expr: &InlineExpression<String>, transform: &impl Fn(&str) -> Cow<str>, depth: usize) {
    write!(output, "{{ ").unwrap();
    write_inline_expression_inner(output, expr, transform, depth);
    write!(output, " }} ").unwrap();
}
fn write_inline_expression_inner(
    output: &mut String,
    expr: &InlineExpression<String>,
    transform: &impl Fn(&str) -> Cow<str>,
    depth: usize,
) {
    match expr {
        InlineExpression::StringLiteral { value } => {
            let value = transform(value);
            let value = value.replace('\\', "\\\\").replace('"', "\\\"");
            write!(output, "\"{value}\"").unwrap()
        }
        InlineExpression::NumberLiteral { value } => write!(output, "{value}").unwrap(),
        InlineExpression::FunctionReference { id, arguments } => {
            write!(output, "{}", id.name).unwrap();
            write_arguments(output, arguments, transform, depth);
        }
        InlineExpression::MessageReference { id, attribute } => {
            write!(output, "{}", id.name).unwrap();
            if let Some(a) = attribute {
                write!(output, ".{}", a.name).unwrap();
            }
        }
        InlineExpression::TermReference { id, attribute, arguments } => {
            write!(output, "-{}", id.name).unwrap();
            if let Some(a) = attribute {
                write!(output, ".{}", a.name).unwrap();
            }
            if let Some(args) = arguments {
                write_arguments(output, args, transform, depth);
            }
        }
        InlineExpression::VariableReference { id } => write!(output, "${}", id.name).unwrap(),
        InlineExpression::Placeable { expression } => {
            write_expression(output, expression, transform, depth);
        }
    }
}

fn write_arguments(output: &mut String, arguments: &CallArguments<String>, transform: &impl Fn(&str) -> Cow<str>, depth: usize) {
    write!(output, "(").unwrap();
    let mut sep = "";
    for a in &arguments.positional {
        write!(output, "{sep}").unwrap();
        write_inline_expression_inner(output, a, transform, depth);
        sep = ", ";
    }
    for a in &arguments.named {
        write!(output, "{sep}{}:", a.name.name).unwrap();
        write_inline_expression_inner(output, &a.value, transform, depth);
        sep = ", ";
    }
    write!(output, ")").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_write_entry() {
        let source = r#"
-lang = en-US

button = Button

window = 
    .title = Localize Example ({-lang})

click-count = {$n ->
    [one] Clicked {$n} time
    *[other] Clicked {$n} times
}
key-count = {NUMBER($n) ->
    [one] Clicked {$n} time
    *[other] Clicked {$n} times
}
        "#;
        let source = fluent_syntax::parser::parse(source.to_owned()).unwrap();

        let mut output = String::new();
        for entry in &source.body {
            match entry {
                Entry::Message(m) => write_entry(&mut output, false, &m.id, m.value.as_ref(), &m.attributes, &|a| Cow::Borrowed(a)),
                Entry::Term(t) => write_entry(&mut output, true, &t.id, Some(&t.value), &t.attributes, &|a| Cow::Borrowed(a)),
                _ => {}
            }
        }

        let _ =
            fluent_syntax::parser::parse(output.clone()).unwrap_or_else(|e| panic!("write_entry output invalid\n{}\n{output}", &e.1[0]));
    }
}
