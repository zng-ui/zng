//! Localization text scraping.

use std::{fmt::Write as _, fs, io, mem, path::PathBuf, sync::Arc};

use litrs::StringLit;
use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};
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
            buf.sort();
            r.extend(scrape_files(&mut buf, custom_macro_names));
        }
    }
    if !buf.is_empty() {
        buf.sort();
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
fn scrape_file(rs_file: PathBuf, custom_macro_names: &[&str]) -> FluentTemplate {
    let mut r = FluentTemplate::default();

    let file = fs::read_to_string(&rs_file).unwrap_or_else(|e| fatal!("cannot open `{}`, {e}", rs_file.display()));

    if !["l10n!", "command!", "l10n-"]
        .iter()
        .chain(custom_macro_names)
        .any(|h| file.contains(h))
    {
        return FluentTemplate::default();
    }

    // skip UTF-8 BOM
    let file = file.strip_prefix('\u{feff}').unwrap_or(file.as_str());
    // skip shebang line
    let file = if file.starts_with("#!") && !file.starts_with("#![") {
        &file[file.find('\n').unwrap_or(file.len())..]
    } else {
        file
    };

    let mut sections = vec![(0, Arc::new(String::new()))];
    let mut comments = vec![];

    // parse comments
    let mut str_lit = false;
    for (ln, mut line) in file.lines().enumerate() {
        if str_lit {
            // seek end of multiline string literal.
            while let Some(i) = line.find('"') {
                let str_end = i == 0 || !line[..i].ends_with('\\');
                line = &line[i + 1..];
                if str_end {
                    break;
                }
            }
        }
        let line = line.trim();
        if let Some(line) = line.strip_prefix("//") {
            let line = line.trim_start();
            if let Some(c) = line.strip_prefix("l10n-") {
                // l10n comment (// l10n-### note | // l10n-file-### note | // l10n-## section | // l10n-# comment)
                if let Some(i) = c.find("###") {
                    let file_name = c[..i].trim_end_matches('-');
                    let c = &c[i + "###".len()..];

                    r.notes.push(FluentNote {
                        file: file_name.to_owned(),
                        note: c.trim().to_owned(),
                    });
                } else if let Some(c) = c.strip_prefix("##") {
                    sections.push((ln + 1, Arc::new(c.trim().to_owned())));
                } else if let Some(c) = c.strip_prefix('#') {
                    comments.push((ln + 1, c.trim()));
                }
            }
        } else {
            let mut line = line;
            while !line.is_empty() {
                if let Some((code, comment)) = line.split_once("//") {
                    let mut escape = false;
                    for c in code.chars() {
                        if mem::take(&mut escape) {
                            continue;
                        }
                        match c {
                            '\\' => escape = true,
                            '"' => str_lit = !str_lit,
                            _ => {}
                        }
                    }
                    if str_lit {
                        line = comment;
                    } else {
                        if let Some(c) = comment.trim_start().strip_prefix("l10n-#")
                            && !c.starts_with('#')
                        {
                            comments.push((ln + 1, c.trim()));
                        }

                        // comment end
                        break;
                    }
                } else {
                    // no potential comment in line
                    break;
                }
            }
        }
    }

    let file: TokenStream = file.parse().unwrap_or_else(|e| fatal!("cannot parse `{}`, {e}", rs_file.display()));

    // TokenTree::Group that are not matched to l10n macros are pushed on this stack
    let mut stream_stack = vec![file.into_iter()];
    let next = |stack: &mut Vec<proc_macro2::token_stream::IntoIter>| {
        while !stack.is_empty() {
            let tt = stack.last_mut().unwrap().next();
            if tt.is_some() {
                return tt;
            }
            stack.pop();
        }
        None
    };

    let mut tail2 = Vec::with_capacity(2);
    while let Some(tt) = next(&mut stream_stack) {
        match tt {
            TokenTree::Group(g) => {
                if matches!(g.delimiter(), Delimiter::Brace | Delimiter::Parenthesis | Delimiter::Bracket)
                    && tail2.len() == 2
                    && matches!(&tail2[0], TokenTree::Punct(p) if p.as_char() == '!')
                    && matches!(&tail2[1], TokenTree::Ident(i) if ["l10n", "command"].iter().chain(custom_macro_names).any(|n| i == n))
                {
                    // matches #macro_name ! #g

                    let macro_ln = match &tail2[1] {
                        TokenTree::Ident(i) => i.span().start().line,
                        _ => unreachable!(),
                    };

                    tail2.clear();

                    if let Ok(args) = L10nMacroArgs::try_from(g.stream()) {
                        let (file, id, attribute) = match parse_validate_id(&args.id) {
                            Ok(t) => t,
                            Err(e) => {
                                let lc = args.id_span.start();
                                error!("{e}\n     {}:{}:{}", rs_file.display(), lc.line, lc.column);
                                continue;
                            }
                        };

                        // first section before macro
                        debug_assert!(!sections.is_empty()); // always an empty header section
                        let section = sections.iter().position(|(l, _)| *l > macro_ln).unwrap_or(sections.len());
                        let section = sections[section - 1].1.clone();

                        // all comments on the line before macro or on the macro lines
                        let last_ln = g.span_close().end().line;
                        let mut t = String::new();
                        let mut sep = "";
                        for (l, c) in &comments {
                            if *l <= last_ln {
                                if (macro_ln - 1..=last_ln).contains(l) {
                                    t.push_str(sep);
                                    t.push_str(c);
                                    sep = "\n";
                                }
                            } else {
                                break;
                            }
                        }

                        r.entries.push(FluentEntry {
                            section,
                            comments: t,
                            file,
                            id,
                            attribute,
                            message: args.msg,
                        })
                    } else {
                        match CommandMacroArgs::try_from(g.stream()) {
                            Ok(cmds) => {
                                for cmd in cmds.entries {
                                    let (file, id, _attribute) = match parse_validate_id(&cmd.id) {
                                        Ok(t) => t,
                                        Err(e) => {
                                            let lc = cmd.file_span.start();
                                            error!("{e}\n     {}:{}:{}", rs_file.display(), lc.line, lc.column);
                                            continue;
                                        }
                                    };
                                    debug_assert!(_attribute.is_empty());

                                    // first section before macro
                                    let section = sections.iter().position(|(l, _)| *l > macro_ln).unwrap_or(sections.len());
                                    let section = sections[section - 1].1.clone();

                                    for meta in cmd.metadata {
                                        // all comments on the line before meta entry and on the value string lines.
                                        let ln = meta.name.span().start().line;
                                        let last_ln = meta.value_span.end().line;

                                        let mut t = String::new();
                                        let mut sep = "";
                                        for (l, c) in &comments {
                                            if *l <= last_ln {
                                                if (ln - 1..=last_ln).contains(l) {
                                                    t.push_str(sep);
                                                    t.push_str(c);
                                                    sep = "\n";
                                                }
                                            } else {
                                                break;
                                            }
                                        }

                                        r.entries.push(FluentEntry {
                                            section: section.clone(),
                                            comments: t,
                                            file: file.clone(),
                                            id: id.clone(),
                                            attribute: meta.name.to_string(),
                                            message: meta.value,
                                        })
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some((e, span)) = e {
                                    let lc = span.start();
                                    error!("{e}\n     {}:{}:{}", rs_file.display(), lc.line, lc.column);
                                }
                                stream_stack.push(g.stream().into_iter());
                            }
                        }
                    }
                } else {
                    stream_stack.push(g.stream().into_iter());
                }
            }
            tt => {
                if tail2.len() == 2 {
                    tail2.pop();
                }
                tail2.insert(0, tt);
            }
        }
    }

    r
}
struct L10nMacroArgs {
    id: String,
    id_span: Span,
    msg: String,
}
impl TryFrom<TokenStream> for L10nMacroArgs {
    type Error = String;

    fn try_from(macro_group_stream: TokenStream) -> Result<Self, Self::Error> {
        let three: Vec<_> = macro_group_stream.into_iter().take(3).collect();
        match &three[..] {
            [TokenTree::Literal(l0), TokenTree::Punct(p), TokenTree::Literal(l1)] if p.as_char() == ',' => {
                match (StringLit::try_from(l0), StringLit::try_from(l1)) {
                    (Ok(s0), Ok(s1)) => Ok(Self {
                        id: s0.into_value().into_owned(),
                        id_span: l0.span(),
                        msg: s1.into_value().into_owned(),
                    }),
                    _ => Err(String::new()),
                }
            }
            _ => Err(String::new()),
        }
    }
}

struct CommandMacroArgs {
    entries: Vec<CommandMacroEntry>,
}
impl TryFrom<TokenStream> for CommandMacroArgs {
    type Error = Option<(String, Span)>;

    fn try_from(macro_group_stream: TokenStream) -> Result<Self, Self::Error> {
        let mut entries = vec![];
        // seek and parse static IDENT = { .. }
        let mut tail4 = Vec::with_capacity(4);
        for tt in macro_group_stream.into_iter() {
            tail4.push(tt);
            if tail4.len() > 4 {
                tail4.remove(0);
                match &tail4[..] {
                    [
                        TokenTree::Ident(i0),
                        TokenTree::Ident(id),
                        TokenTree::Punct(p0),
                        TokenTree::Group(g),
                    ] if i0 == "static"
                        && p0.as_char() == '='
                        && matches!(g.delimiter(), Delimiter::Brace | Delimiter::Parenthesis | Delimiter::Bracket) =>
                    {
                        match CommandMacroEntry::try_from(g.stream()) {
                            Ok(mut entry) => {
                                entry.id.push('/');
                                entry.id.push_str(&id.to_string());
                                entries.push(entry);
                            }
                            Err(e) => {
                                if e.is_some() {
                                    return Err(e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if entries.is_empty() { Err(None) } else { Ok(Self { entries }) }
    }
}
struct CommandMacroEntry {
    id: String,
    file_span: Span,
    metadata: Vec<CommandMetaEntry>,
}
impl TryFrom<TokenStream> for CommandMacroEntry {
    type Error = Option<(String, Span)>;

    fn try_from(command_meta_group_stream: TokenStream) -> Result<Self, Self::Error> {
        // static FOO_CMD = { #command_meta_group_stream };
        let mut tts = command_meta_group_stream.into_iter();

        let mut r = CommandMacroEntry {
            id: String::new(),
            file_span: Span::call_site(),
            metadata: vec![],
        };

        // parse l10n!: #lit
        let mut buf: Vec<_> = (&mut tts).take(5).collect();
        match &buf[..] {
            [
                TokenTree::Ident(i),
                TokenTree::Punct(p0),
                TokenTree::Punct(p1),
                value,
                TokenTree::Punct(p2),
            ] if i == "l10n" && p0.as_char() == '!' && p1.as_char() == ':' && p2.as_char() == ',' => {
                match litrs::Literal::try_from(value) {
                    Ok(litrs::Literal::String(str)) => {
                        r.id = str.into_value().into_owned();
                        r.file_span = value.span();
                    }
                    Ok(litrs::Literal::Bool(b)) => {
                        if !b.value() {
                            return Err(None);
                        }
                    }
                    _ => {
                        return Err(Some((
                            "unexpected l10n: value, must be string or bool literal".to_owned(),
                            value.span(),
                        )));
                    }
                }
            }
            _ => return Err(None),
        }

        // seek and parse meta: "lit",
        buf.clear();
        for tt in tts {
            if buf.is_empty() && matches!(&tt, TokenTree::Punct(p) if p.as_char() == ',') {
                continue;
            }

            buf.push(tt);
            if buf.len() == 3 {
                match &buf[..] {
                    [TokenTree::Ident(i), TokenTree::Punct(p), TokenTree::Literal(l)] if p.as_char() == ':' => {
                        if let Ok(s) = StringLit::try_from(l) {
                            r.metadata.push(CommandMetaEntry {
                                name: i.clone(),
                                value: s.into_value().into_owned(),
                                value_span: l.span(),
                            })
                        }
                    }
                    _ => {}
                }
                buf.clear();
            }
        }

        if r.metadata.is_empty() { Err(None) } else { Ok(r) }
    }
}
struct CommandMetaEntry {
    name: Ident,
    value: String,
    value_span: Span,
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
                if Arc::ptr_eq(&marker, &self.entries[i].section) {
                    self.entries.swap_remove(i);
                } else {
                    i += 1;
                }
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
    /// The `write_file` closure is called once for each different file, it must write (or check) the file.
    pub fn write(&self, write_file: impl Fn(&str, &str) -> io::Result<()> + Send + Sync) -> io::Result<()> {
        let mut file = None;
        let mut output = String::new();
        let mut section = "";
        let mut id = "";

        for (i, entry) in self.entries.iter().enumerate() {
            if file != Some(&entry.file) {
                if let Some(prev) = &file {
                    write_file(prev, &output)?;
                    output.clear();
                    section = "";
                    id = "";
                }
                file = Some(&entry.file);

                // write ### Notes

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
                            writeln!(&mut output, "### {}", n.note).unwrap();
                        }
                    }
                    writeln!(&mut output).unwrap();
                }
            }

            if id != entry.id && !id.is_empty() {
                writeln!(&mut output).unwrap();
            }

            if section != entry.section.as_str() {
                // Write ## Section
                for line in entry.section.lines() {
                    writeln!(&mut output, "## {line}").unwrap();
                }
                writeln!(&mut output).unwrap();
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
                        writeln!(&mut output, "# {}:", entry.attribute).unwrap();
                        prefix = "    ";
                    }
                    for line in entry.comments.lines() {
                        writeln!(&mut output, "# {prefix}{line}").unwrap();
                    }
                }

                write!(&mut output, "{id} =").unwrap();
                if entry.attribute.is_empty() {
                    let mut prefix = " ";

                    for line in entry.message.lines() {
                        writeln!(&mut output, "{prefix}{line}").unwrap();
                        prefix = "    ";
                    }
                } else {
                    writeln!(&mut output).unwrap();
                }
            }
            if !entry.attribute.is_empty() {
                write!(&mut output, "    .{} = ", entry.attribute).unwrap();
                let mut prefix = "";
                for line in entry.message.lines() {
                    writeln!(&mut output, "{prefix}{line}").unwrap();
                    prefix = "        ";
                }
            }
        }

        if let Some(prev) = &file {
            write_file(prev, &output)?;
        }

        Ok(())
    }
}

// Returns "file", "id", "attribute"
fn parse_validate_id(s: &str) -> Result<(String, String, String), String> {
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
            return Err(format!("invalid file {file:?}, must be a single file name"));
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
        return Err(format!(
            "invalid id {id:?}, must start with letter, followed by any letters, digits, `_` or `-`"
        ));
    }
    if !attribute.is_empty() && !validate(attribute) {
        return Err(format!(
            "invalid id {attribute:?}, must start with letter, followed by any letters, digits, `_` or `-`"
        ));
    }

    Ok((file.to_owned(), id.to_owned(), attribute.to_owned()))
}
