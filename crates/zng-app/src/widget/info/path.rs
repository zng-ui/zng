use crate::{widget::WidgetId, window::WindowId};

use super::*;

/// Full address of a widget.
///
/// The path is reference counted, cloning this struct does not alloc.
#[derive(Clone)]
pub struct WidgetPath {
    window_id: WindowId,
    path: Arc<Vec<WidgetId>>,
}
impl PartialEq for WidgetPath {
    /// Paths are equal if they share the same [window](Self::window_id) and [widget paths](Self::widgets_path).
    fn eq(&self, other: &Self) -> bool {
        self.window_id == other.window_id && self.path == other.path
    }
}
impl Eq for WidgetPath {}
impl PartialEq<InteractionPath> for WidgetPath {
    fn eq(&self, other: &InteractionPath) -> bool {
        other == self
    }
}
impl fmt::Debug for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WidgetPath")
                .field("window_id", &self.window_id)
                .field("path", &self.path)
                .finish_non_exhaustive()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}//", self.window_id)?;
        for w in self.ancestors() {
            write!(f, "{w}/")?;
        }
        write!(f, "{}", self.widget_id())
    }
}
impl WidgetPath {
    /// New custom widget path.
    pub fn new(window_id: WindowId, path: Arc<Vec<WidgetId>>) -> WidgetPath {
        WidgetPath { window_id, path }
    }

    /// Into internal parts.
    pub fn into_parts(self) -> (WindowId, Arc<Vec<WidgetId>>) {
        (self.window_id, self.path)
    }

    /// Id of the window that contains the widgets.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Widgets that contain [`widget_id`](WidgetPath::widget_id), root first.
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 1]
    }

    /// The widget.
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// The widget parent, if it is not the root widget.
    pub fn parent_id(&self) -> Option<WidgetId> {
        self.ancestors().iter().copied().next_back()
    }

    /// [`ancestors`](WidgetPath::ancestors) and [`widget_id`](WidgetPath::widget_id), root first.
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
    }

    /// If the `widget_id` is part of the path.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.path.contains(&widget_id)
    }

    /// Make a path to an ancestor id that is contained in the current path.
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<Cow<'_, WidgetPath>> {
        self.path.iter().position(|&id| id == ancestor_id).map(|i| {
            if i == self.path.len() - 1 {
                Cow::Borrowed(self)
            } else {
                Cow::Owned(WidgetPath {
                    window_id: self.window_id,
                    path: self.path[..i].to_vec().into(),
                })
            }
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other`.
    pub fn shared_ancestor<'a>(&'a self, other: &'a WidgetPath) -> Option<Cow<'a, WidgetPath>> {
        if self.window_id == other.window_id {
            if let Some(i) = self.path.iter().zip(other.path.iter()).position(|(a, b)| a != b) {
                if i == 0 {
                    None
                } else {
                    let path = self.path[..i].to_vec().into();
                    Some(Cow::Owned(WidgetPath {
                        window_id: self.window_id,
                        path,
                    }))
                }
            } else if self.path.len() <= other.path.len() {
                Some(Cow::Borrowed(self))
            } else {
                Some(Cow::Borrowed(other))
            }
        } else {
            None
        }
    }

    /// Gets a path to the root widget of this path.
    pub fn root_path(&self) -> Cow<'_, WidgetPath> {
        if self.path.len() == 1 {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(WidgetPath {
                window_id: self.window_id,
                path: Arc::new(vec![self.path[0]]),
            })
        }
    }

    /// Gets a path to the `widget_id` of this path.
    pub fn sub_path(&self, widget_id: WidgetId) -> Option<Cow<'_, WidgetPath>> {
        if self.widget_id() == widget_id {
            Some(Cow::Borrowed(self))
        } else {
            let i = self.path.iter().position(|&id| id == widget_id)?;
            let path = Self::new(self.window_id, Arc::new(self.path[..=i].to_vec()));
            Some(Cow::Owned(path))
        }
    }
}

/// Represents a [`WidgetPath`] annotated with each widget's [`Interactivity`].
#[derive(Clone)]
pub struct InteractionPath {
    path: WidgetPath,
    blocked: usize,
    disabled: usize,
}
impl PartialEq for InteractionPath {
    /// Paths are equal if the are the same window, widgets and interactivity.
    fn eq(&self, other: &Self) -> bool {
        self.as_path() == other.as_path() && self.blocked == other.blocked && self.disabled == other.disabled
    }
}
impl Eq for InteractionPath {}
impl PartialEq<WidgetPath> for InteractionPath {
    /// Paths are equal if the are the same window, widgets and interactivity.
    fn eq(&self, other: &WidgetPath) -> bool {
        self.as_path() == other
    }
}
impl fmt::Debug for InteractionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("InteractionPath")
                .field("window_id", &self.window_id)
                .field("path", &self.path)
                .field("blocked", &self.blocked_index())
                .field("disabled", &self.disabled_index())
                .finish_non_exhaustive()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for InteractionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}//", self.window_id)?;
        let mut sep = "";
        for (w, i) in self.zip() {
            write!(f, "{sep}{w}{{{i:?}}}")?;
            sep = "/";
        }
        Ok(())
    }
}
impl InteractionPath {
    pub(super) fn new_internal(path: WidgetPath, blocked: usize, disabled: usize) -> Self {
        Self { path, blocked, disabled }
    }

    /// New custom path.
    pub fn new<P: IntoIterator<Item = (WidgetId, Interactivity)>>(window_id: WindowId, path: P) -> InteractionPath {
        let iter = path.into_iter();
        let mut path = Vec::with_capacity(iter.size_hint().0);
        let mut blocked = None;
        let mut disabled = None;
        for (i, (w, interactivity)) in iter.enumerate() {
            path.push(w);
            if blocked.is_none() && interactivity.contains(Interactivity::BLOCKED) {
                blocked = Some(i);
            }
            if disabled.is_none() && interactivity.contains(Interactivity::DISABLED) {
                disabled = Some(i);
            }
        }
        let len = path.len();
        InteractionPath {
            path: WidgetPath::new(window_id, path.into()),
            blocked: blocked.unwrap_or(len),
            disabled: disabled.unwrap_or(len),
        }
    }

    /// New custom path with all widgets enabled.
    pub fn new_enabled(window_id: WindowId, path: Arc<Vec<WidgetId>>) -> InteractionPath {
        let path = WidgetPath::new(window_id, path);
        Self::from_enabled(path)
    }

    /// New interactivity path with all widgets enabled.
    pub fn from_enabled(path: WidgetPath) -> InteractionPath {
        let len = path.path.len();
        InteractionPath {
            path,
            blocked: len,
            disabled: len,
        }
    }

    /// Dereferences to the path.
    pub fn as_path(&self) -> &WidgetPath {
        &self.path
    }

    /// Index of first [`BLOCKED`].
    ///
    /// [`BLOCKED`]: Interactivity::BLOCKED
    pub fn blocked_index(&self) -> Option<usize> {
        if self.blocked < self.path.path.len() {
            Some(self.blocked)
        } else {
            None
        }
    }
    /// Index of first [`DISABLED`].
    ///
    /// [`DISABLED`]: Interactivity::DISABLED
    pub fn disabled_index(&self) -> Option<usize> {
        if self.disabled < self.path.path.len() {
            Some(self.disabled)
        } else {
            None
        }
    }

    /// Interactivity for each widget, root first.
    pub fn interaction_path(&self) -> impl DoubleEndedIterator<Item = Interactivity> + ExactSizeIterator {
        struct InteractivityIter {
            range: ops::Range<usize>,
            blocked: usize,
            disabled: usize,
        }

        impl InteractivityIter {
            fn interactivity(&self, i: usize) -> Interactivity {
                let mut interactivity = Interactivity::ENABLED;
                if self.blocked <= i {
                    interactivity |= Interactivity::BLOCKED;
                }
                if self.disabled <= i {
                    interactivity |= Interactivity::DISABLED;
                }
                interactivity
            }
        }
        impl Iterator for InteractivityIter {
            type Item = Interactivity;

            fn next(&mut self) -> Option<Self::Item> {
                self.range.next().map(|i| self.interactivity(i))
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.range.len(), Some(self.range.len()))
            }
        }
        impl ExactSizeIterator for InteractivityIter {}
        impl DoubleEndedIterator for InteractivityIter {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.range.next_back().map(|i| self.interactivity(i))
            }
        }

        InteractivityIter {
            range: 0..self.path.path.len(),
            blocked: self.blocked,
            disabled: self.disabled,
        }
    }

    /// Search for the interactivity value associated with the widget in the path.
    pub fn interactivity_of(&self, widget_id: WidgetId) -> Option<Interactivity> {
        self.path.widgets_path().iter().position(|&w| w == widget_id).map(|i| {
            let mut interactivity = Interactivity::ENABLED;
            if self.blocked <= i {
                interactivity |= Interactivity::BLOCKED;
            }
            if self.disabled <= i {
                interactivity |= Interactivity::DISABLED;
            }
            interactivity
        })
    }

    /// Interactivity of the widget.
    pub fn interactivity(&self) -> Interactivity {
        let mut interactivity = Interactivity::ENABLED;
        let len = self.path.path.len();
        if self.blocked < len {
            interactivity |= Interactivity::BLOCKED;
        }
        if self.disabled < len {
            interactivity |= Interactivity::DISABLED;
        }
        interactivity
    }

    /// Zip widgets and interactivity.
    pub fn zip(&self) -> impl DoubleEndedIterator<Item = (WidgetId, Interactivity)> + ExactSizeIterator + '_ {
        self.path.widgets_path().iter().copied().zip(self.interaction_path())
    }

    /// Gets the [`ENABLED`] or [`DISABLED`] part of the path, or none if the widget is blocked at the root.
    ///
    /// [`ENABLED`]: Interactivity::ENABLED
    /// [`DISABLED`]: Interactivity::DISABLED
    pub fn unblocked(self) -> Option<InteractionPath> {
        if self.blocked < self.path.path.len() {
            if self.blocked == 0 {
                return None;
            }
            Some(InteractionPath {
                path: WidgetPath {
                    window_id: self.path.window_id,
                    path: self.path.path[..self.blocked].to_vec().into(),
                },
                blocked: self.blocked,
                disabled: self.disabled,
            })
        } else {
            Some(self)
        }
    }

    /// Gets the [`ENABLED`] part of the path, or none if the widget is not enabled at the root.
    ///
    /// [`ENABLED`]: Interactivity::ENABLED
    pub fn enabled(self) -> Option<WidgetPath> {
        let enabled_end = self.blocked.min(self.disabled);

        if enabled_end < self.path.path.len() {
            if enabled_end == 0 {
                return None;
            }
            Some(WidgetPath {
                window_id: self.path.window_id,
                path: self.path.path[..enabled_end].to_vec().into(),
            })
        } else {
            Some(self.path)
        }
    }

    /// Make a path to an ancestor id that is contained in the current path.
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<Cow<'_, InteractionPath>> {
        self.widgets_path().iter().position(|&id| id == ancestor_id).map(|i| {
            if i == self.path.path.len() - 1 {
                Cow::Borrowed(self)
            } else {
                Cow::Owned(InteractionPath {
                    path: WidgetPath {
                        window_id: self.window_id,
                        path: self.path.path[..=i].to_vec().into(),
                    },
                    blocked: self.blocked,
                    disabled: self.disabled,
                })
            }
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other` with the same interactivity.
    pub fn shared_ancestor<'a>(&'a self, other: &'a InteractionPath) -> Option<Cow<'a, InteractionPath>> {
        if self.window_id == other.window_id {
            if let Some(i) = self.zip().zip(other.zip()).position(|(a, b)| a != b) {
                if i == 0 {
                    None
                } else {
                    let path = self.path.path[..i].to_vec().into();
                    Some(Cow::Owned(InteractionPath {
                        path: WidgetPath {
                            window_id: self.window_id,
                            path,
                        },
                        blocked: self.blocked,
                        disabled: self.disabled,
                    }))
                }
            } else if self.path.path.len() <= other.path.path.len() {
                Some(Cow::Borrowed(self))
            } else {
                Some(Cow::Borrowed(other))
            }
        } else {
            None
        }
    }

    /// Gets a path to the root widget of this path.
    pub fn root_path(&self) -> Cow<'_, InteractionPath> {
        if self.path.path.len() == 1 {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(InteractionPath {
                path: WidgetPath {
                    window_id: self.window_id,
                    path: Arc::new(vec![self.path.path[0]]),
                },
                blocked: self.blocked,
                disabled: self.disabled,
            })
        }
    }

    /// Gets a sub-path up to `widget_id` (inclusive), or `None` if the widget is not in the path.
    pub fn sub_path(&self, widget_id: WidgetId) -> Option<Cow<'_, InteractionPath>> {
        if widget_id == self.widget_id() {
            Some(Cow::Borrowed(self))
        } else {
            let path = self.path.sub_path(widget_id)?;
            Some(Cow::Owned(Self {
                path: path.into_owned(),
                blocked: self.blocked,
                disabled: self.disabled,
            }))
        }
    }
}
impl ops::Deref for InteractionPath {
    type Target = WidgetPath;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}
impl From<InteractionPath> for WidgetPath {
    fn from(p: InteractionPath) -> Self {
        p.path
    }
}
