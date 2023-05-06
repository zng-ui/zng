//! Localization text scraping.

use std::{
    collections::{hash_map, HashMap},
    fmt, io, mem,
    path::PathBuf,
    sync::Arc,
};

use rayon::prelude::*;

/// Scraps all use of the [`l10n!`] macro in Rust files selected by a glob pattern.
///
/// The `custom_macro_names` can contain extra macro names to search in the form of the name literal only (no :: or !).
///
/// Scrapper does not match text inside doc comments or normal comments, but it may match text in code files that
/// are not linked in the `Cargo.toml`.
///
/// See [`FluentEntry`] for details on what is scrapped.
///
/// # Panics
///
/// Panics if `code_files_glob` had an incorrect pattern.
pub fn scrap_fluent_text(code_files_glob: &str, custom_macro_names: &[&str]) -> io::Result<FluentTemplate> {
    let num_threads = rayon::max_num_threads();
    let mut buf = Vec::with_capacity(num_threads);

    let mut r = FluentTemplate {
        entries: vec![],
        sort_by_id: true,
    };
    for file in glob::glob(code_files_glob).unwrap() {
        buf.push(file.map_err(|e| e.into_error())?);
        if buf.len() == num_threads {
            r.entries.extend(scrap_files(&mut buf, custom_macro_names)?);
        }
    }
    if !buf.is_empty() {
        r.entries.extend(scrap_files(&mut buf, custom_macro_names)?);
    }

    Ok(r)
}
fn scrap_files(buf: &mut Vec<PathBuf>, custom_macro_names: &[&str]) -> io::Result<Vec<FluentEntry>> {
    buf.par_drain(..).map(|f| scrap_file(f, custom_macro_names)).reduce(
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
fn scrap_file(file: PathBuf, custom_macro_names: &[&str]) -> io::Result<Vec<FluentEntry>> {
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
        resource_id: String::new(),
        template: String::new(),
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
        StrLiteralTemplate,
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
                        entry.resource_id = s[..token.len]
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
                    expect = Expect::StrLiteralTemplate;
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
                    entry.resource_id.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
            Expect::StrLiteralTemplate => match token.kind {
                rustc_lexer::TokenKind::Literal { kind, .. } => match kind {
                    rustc_lexer::LiteralKind::Str { .. } | rustc_lexer::LiteralKind::RawStr { .. } => {
                        entry.template = s[..token.len]
                            .trim_start_matches('r')
                            .trim_matches('#')
                            .trim_matches('"')
                            .to_owned();

                        output.push(mem::replace(
                            &mut entry,
                            FluentEntry {
                                l10n_file: l10n_file.clone(),
                                comments: String::new(),
                                resource_id: String::new(),
                                template: String::new(),
                            },
                        ));
                        last_entry_line = line;

                        expect = Expect::CommentOrMacroName;
                    }
                    _ => {
                        entry.comments.clear();
                        entry.resource_id.clear();
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
                    entry.resource_id.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
        }
        s = &s[token.len..];
    }

    Ok(output)
}

/// Represents one call to [`l10n!`] or similar macro in a Rust code file.
///
/// Use [`scrap_fluent_text`] to collect entries.
#[derive(Debug, Clone)]
pub struct FluentEntry {
    /// Resource file selected for this resource.
    ///
    /// Selected by a comment in the Rust source file in the format of `l10n-source: #file`.
    pub l10n_file: Arc<String>,

    /// Comments in the line before the macro call or the same line that starts with `l10n: #comment`.
    pub comments: String,

    /// The resource ID.
    pub resource_id: String,
    /// The resource template/fallback.
    pub template: String,
}
impl fmt::Display for FluentEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for line in self.comments.lines() {
            writeln!(f, "# {line}")?;
        }
        writeln!(f, "{} = {}", self.resource_id, self.template)
    }
}

/// Represnets all calls to [`l10n!`] or similat macro scrapped from selected Rust code files.
///
/// Use [`scrap_fluent_text`] to collect entries.
pub struct FluentTemplate {
    /// Scrapped entries.
    pub entries: Vec<FluentEntry>,

    /// If the output entries are sorted by the resource ID, is `true` by default.
    pub sort_by_id: bool,
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
            return write_file(select_l10n_file("")?, self.entries, self.sort_by_id);
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
            .map(|(_, (file, entries))| write_file(file, entries, self.sort_by_id))
            .reduce(|| Ok(()), |a, b| if a.is_err() { a } else { b })
    }
}
fn write_file(mut file: Box<dyn io::Write + Send>, mut entries: Vec<FluentEntry>, sort_by_id: bool) -> io::Result<()> {
    if sort_by_id {
        entries.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
    }
    for entry in entries {
        file.write_fmt(format_args!("{entry}"))?;
    }
    Ok(())
}
