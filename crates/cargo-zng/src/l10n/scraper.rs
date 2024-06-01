//! Localization text scraping.

use std::{borrow::Cow, fmt::Debug, io, mem, path::PathBuf, sync::Arc};

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
pub fn scrape_fluent_text(code_files_glob: &str, custom_macro_names: &[&str]) -> FluentTemplate {
    let num_threads = rayon::max_num_threads();
    let mut buf = Vec::with_capacity(num_threads);

    let mut r = FluentTemplate::default();
    for file in glob::glob(code_files_glob).unwrap_or_else(|e| fatal!("{e}")) {
        let file = file.unwrap_or_else(|e| fatal!("{e}"));
        if file.is_dir() {
            continue;
        }
        buf.push(file);
        if buf.len() == num_threads {
            r.extend(scrape_files(&mut buf, custom_macro_names));
        }
    }
    buf.sort();
    if !buf.is_empty() {
        r.extend(scrape_files(&mut buf, custom_macro_names));
    }

    r
}
fn scrape_files(buf: &mut Vec<PathBuf>, custom_macro_names: &[&str]) -> FluentTemplate {
    buf.par_drain(..).map(|f| scrape_file(f, custom_macro_names)).reduce(
        || FluentTemplate {
            notes: vec![],
            entries: vec![],
        },
        |mut a, b| {
            a.extend(b);
            a
        },
    )
}
fn scrape_file(file: PathBuf, custom_macro_names: &[&str]) -> FluentTemplate {
    let file = std::fs::read_to_string(&file).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", file.display()));
    let mut s = file.as_str();

    const BOM: &str = "\u{feff}";
    if s.starts_with(BOM) {
        s = &s[BOM.len()..];
    }
    if let Some(i) = rustc_lexer::strip_shebang(s) {
        s = &s[i..];
    }

    let mut l10n_notes = vec![];
    let mut l10n_section = Arc::new(String::new());

    let mut output: Vec<FluentEntry> = vec![];
    let mut entry = FluentEntry {
        section: l10n_section.clone(),
        comments: String::new(),
        file: String::new(),
        id: String::new(),
        attribute: String::new(),
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

                    if let Some(c) = c.strip_prefix("l10n-") {
                        if let Some(i) = c.find("###") {
                            let file_name = c[..i].trim_end_matches('-');
                            let c = &c[i + "###".len()..];

                            l10n_notes.push(FluentNote {
                                file: file_name.to_owned(),
                                note: c.trim().to_owned(),
                            });
                        } else if let Some(c) = c.strip_prefix("##") {
                            l10n_section = Arc::new(c.trim().to_owned())
                        } else if let Some(c) = c.strip_prefix('#') {
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
                        }
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
                        let message_id = s[..token.len]
                            .trim_start_matches('r')
                            .trim_matches('#')
                            .trim_matches('"')
                            .to_owned();
                        let (file, id, attr) = parse_validate_id(&message_id);
                        entry.file = file;
                        entry.id = id;
                        entry.attribute = attr;

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
                    if let Some(c) = c.strip_prefix("l10n-") {
                        if let Some(i) = c.find("###") {
                            let file_name = c[..i].trim_end_matches('-');
                            let c = &c[i + "###".len()..];

                            l10n_notes.push(FluentNote {
                                file: file_name.to_owned(),
                                note: c.trim().to_owned(),
                            });
                        } else if let Some(c) = c.strip_prefix("##") {
                            l10n_section = Arc::new(c.trim().to_owned())
                        } else if let Some(c) = c.strip_prefix('#') {
                            let c = c.trim_start();

                            if !entry.comments.is_empty() {
                                entry.comments.push('\n');
                            }
                            entry.comments.push_str(c);
                            last_comment_line = line;
                        }
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
                    if let Some(c) = c.strip_prefix("l10n-") {
                        if let Some(i) = c.find("###") {
                            let file_name = c[..i].trim_end_matches('-');
                            let c = &c[i + "###".len()..];

                            l10n_notes.push(FluentNote {
                                file: file_name.to_owned(),
                                note: c.trim().to_owned(),
                            });
                        } else if let Some(c) = c.strip_prefix("##") {
                            l10n_section = Arc::new(c.trim().to_owned())
                        } else if let Some(c) = c.strip_prefix('#') {
                            let c = c.trim_start();

                            if !entry.comments.is_empty() {
                                entry.comments.push('\n');
                            }
                            entry.comments.push_str(c);
                            last_comment_line = line;
                        }
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    entry.file.clear();
                    entry.id.clear();
                    entry.attribute.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
            Expect::StrLiteralMessage => match token.kind {
                rustc_lexer::TokenKind::Literal { kind, .. } => match kind {
                    rustc_lexer::LiteralKind::Str { .. } | rustc_lexer::LiteralKind::RawStr { .. } => {
                        s[..token.len]
                            .trim_start_matches('r')
                            .trim_matches('#')
                            .trim_matches('"')
                            .clone_into(&mut entry.message);

                        output.push(mem::replace(
                            &mut entry,
                            FluentEntry {
                                section: l10n_section.clone(),
                                comments: String::new(),
                                file: String::new(),
                                id: String::new(),
                                attribute: String::new(),
                                message: String::new(),
                            },
                        ));
                        last_entry_line = line;

                        expect = Expect::CommentOrMacroName;
                    }
                    _ => {
                        entry.comments.clear();
                        entry.file.clear();
                        entry.id.clear();
                        entry.attribute.clear();
                        expect = Expect::CommentOrMacroName;
                    }
                },
                rustc_lexer::TokenKind::LineComment => {
                    // comment inside macro

                    let c = s[..token.len].trim().trim_start_matches('/').trim_start();
                    if let Some(c) = c.strip_prefix("l10n-") {
                        if let Some(i) = c.find("###") {
                            let file_name = c[..i].trim_end_matches('-');
                            let c = &c[i + "###".len()..];

                            l10n_notes.push(FluentNote {
                                file: file_name.to_owned(),
                                note: c.trim().to_owned(),
                            });
                        } else if let Some(c) = c.strip_prefix("##") {
                            l10n_section = Arc::new(c.trim().to_owned())
                        } else if let Some(c) = c.strip_prefix('#') {
                            let c = c.trim_start();

                            if !entry.comments.is_empty() {
                                entry.comments.push('\n');
                            }
                            entry.comments.push_str(c);
                            last_comment_line = line;
                        }
                    }
                }
                rustc_lexer::TokenKind::Whitespace => {}
                _ => {
                    entry.comments.clear();
                    entry.file.clear();
                    entry.id.clear();
                    entry.attribute.clear();
                    expect = Expect::CommentOrMacroName;
                }
            },
        }
        s = &s[token.len..];
    }

    FluentTemplate {
        notes: l10n_notes,
        entries: output,
    }
}

/// Represents a standalone note, declared using `// l10n-{file}-### {note}` or `l10n-### {note}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluentNote {
    /// Localization file name pattern where the note must be added.
    pub file: String,

    /// The note.
    pub note: String,
}

/// Represents one call to `l10n!` or similar macro in a Rust code file.
///
/// Use [`scrape_fluent_text`] to collect entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluentEntry {
    /// Resource file section, `// l10n-## `.
    pub section: Arc<String>,

    /// Comments in the line before the macro call or the same line that starts with `l10n-# `.
    pub comments: String,

    /// File name.
    pub file: String,
    /// Message identifier.
    pub id: String,
    /// Attribute name.
    pub attribute: String,

    /// The resource template/fallback.
    pub message: String,
}

/// Represents all calls to `l10n!` or similar macro scraped from selected Rust code files.
///
/// Use [`scrape_fluent_text`] to collect entries.
#[derive(Default)]
pub struct FluentTemplate {
    /// Scraped standalone note comments.
    pub notes: Vec<FluentNote>,

    /// Scraped entries.
    ///
    /// Not sorted, keys not validated.
    pub entries: Vec<FluentEntry>,
}
impl FluentTemplate {
    /// Append `other` to `self`.
    pub fn extend(&mut self, other: Self) {
        self.notes.extend(other.notes);
        self.entries.extend(other.entries);
    }

    /// Sort by file, section, id and attribute. Attributes on different sections are moved to the id
    /// or first attribute section, repeated id and entries are merged.
    pub fn sort(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        // sort to correct attributes in different sections of the same file
        self.entries.sort_unstable_by(|a, b| {
            match a.file.cmp(&b.file) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
            match a.id.cmp(&b.id) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
            a.attribute.cmp(&b.attribute)
        });
        // move attributes to the id section
        let mut file = None;
        let mut id = None;
        let mut id_section = None;
        for entry in &mut self.entries {
            let f = Some(&entry.file);
            let i = Some(&entry.id);

            if (&file, &id) != (&f, &i) {
                file = f;
                id = i;
                id_section = Some(&entry.section);
            } else {
                entry.section = Arc::clone(id_section.as_ref().unwrap());
            }
        }

        // merge repeats
        let mut rmv_marker = None;
        let mut id_start = 0;
        for i in 1..self.entries.len() {
            let prev = &self.entries[i - 1];
            let e = &self.entries[i];

            if e.id == prev.id && e.file == prev.file {
                if let Some(already_i) = self.entries[id_start..i].iter().position(|s| s.attribute == e.attribute) {
                    let already_i = already_i + id_start;
                    // found repeat

                    // mark for remove
                    self.entries[i].section = rmv_marker.get_or_insert_with(|| Arc::new(String::new())).clone();

                    // merge comments
                    let comment = mem::take(&mut self.entries[i].comments);
                    let c = &mut self.entries[already_i].comments;
                    if c.is_empty() {
                        *c = comment;
                    } else if !comment.is_empty() && !c.contains(&comment) {
                        c.push_str("\n\n");
                        c.push_str(&comment);
                    }
                }
            } else {
                id_start = i;
            }
        }
        if let Some(marker) = rmv_marker.take() {
            // remove repeated
            let mut i = 0;
            while i < self.entries.len() {
                while Arc::ptr_eq(&marker, &self.entries[i].section) {
                    self.entries.swap_remove(i);
                }
                i += 1;
            }
        }

        // final sort
        self.entries.sort_unstable_by(|a, b| {
            match a.file.cmp(&b.file) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
            match a.section.cmp(&b.section) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
            match a.id.cmp(&b.id) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
            a.attribute.cmp(&b.attribute)
        });
    }

    /// Write all entries to new FLT files.
    ///
    /// Template must be sorted before this call.
    ///
    /// Entries are separated by file and grouped by section, the notes are
    /// copied at the beginning of each file, the section, id and attribute lists are sorted.
    ///
    /// The `select_l10n_file` closure is called once for each different file, it must return
    /// a writer that will be the output file.
    pub fn write(
        &self,
        transform_msg: fn(&str) -> Cow<str>,
        select_l10n_file: impl Fn(&str) -> io::Result<Box<dyn io::Write + Send>> + Send + Sync,
    ) -> io::Result<()> {
        let mut file = None;
        let mut output = None;
        let mut section = "";
        let mut id = "";

        for (i, entry) in self.entries.iter().enumerate() {
            if file != Some(&entry.file) {
                // Open file and write ### Notes
                let mut out = select_l10n_file(&entry.file)?;

                if !self.notes.is_empty() {
                    for n in &self.notes {
                        let matches_file = if n.file.contains('*') {
                            match glob::Pattern::new(&n.file) {
                                Ok(b) => b.matches(&entry.file),
                                Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
                            }
                        } else {
                            n.file == entry.file
                        };

                        if matches_file {
                            out.write_fmt(format_args!("### {}\n", n.note))?;
                        }
                    }
                    out.write_all("\n".as_bytes())?;
                }

                output = Some(out);
                file = Some(&entry.file);
                section = "";
                id = "";
            }

            let output = output.as_mut().unwrap();

            if id != entry.id && !id.is_empty() {
                output.write_all("\n".as_bytes())?;
            }

            if section != entry.section.as_str() {
                // Write ## Section
                for line in entry.section.lines() {
                    output.write_fmt(format_args!("## {line}\n"))?;
                }
                output.write_all("\n".as_bytes())?;
                section = entry.section.as_str();
            }

            // Write entry:

            // FLT does not allow comments in attributes, but we collected these comments.
            // Solution: write all comments first, this requires peeking.

            // # attribute1:
            // #     comments for attribute1
            // # attribute2:
            // #     comments for attribute1
            // message-id = msg?
            //    .attribute1 = msg1
            //    .attribute2 = msg2

            if id != entry.id {
                id = &entry.id;

                for entry in self.entries[i..].iter() {
                    if entry.id != id {
                        break;
                    }

                    if entry.comments.is_empty() {
                        continue;
                    }
                    let mut prefix = "";
                    if !entry.attribute.is_empty() {
                        output.write_fmt(format_args!("# {}:\n", entry.attribute))?;
                        prefix = "    ";
                    }
                    for line in entry.comments.lines() {
                        output.write_fmt(format_args!("# {prefix}{line}\n"))?;
                    }
                }

                output.write_fmt(format_args!("{id} ="))?;
                if entry.attribute.is_empty() {
                    let mut prefix = " ";

                    for line in transform_msg(&entry.message).lines() {
                        output.write_fmt(format_args!("{prefix}{line}\n"))?;
                        prefix = "    ";
                    }
                } else {
                    output.write_all("\n".as_bytes())?;
                }
            }
            if !entry.attribute.is_empty() {
                output.write_fmt(format_args!("    .{} = ", entry.attribute))?;
                let mut prefix = "";
                for line in transform_msg(&entry.message).lines() {
                    output.write_fmt(format_args!("{prefix}{line}\n"))?;
                    prefix = "        ";
                }
            }
        }

        Ok(())
    }
}

// Returns "file", "id", "attribute"
fn parse_validate_id(s: &str) -> (String, String, String) {
    let mut id = s;
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
            fatal!("invalid file {file:?}, must be a single file name")
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
        fatal!("invalid id {id:?}, must start with letter, followed by any letters, digits, `_` or `-`")
    }
    if !attribute.is_empty() && !validate(attribute) {
        fatal!("invalid id {attribute:?}, must start with letter, followed by any letters, digits, `_` or `-`")
    }

    (file.to_owned(), id.to_owned(), attribute.to_owned())
}
