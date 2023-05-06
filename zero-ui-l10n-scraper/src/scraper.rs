//! Localization text scraping.

use std::{
    collections::{hash_map, HashMap},
    io, mem,
    path::PathBuf,
    sync::Arc,
};

use rayon::prelude::*;

/// Scrapes all use of the `l10n!` macro in Rust files selected by a glob pattern.
///
/// The `custom_macro_names` can contain extra macro names to search in the form of the name literal only (no :: or !).
///
/// Scraper does not match text inside doc comments or normal comments, but it may match text in code files that
/// are not linked in the `Cargo.toml`.
///
/// See [`FluentEntry`] for details on what is scraped.
///
/// # Panics
///
/// Panics if `code_files_glob` had an incorrect pattern.
pub fn scrape_fluent_text(code_files_glob: &str, custom_macro_names: &[&str]) -> io::Result<FluentTemplate> {
    let num_threads = rayon::max_num_threads();
    let mut buf = Vec::with_capacity(num_threads);

    let mut r = FluentTemplate { entries: vec![] };
    for file in glob::glob(code_files_glob).unwrap() {
        buf.push(file.map_err(|e| e.into_error())?);
        if buf.len() == num_threads {
            r.entries.extend(scrape_files(&mut buf, custom_macro_names)?);
        }
    }
    if !buf.is_empty() {
        r.entries.extend(scrape_files(&mut buf, custom_macro_names)?);
    }

    Ok(r)
}
fn scrape_files(buf: &mut Vec<PathBuf>, custom_macro_names: &[&str]) -> io::Result<Vec<FluentEntry>> {
    buf.par_drain(..).map(|f| scrape_file(f, custom_macro_names)).reduce(
        || Ok(vec![]),
        |a, b| match (a, b) {
            (Ok(mut a), Ok(b)) => {
                a.extend(b);
                Ok(a)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
    )
}
fn scrape_file(file: PathBuf, custom_macro_names: &[&str]) -> io::Result<Vec<FluentEntry>> {
    let file = std::fs::read_to_string(file)?;
    let mut s = file.as_str();

    const BOM: &str = "\u{feff}";
    if s.starts_with(BOM) {
        s = &s[BOM.len()..];
    }
    if let Some(i) = rustc_lexer::strip_shebang(s) {
        s = &s[i..];
    }

    let mut l10n_file = Arc::new(String::new());

    let mut output: Vec<FluentEntry> = vec![];
    let mut entry = FluentEntry {
        l10n_file: l10n_file.clone(),
        comments: String::new(),
        message_id: String::new(),
        message: String::new(),
    };
    let mut last_comment_line = 0;
    let mut last_entry_line = 0;
    let mut line = 0;

    #[derive(Clone, Copy)]
    enum Expect {
        CommentOrMacroName,
        Bang,
        OpenGroup,
        StrLiteralId,
        Comma,
        StrLiteralMessage,
    }
    let mut expect = Expect::CommentOrMacroName;

    for token in rustc_lexer::tokenize(s) {
        line += s[..token.len].chars().filter(|&a| a == '\n').count();

        match expect {
            Expect::CommentOrMacroName => match token.kind {
                rustc_lexer::TokenKind::LineComment => {
                    let c = s[..token.len].trim().trim_start_matches('/').trim_start();
                    if let Some(c) = c.strip_prefix("l10n:") {
                        let c = c.trim_start();

                        // comment still on the last already inserted entry lines
                        if last_entry_line == line && !output.is_empty() {
                            let last = output.len() - 1;
                            if !output[last].comments.is_empty() {
                                output[last].comments.push('\n');
                            }
                            output[last].comments.push_str(c);
                        } else {
                            if !entry.comments.is_empty() {
                                if (line - last_comment_line) > 1 {
                                    entry.comments.clear();
                                } else {
                                    entry.comments.push('\n');
                                }
                            }
                            entry.comments.push_str(c);
                            last_comment_line = line;
                        }
                    } else if let Some(c) = c.strip_prefix("l10n-source:") {
                        l10n_file = Arc::new(c.trim_start().to_owned())
                    }
                }
                rustc_lexer::TokenKind::Ident => {
                    if (line - last_comment_line) > 1 {
                        entry.comments.clear();
                    }

                    let ident = &s[..token.len];
                    if ["l10n"].iter().chain(custom_macro_names).any(|&i| i == ident) {
                        expect = Expect::Bang;
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {}
            },
            Expect::Bang => {
                if "!" == &s[..token.len] {
                    expect = Expect::OpenGroup;
                } else {
                    entry.comments.clear();
                    expect = Expect::CommentOrMacroName;
                }
            }
            Expect::OpenGroup => match token.kind {
                rustc_lexer::TokenKind::OpenParen | rustc_lexer::TokenKind::OpenBrace | rustc_lexer::TokenKind::OpenBracket => {
                    expect = Expect::StrLiteralId;
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
            Expect::StrLiteralId => match token.kind {
                rustc_lexer::TokenKind::Literal { kind, .. } => match kind {
                    rustc_lexer::LiteralKind::Str { .. } | rustc_lexer::LiteralKind::RawStr { .. } => {
                        entry.message_id = s[..token.len]
                            .trim_start_matches('r')
                            .trim_matches('#')
                            .trim_matches('"')
                            .to_owned();
                        expect = Expect::Comma;
                    }
                    _ => {
                        entry.comments.clear();
                        expect = Expect::CommentOrMacroName;
                    }
                },
                rustc_lexer::TokenKind::LineComment => {
                    // comment inside macro

                    let c = s[..token.len].trim().trim_start_matches('/').trim_start();
                    if let Some(c) = c.strip_prefix("l10n:") {
                        let c = c.trim_start();

                        if !entry.comments.is_empty() {
                            entry.comments.push('\n');
                        }
                        entry.comments.push_str(c);
                        last_comment_line = line;
                    } else if let Some(c) = c.strip_prefix("l10n-source:") {
                        l10n_file = Arc::new(c.trim_start().to_owned())
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
            Expect::Comma => match token.kind {
                rustc_lexer::TokenKind::Comma => {
                    expect = Expect::StrLiteralMessage;
                }
                rustc_lexer::TokenKind::LineComment => {
                    // comment inside macro

                    let c = s[..token.len].trim().trim_start_matches('/').trim_start();
                    if let Some(c) = c.strip_prefix("l10n:") {
                        let c = c.trim_start();

                        if !entry.comments.is_empty() {
                            entry.comments.push('\n');
                        }
                        entry.comments.push_str(c);
                        last_comment_line = line;
                    } else if let Some(c) = c.strip_prefix("l10n-source:") {
                        l10n_file = Arc::new(c.trim_start().to_owned())
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    entry.message_id.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
            Expect::StrLiteralMessage => match token.kind {
                rustc_lexer::TokenKind::Literal { kind, .. } => match kind {
                    rustc_lexer::LiteralKind::Str { .. } | rustc_lexer::LiteralKind::RawStr { .. } => {
                        entry.message = s[..token.len]
                            .trim_start_matches('r')
                            .trim_matches('#')
                            .trim_matches('"')
                            .to_owned();

                        output.push(mem::replace(
                            &mut entry,
                            FluentEntry {
                                l10n_file: l10n_file.clone(),
                                comments: String::new(),
                                message_id: String::new(),
                                message: String::new(),
                            },
                        ));
                        last_entry_line = line;

                        expect = Expect::CommentOrMacroName;
                    }
                    _ => {
                        entry.comments.clear();
                        entry.message_id.clear();
                        expect = Expect::CommentOrMacroName;
                    }
                },
                rustc_lexer::TokenKind::LineComment => {
                    // comment inside macro

                    let c = s[..token.len].trim().trim_start_matches('/').trim_start();
                    if let Some(c) = c.strip_prefix("l10n:") {
                        let c = c.trim_start();

                        if !entry.comments.is_empty() {
                            entry.comments.push('\n');
                        }
                        entry.comments.push_str(c);
                        last_comment_line = line;
                    } else if let Some(c) = c.strip_prefix("l10n-source:") {
                        l10n_file = Arc::new(c.trim_start().to_owned())
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    entry.message_id.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
        }
        s = &s[token.len..];
    }

    Ok(output)
}

/// Represents one call to `l10n!` or similar macro in a Rust code file.
///
/// Use [`scrape_fluent_text`] to collect entries.
#[derive(Debug, Clone)]
pub struct FluentEntry {
    /// Resource file selected for this resource.
    ///
    /// Selected by a comment in the Rust source file in the format of `l10n-source: #file`.
    pub l10n_file: Arc<String>,

    /// Comments in the line before the macro call or the same line that starts with `l10n: #comment`.
    pub comments: String,

    /// The message ID, can contain raw ".attribute".
    pub message_id: String,
    /// The resource template/fallback.
    pub message: String,
}
/// Represnets all calls to `l10n!` or similat macro scraped from selected Rust code files.
///
/// Use [`scrape_fluent_text`] to collect entries.
pub struct FluentTemplate {
    /// Scraped entries.
    pub entries: Vec<FluentEntry>,
}
impl FluentTemplate {
    /// Write all entries to new FLT files.
    ///
    /// Entries are sorted by code file, and if `sort_by_id` is `false` they are also sorted by code line.
    ///
    /// The `select_l10n_file` closure is called once for each different [`FluentEntry::l10n_file`], it must return
    /// a writer that will be the output file.
    pub fn write(self, select_l10n_file: impl Fn(&str) -> io::Result<Box<dyn io::Write + Send>> + Send + Sync) -> io::Result<()> {
        if self.entries.iter().all(|e| e.l10n_file.is_empty()) {
            // simple output.
            return write_file(select_l10n_file("")?, self.entries);
        }

        // group fy l10n file and request the files.
        let mut groups = HashMap::new();
        for entry in self.entries {
            match groups.entry(entry.l10n_file.clone()) {
                hash_map::Entry::Vacant(e) => {
                    let file = select_l10n_file(e.key())?;
                    e.insert((file, vec![entry]));
                }
                hash_map::Entry::Occupied(mut e) => {
                    e.get_mut().1.push(entry);
                }
            }
        }

        groups
            .into_par_iter()
            .map(|(_, (file, entries))| write_file(file, entries))
            .reduce(|| Ok(()), |a, b| if a.is_err() { a } else { b })
    }
}
fn write_file(mut file: Box<dyn io::Write + Send>, mut entries: Vec<FluentEntry>) -> io::Result<()> {
    entries.sort_by(|a, b| a.message_id.cmp(&b.message_id));
    let entries = entries;

    let mut i = 0;
    while i < entries.len() {
        if i > 0 {
            // blank line between entries
            file.write_fmt(format_args!("\n"))?;
        }

        if let Some((id, _)) = entries[i].message_id.split_once('.') {
            // message-id.attribute with only attribute message

            // # attribute1:
            // #     comments for attribute1
            // # attribute2:
            // #     comments for attribute1
            // message-id =
            //    .attribute1 = msg1
            //    .attribute2 = msg2

            // write comments of attributes first
            let mut j = i;
            while j < entries.len() {
                if let Some((next_id, attribute)) = entries[j].message_id.split_once('.') {
                    if next_id == id {
                        if !entries[j].comments.is_empty() {
                            file.write_fmt(format_args!("# {attribute}:\n"))?;

                            for line in entries[j].comments.lines() {
                                file.write_fmt(format_args!("#    {line}\n"))?;
                            }
                        }
                        j += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // write id
            file.write_fmt(format_args!("{id} = \n"))?;

            // write attributes
            for entry in &entries[i..j] {
                let (_, attribute) = entry.message_id.split_once('.').unwrap();

                file.write_fmt(format_args!("    .{attribute} = "))?;

                let mut multi_line_fmt = "";
                for line in entry.message.lines() {
                    file.write_fmt(format_args!("{multi_line_fmt}{line}\n"))?;
                    multi_line_fmt = "        ";
                }
            }

            if j > i {
                i = j - 1;
            }
        } else {
            // message-id with message

            // # comments for message-id
            // #
            // # attribute1:
            // #     comments for attribute1
            // # attribute2:
            // #     comments for attribute1
            // message-id = msg
            //    .attribute1 = msg1
            //    .attribute2 = msg2

            let mut attr_comment_prefix = "";

            if !entries[i].comments.is_empty() {
                for line in entries[i].comments.lines() {
                    file.write_fmt(format_args!("# {line}\n"))?;
                }
                // blank line between id comment and first attribute comment
                attr_comment_prefix = "\n";
            }

            // write comments of attributes first
            let mut j = i + 1;
            while j < entries.len() {
                if let Some((next_id, attribute)) = entries[j].message_id.split_once('.') {
                    if next_id == entries[i].message_id {
                        if !entries[j].comments.is_empty() {
                            file.write_fmt(format_args!("#{attr_comment_prefix} {attribute}:\n"))?;
                            attr_comment_prefix = "";

                            for line in entries[j].comments.lines() {
                                file.write_fmt(format_args!("#    {line}\n"))?;
                            }
                        }
                        j += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // write id
            file.write_fmt(format_args!("{} = ", entries[i].message_id))?;
            let mut multi_line_fmt = "";
            for line in entries[i].message.lines() {
                file.write_fmt(format_args!("{multi_line_fmt}{line}\n"))?;
                multi_line_fmt = "    ";
            }

            // write attributes
            for entry in &entries[i + 1..j] {
                let (_, attribute) = entry.message_id.split_once('.').unwrap();

                file.write_fmt(format_args!("    .{attribute} = "))?;

                let mut multi_line_fmt = "";
                for line in entry.message.lines() {
                    file.write_fmt(format_args!("{multi_line_fmt}{line}\n"))?;
                    multi_line_fmt = "        ";
                }
            }

            i = j - 1;
        }

        i += 1;
    }
    Ok(())
}
