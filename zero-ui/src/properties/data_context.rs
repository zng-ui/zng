//! Contextual [`DATA`] and validation.

use std::{any::Any, fmt, mem, ops, sync::Arc};

use zero_ui_core::task::parking_lot::RwLock;

use crate::{core::var::types::ContextualizedVar, prelude::new_property::*};

/// Data context.
///
/// Sets the [`DATA`] context for this widget and descendants, replacing the parent's data. Note that only
/// one data context can be set at a time, the `data` will override the parent's data even if the type `T`
/// does not match.
#[property(CONTEXT - 1)]
pub fn data<T: VarValue>(child: impl UiNode, data: impl IntoVar<T>) -> impl UiNode {
    with_context_local(child, &DATA_CTX, data.into_var().boxed_any())
}

/// Get all data notes set on the context.
#[property(CONTEXT - 1)]
pub fn get_data_notes(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone());
    })
}

/// Gets if any data notes are set on the context.
#[property(CONTEXT - 1)]
pub fn has_data_notes(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(!n.is_empty());
    })
}

/// Get all `INFO` data notes set on the context.
#[property(CONTEXT - 1)]
pub fn get_data_info(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::INFO));
    })
}

/// Write all `INFO` data notes set on the context to a text.
#[property(CONTEXT - 1)]
pub fn get_data_info_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::INFO));
    })
}

/// Gets if any `INFO` data notes are set on the context.
#[property(CONTEXT - 1)]
pub fn has_data_info(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::INFO));
    })
}

/// Get all `WARN` data notes set on the context.
#[property(CONTEXT - 1)]
pub fn get_data_warn(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::WARN));
    })
}

/// Write all `WARN` data notes set on the context to a text.
#[property(CONTEXT - 1)]
pub fn get_data_warn_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::WARN));
    })
}

/// Gets if any `WARN` data notes are set on the context.
#[property(CONTEXT - 1)]
pub fn has_data_warn(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::WARN));
    })
}

/// Get all `ERROR` data notes set on the context.
#[property(CONTEXT - 1)]
pub fn get_data_error(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::ERROR));
    })
}

/// Write all `ERROR` data notes set on the context to a text.
#[property(CONTEXT - 1)]
pub fn get_data_error_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::ERROR));
    })
}

/// Gets if any `ERROR` data notes are set on the context.
#[property(CONTEXT - 1)]
pub fn has_data_error(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::ERROR));
    })
}

/// Data context and validation.
///
/// This service enables data flow from a context to descendants, a little like an anonymous context var, and
/// from descendants up-to contexts.
/// 
/// Arbitrary data can be set on a context using the [`data`] property and retrieved using [`DATA.get`] or [`DATA.req`],
/// behaving a little like an anonymous context var. Only one data entry and type can exist in a context, nested
/// [`data`] properties override the parent data and type in their context.
///
/// Annotation on the data can be set back using [`DATA.annotate`] and can be retrieved using the [`get_data_notes`] property,
/// annotations are classified by [`DataNoteLevel`], including `INFO`, `WARN` and `ERROR`. For each level there are specialized
/// methods and properties, as an example, the [`DATA.invalidate`] is used to set an error note, and the [`get_data_error_txt`]
/// property gets the error formatted for display. Data notes are aggregated from descendants up-to the context, continuing
/// up to outer nested contexts too, this means that you can get data errors for a form field by setting [`get_data_error_txt`] on
/// the field widget, and get all form errors from that field and others by also setting [`get_data_error_txt`] in the form widget.
/// 
/// [`data`]: fn@data
/// [`get_data_notes`]: fn@get_data_notes
/// [`get_data_error_txt`]: fn@get_data_error_txt
/// [`DATA.get`]: DATA::get
/// [`DATA.req`]: DATA::req
/// [`DATA.annotate`]: DATA::annotate
/// [`DATA.invalidate`]: DATA::invalidate
pub struct DATA;
impl DATA {
    /// Require context data of type `T`.
    ///
    /// # Panics
    ///
    /// Panics if the context data is not set to a variable of type `T` on the first usage of the returned variable.
    pub fn req<T: VarValue>(&self) -> ContextualizedVar<T, BoxedVar<T>> {
        self.get(|| panic!("expected DATA of type `{}`", std::any::type_name::<T>()))
    }

    /// Get context data of type `T` if the context data is set with the same type, or gets the `fallback` value.
    pub fn get<T: VarValue>(&self, fallback: impl Fn() -> T + Send + Sync + 'static) -> ContextualizedVar<T, BoxedVar<T>> {
        ContextualizedVar::new(Arc::new(move || {
            DATA_CTX
                .get()
                .clone_any()
                .double_boxed_any()
                .downcast::<BoxedVar<T>>()
                .map(|b| *b)
                .unwrap_or_else(|_| LocalVar(fallback()).boxed())
        }))
    }

    /// Gets the current context data.
    ///
    /// Note that this is does not return a contextualizing var like [`get`], it gets the data var in the calling context.
    ///
    /// [`get`]: Self::get
    pub fn get_any(&self) -> BoxedAnyVar {
        DATA_CTX.get().clone_any()
    }

    /// Insert a data note in the current context.
    ///
    /// The note will stay in context until the context is unloaded or the handle is dropped.
    pub fn annotate(&self, level: DataNoteLevel, note: impl DataNoteValue) -> DataNoteHandle {
        if !DATA_NOTES_CTX.is_default() {
            let (note, handle) = DataNote::new(WIDGET.id(), level, note);
            let notes = DATA_NOTES_CTX.get();
            let mut notes = notes.write();
            notes.notes.notes.push(note);
            notes.changed = true;
            handle
        } else {
            DataNoteHandle::dummy()
        }
    }

    /// Insert an `INFO` note in the current context.
    ///
    /// The note will stay in context until the context is unloaded or the handle is dropped.
    pub fn inform(&self, note: impl DataNoteValue) -> DataNoteHandle {
        self.annotate(DataNoteLevel::INFO, note)
    }

    /// Insert a `WARN` note in the current context.
    ///
    /// The note will stay in context until the context is unloaded or the handle is dropped.
    pub fn warn(&self, note: impl DataNoteValue) -> DataNoteHandle {
        self.annotate(DataNoteLevel::WARN, note)
    }

    /// Insert an `ERROR` note in the current context.
    ///
    /// The note will stay in context until the context is unloaded or the handle is dropped.
    pub fn invalidate(&self, note: impl DataNoteValue) -> DataNoteHandle {
        self.annotate(DataNoteLevel::ERROR, note)
    }
}

context_local! {
    static DATA_CTX: BoxedAnyVar = LocalVar(()).boxed_any();
    static DATA_NOTES_CTX: RwLock<DataNotesProbe> = RwLock::default();
}

/// Classifies the kind of information conveyed by a [`DataNote`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DataNoteLevel(pub u8);
impl DataNoteLevel {
    /// Entry represents useful information.
    pub const INFO: Self = Self(0);
    /// Entry represents a data validation warning.
    pub const WARN: Self = Self(128);
    /// Entry represents a data validation error.
    pub const ERROR: Self = Self(255);

    /// Gets the level name, if it is one of the `const` levels.
    pub fn name(self) -> &'static str {
        if self == Self::INFO {
            "INFO"
        } else if self == Self::WARN {
            "WARN"
        } else if self == Self::ERROR {
            "ERROR"
        } else {
            ""
        }
    }
}
impl fmt::Debug for DataNoteLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if name.is_empty() {
            f.debug_tuple("DataNoteLevel").field(&self.0).finish()
        } else {
            if f.alternate() {
                write!(f, "DataNoteLevel::")?;
            }
            write!(f, "{name}")
        }
    }
}

/// Represents an annotation set in a data context.
///
/// See [`DATA`] for more details.
#[derive(Clone)]
pub struct DataNote {
    source: WidgetId,
    level: DataNoteLevel,
    value: std::sync::Weak<dyn DataNoteValue>,
}
impl fmt::Debug for DataNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataNote")
            .field("source", &self.source)
            .field("level", &self.level)
            .field("value", &self.value())
            .finish()
    }
}
impl fmt::Display for DataNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(value) = self.value() {
            write!(f, "{value}")
        } else {
            Ok(())
        }
    }
}
impl PartialEq for DataNote {
    fn eq(&self, other: &Self) -> bool {
        self.value.ptr_eq(&other.value) && self.source == other.source && self.level == other.level
    }
}
impl DataNote {
    /// New note.
    pub fn new(source: WidgetId, level: DataNoteLevel, value: impl DataNoteValue + 'static) -> (Self, DataNoteHandle) {
        let handle = Arc::new(value);
        let value = Arc::downgrade(&handle);
        (Self { source, level, value }, DataNoteHandle(Some(handle)))
    }

    /// Widget that setted the annotation.
    pub fn source(&self) -> WidgetId {
        self.source
    }

    /// Annotation level.
    pub fn level(&self) -> DataNoteLevel {
        self.level
    }

    /// Annotation value.
    ///
    /// Is `None` if the note was dropped since last cleanup.
    pub fn value(&self) -> Option<Arc<dyn DataNoteValue>> {
        self.value.upgrade()
    }

    /// If the note is still valid.
    pub fn retain(&self) -> bool {
        self.value.strong_count() > 0
    }
}

/// Handle for a [`DataNote`] in a context.
#[must_use = "dropping the handle drops the data note"]
pub struct DataNoteHandle(Option<Arc<dyn DataNoteValue>>);
impl DataNoteHandle {
    /// New dummy handle.
    pub fn dummy() -> Self {
        Self(None)
    }

    /// If this is a dummy handle.
    pub fn is_dummy(&self) -> bool {
        self.0.is_some()
    }
}

/// Represents a [`DataNote`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait DataNoteValue: fmt::Debug + fmt::Display + Send + Sync + Any {
    /// /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;
}
impl<T: fmt::Debug + fmt::Display + Send + Sync + Any + 'static> DataNoteValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Represents the data notes set in a context.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DataNotes {
    notes: Vec<DataNote>,
}
impl ops::Deref for DataNotes {
    type Target = [DataNote];

    fn deref(&self) -> &Self::Target {
        &self.notes
    }
}
impl DataNotes {
    /// Remove dropped notes.
    pub fn cleanup(&mut self) -> bool {
        let len = self.notes.len();
        self.notes.retain(|n| n.retain());
        len != self.notes.len()
    }

    /// Clone notes of the same `level`.
    pub fn clone_level(&self, level: DataNoteLevel) -> Self {
        let mut notes = vec![];
        for note in &self.notes {
            if note.level == level {
                notes.push(note.clone())
            }
        }
        Self { notes }
    }

    /// Write all notes of the level to a text.
    ///
    /// Multiple notes are placed each in a line.
    pub fn level_txt(&self, level: DataNoteLevel) -> Txt {
        let mut txt = Txt::from_string(String::new());
        let mut sep = "";
        for note in &self.notes {
            if note.level == level {
                if let Some(value) = note.value() {
                    use std::fmt::Write;
                    let _ = write!(&mut txt, "{sep}{value}");
                    sep = "\n";
                }
            }
        }
        txt.end_mut();
        txt
    }
}
impl fmt::Display for DataNotes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for note in &self.notes {
            if let Some(value) = note.value() {
                write!(f, "{sep}{value}")?;
                sep = "\n";
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct DataNotesProbe {
    notes: DataNotes,
    changed: bool,
}

/// Creates a note that samples [`DataNotes`] in a context.
///
/// The `on_changed` closure is called every time a note is inserted or removed in context. The closure
/// can be called in any [`UiNodeOp`], it is always called after the `child` processed the operation. The
/// notes always change to empty on deinit.
pub fn with_data_notes(child: impl UiNode, mut on_changed: impl FnMut(&DataNotes) + Send + 'static) -> impl UiNode {
    let mut notes = None;
    match_node(child, move |c, op| {
        let is_deinit = match &op {
            UiNodeOp::Init => {
                notes = Some(Arc::new(RwLock::new(DataNotesProbe::default())));
                false
            }
            UiNodeOp::Deinit => true,
            _ => false,
        };

        DATA_NOTES_CTX.with_context(&mut notes, || c.op(op));

        if is_deinit {
            let n = notes.take().unwrap();
            let not_empty = !mem::take(&mut n.write().notes).is_empty();
            if not_empty {
                on_changed(&DataNotes::default());
            }
        } else {
            let notes = notes.as_ref().unwrap();
            let mut notes = notes.write();

            let cleaned = notes.notes.cleanup();
            if mem::take(&mut notes.changed) || cleaned {
                let notes = crate::core::task::parking_lot::lock_api::RwLockWriteGuard::downgrade(notes);
                let notes = &notes.notes;

                if !DATA_NOTES_CTX.is_default() {
                    let parent = DATA_NOTES_CTX.get();
                    let mut parent = parent.write();
                    for note in notes.iter() {
                        if parent.notes.iter().all(|n| n != note) {
                            parent.notes.notes.push(note.clone());
                            parent.changed = true;
                        }
                    }
                }

                on_changed(notes);
            }
        }
    })
}
