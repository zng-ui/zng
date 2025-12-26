//! Native dialog types.

use std::{mem, path::PathBuf};

use zng_txt::Txt;

crate::declare_id! {
    /// Identifies an ongoing async native dialog with the user.
    pub struct DialogId(_);
}

/// Defines a native message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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
impl MsgDialog {
    /// New message dialog.
    pub fn new(title: impl Into<Txt>, message: impl Into<Txt>, icon: MsgDialogIcon, buttons: MsgDialogButtons) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            icon,
            buttons,
        }
    }
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub enum MsgDialogResponse {
    /// Message received or approved.
    Ok,
    /// Question approved.
    Yes,
    /// Question denied.
    No,
    /// Message denied.
    Cancel,
    /// Failed to show the message.
    ///
    /// The associated string may contain debug information, caller should assume that native file dialogs
    /// are not available for the given window ID at the current view-process instance.
    Error(Txt),
}

/// File dialog filters builder.
///
/// # Syntax
///
/// ```txt
/// Display Name|ext1;ext2|All Files|*
/// ```
///
/// You can use the [`push_filter`] method to create filters. Note that the extensions are
/// not glob patterns, they must be an extension (without the dot prefix) or `*` for all files.
///
/// [`push_filter`]: FileDialogFilters::push_filter
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FileDialogFilters(Txt);
impl FileDialogFilters {
    /// New default (empty).
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a filter entry.
    pub fn push_filter<'a>(&mut self, display_name: &str, extensions: impl IntoIterator<Item = &'a str>) -> &mut Self {
        if !self.0.is_empty() && !self.0.ends_with('|') {
            self.0.push('|');
        }

        let extensions: Vec<_> = extensions.into_iter().filter(|s| !s.contains('|') && !s.contains(';')).collect();
        self.push_filter_impl(display_name, extensions)
    }

    fn push_filter_impl(&mut self, display_name: &str, mut extensions: Vec<&str>) -> &mut FileDialogFilters {
        if extensions.is_empty() {
            extensions = vec!["*"];
        }

        let display_name = display_name.replace('|', " ");
        let display_name = display_name.trim();
        if !display_name.is_empty() {
            self.0.push_str(display_name);
            self.0.push_str(" (");
        }
        let mut prefix = "";
        for pat in &extensions {
            self.0.push_str(prefix);
            prefix = ", ";
            self.0.push_str("*.");
            self.0.push_str(pat);
        }
        if !display_name.is_empty() {
            self.0.push(')');
        }

        self.0.push('|');

        prefix = "";
        for pat in extensions {
            self.0.push_str(prefix);
            prefix = ";";
            self.0.push_str(pat);
        }

        self
    }

    /// Iterate over filter entries and patterns.
    pub fn iter_filters(&self) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
        Self::iter_filters_str(self.0.as_str())
    }
    fn iter_filters_str(filters: &str) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
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
            filters: filters.trim_start().trim_start_matches('|'),
        }
    }

    /// Gets the filter text.
    pub fn build(mut self) -> Txt {
        self.0.end_mut();
        self.0
    }
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(filter: Txt) -> FileDialogFilters {
        FileDialogFilters(filter)
    }

    fn from(filter: &'static str) -> FileDialogFilters {
        FileDialogFilters(filter.into())
    }
}

/// Defines a native file dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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

    /// Defines the file dialog looks and what kind of result is expected.
    pub kind: FileDialogKind,
}
impl FileDialog {
    /// New file dialog.
    pub fn new(
        title: impl Into<Txt>,
        starting_dir: PathBuf,
        starting_name: impl Into<Txt>,
        filters: impl Into<Txt>,
        kind: FileDialogKind,
    ) -> Self {
        Self {
            title: title.into(),
            starting_dir,
            starting_name: starting_name.into(),
            filters: filters.into(),
            kind,
        }
    }

    /// Push a filter entry.
    pub fn push_filter<'a>(&mut self, display_name: &str, extensions: impl IntoIterator<Item = &'a str>) -> &mut Self {
        let mut f = FileDialogFilters(mem::take(&mut self.filters));
        f.push_filter(display_name, extensions);
        self.filters = f.build();
        self
    }

    /// Iterate over filter entries and patterns.
    pub fn iter_filters(&self) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
        FileDialogFilters::iter_filters_str(&self.filters)
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
#[non_exhaustive]
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
#[non_exhaustive]
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
impl FileDialogResponse {
    /// Gets the selected paths, or empty for cancel.
    pub fn into_paths(self) -> Result<Vec<PathBuf>, Txt> {
        match self {
            FileDialogResponse::Selected(s) => Ok(s),
            FileDialogResponse::Cancel => Ok(vec![]),
            FileDialogResponse::Error(e) => Err(e),
        }
    }

    /// Gets the last selected path, or `None` for cancel.
    pub fn into_path(self) -> Result<Option<PathBuf>, Txt> {
        self.into_paths().map(|mut p| p.pop())
    }
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

        dlg.push_filter("Display Name", ["abc", "bca"]).push_filter("All Files", ["*"]);
        assert_eq!(expected, dlg.filters);

        let expected = vec![("Display Name (*.abc, *.bca)", vec!["abc", "bca"]), ("All Files (*.*)", vec!["*"])];
        let parsed: Vec<(&str, Vec<&str>)> = dlg.iter_filters().map(|(n, p)| (n, p.collect())).collect();
        assert_eq!(expected, parsed);
    }
}
