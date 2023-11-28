//! Native dialog types.

use std::path::PathBuf;

use zero_ui_txt::Txt;

crate::declare_id! {
    /// Identifies an ongoing async native dialog with the user.
    pub struct DialogId(_);
}

/// Defines a native message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MsgDialog {
    /// Message dialog window title.
    pub title: Txt,
    /// Message text.
    pub message: Txt,
    /// Kind of message.
    pub icon: MsgDialogIcon,
    /// Message buttons.
    pub buttons: MsgDialogButtons,
}
impl Default for MsgDialog {
    fn default() -> Self {
        Self {
            title: Txt::from_str(""),
            message: Txt::from_str(""),
            icon: MsgDialogIcon::Info,
            buttons: MsgDialogButtons::Ok,
        }
    }
}

/// Icon of a message dialog.
///
/// Defines the overall *level* style of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogIcon {
    /// Informational.
    Info,
    /// Warning.
    Warn,
    /// Error.
    Error,
}

/// Buttons of a message dialog.
///
/// Defines what kind of question the user is answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogButtons {
    /// Ok.
    ///
    /// Just a confirmation of message received.
    Ok,
    /// Ok or Cancel.
    ///
    /// Approve selected choice or cancel.
    OkCancel,
    /// Yes or No.
    YesNo,
}

/// Response to a message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogResponse {
    ///
    Ok,
    ///
    Yes,
    ///
    No,
    ///
    Cancel,
    /// Failed to show the message.
    ///
    /// The associated string may contain debug information, caller should assume that native file dialogs
    /// are not available for the given window ID at the current view-process instance.
    Error(Txt),
}

/// Defines a native file dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileDialog {
    /// Dialog window title.
    pub title: Txt,
    /// Selected directory when the dialog opens.
    pub starting_dir: PathBuf,
    /// Starting file name.
    pub starting_name: Txt,
    /// File extension filters.
    ///
    /// Syntax:
    ///
    /// ```txt
    /// Display Name|ext1;ext2|All Files|*
    /// ```
    ///
    /// You can use the [`push_filter`] method to create filters. Note that the extensions are
    /// not glob patterns, they must be an extension (without the dot prefix) or `*` for all files.
    ///
    /// [`push_filter`]: Self::push_filter
    pub filters: Txt,

    /// Defines the file  dialog looks and what kind of result is expected.
    pub kind: FileDialogKind,
}
impl FileDialog {
    /// Push a filter entry.
    pub fn push_filter(&mut self, display_name: &str, extensions: &[&str]) -> &mut Self {
        if !self.filters.is_empty() && !self.filters.ends_with('|') {
            self.filters.push('|');
        }

        let mut extensions: Vec<_> = extensions
            .iter()
            .copied()
            .filter(|&s| !s.contains('|') && !s.contains(';'))
            .collect();
        if extensions.is_empty() {
            extensions = vec!["*"];
        }

        let display_name = display_name.replace('|', " ");
        let display_name = display_name.trim();
        if !display_name.is_empty() {
            self.filters.push_str(display_name);
            self.filters.push_str(" (");
        }
        let mut prefix = "";
        for pat in &extensions {
            self.filters.push_str(prefix);
            prefix = ", ";
            self.filters.push_str("*.");
            self.filters.push_str(pat);
        }
        if !display_name.is_empty() {
            self.filters.push(')');
        }

        self.filters.push('|');

        prefix = "";
        for pat in extensions {
            self.filters.push_str(prefix);
            prefix = ";";
            self.filters.push_str(pat);
        }

        self
    }

    /// Iterate over filter entries and patterns.
    pub fn iter_filters(&self) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
        struct Iter<'a> {
            filters: &'a str,
        }
        struct PatternIter<'a> {
            patterns: &'a str,
        }
        impl<'a> Iterator for Iter<'a> {
            type Item = (&'a str, PatternIter<'a>);

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(i) = self.filters.find('|') {
                    let display_name = &self.filters[..i];
                    self.filters = &self.filters[i + 1..];

                    let patterns = if let Some(i) = self.filters.find('|') {
                        let pat = &self.filters[..i];
                        self.filters = &self.filters[i + 1..];
                        pat
                    } else {
                        let pat = self.filters;
                        self.filters = "";
                        pat
                    };

                    if !patterns.is_empty() {
                        Some((display_name.trim(), PatternIter { patterns }))
                    } else {
                        self.filters = "";
                        None
                    }
                } else {
                    self.filters = "";
                    None
                }
            }
        }
        impl<'a> Iterator for PatternIter<'a> {
            type Item = &'a str;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(i) = self.patterns.find(';') {
                    let pattern = &self.patterns[..i];
                    self.patterns = &self.patterns[i + 1..];
                    Some(pattern.trim())
                } else if !self.patterns.is_empty() {
                    let pat = self.patterns;
                    self.patterns = "";
                    Some(pat)
                } else {
                    self.patterns = "";
                    None
                }
            }
        }
        Iter {
            filters: self.filters.trim_start().trim_start_matches('|'),
        }
    }
}
impl Default for FileDialog {
    fn default() -> Self {
        FileDialog {
            title: Txt::from_str(""),
            starting_dir: PathBuf::new(),
            starting_name: Txt::from_str(""),
            filters: Txt::from_str(""),
            kind: FileDialogKind::OpenFile,
        }
    }
}

/// Kind of file dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileDialogKind {
    /// Pick one file for reading.
    OpenFile,
    /// Pick one or many files for reading.
    OpenFiles,
    /// Pick one directory for reading.
    SelectFolder,
    /// Pick one or many directories for reading.
    SelectFolders,
    /// Pick one file for writing.
    SaveFile,
}

/// Response to a message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileDialogResponse {
    /// Selected paths.
    ///
    /// Is never empty.
    Selected(Vec<PathBuf>),
    /// User did not select any path.
    Cancel,
    /// Failed to show the dialog.
    ///
    /// The associated string may contain debug information, caller should assume that native file dialogs
    /// are not available for the given window ID at the current view-process instance.
    Error(Txt),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_filters() {
        let mut dlg = FileDialog {
            title: "".into(),
            starting_dir: "".into(),
            starting_name: "".into(),
            filters: "".into(),
            kind: FileDialogKind::OpenFile,
        };

        let expected = "Display Name (*.abc, *.bca)|abc;bca|All Files (*.*)|*";

        dlg.push_filter("Display Name", &["abc", "bca"]).push_filter("All Files", &["*"]);
        assert_eq!(expected, dlg.filters);

        let expected = vec![("Display Name (*.abc, *.bca)", vec!["abc", "bca"]), ("All Files (*.*)", vec!["*"])];
        let parsed: Vec<(&str, Vec<&str>)> = dlg.iter_filters().map(|(n, p)| (n, p.collect())).collect();
        assert_eq!(expected, parsed);
    }
}
