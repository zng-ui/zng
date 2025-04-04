#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Contextual [`DATA`] and validation.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{any::Any, collections::HashMap, fmt, mem, num::NonZeroU8, ops, sync::Arc};

use zng_color::COLOR_SCHEME_VAR;
use zng_var::{BoxedAnyVar, types::ContextualizedVar};
use zng_wgt::prelude::*;

use task::parking_lot::RwLock;

/// Data context.
///
/// Sets the [`DATA`] context for this widget and descendants, replacing the parent's data.
///
/// Note that only one data context can be set at a time, the `data` will override the parent's
/// data even if the type `T` does not match.
#[property(CONTEXT - 1)]
pub fn data<T: VarValue>(child: impl UiNode, data: impl IntoVar<T>) -> impl UiNode {
    with_context_local(child, &DATA_CTX, data.into_var().boxed_any())
}

/// Insert a data note in the context.
///
/// This properties synchronizes the `level` and `note` variables with an [`DATA.annotate`] entry. If
/// the `note` is empty the data note is not inserted.
///
/// [`DATA.annotate`]: DATA::annotate
#[property(CONTEXT, default(DataNoteLevel::INFO, ""))]
pub fn data_note(child: impl UiNode, level: impl IntoVar<DataNoteLevel>, note: impl IntoVar<Txt>) -> impl UiNode {
    let level = level.into_var();
    let note = note.into_var();
    let mut _handle = DataNoteHandle::dummy();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&level).sub_var(&note);

            let note = note.get();
            if !note.is_empty() {
                _handle = DATA.annotate(level.get(), note);
            }
        }
        UiNodeOp::Deinit => {
            _handle = DataNoteHandle::dummy();
        }
        UiNodeOp::Update { .. } => {
            if level.is_new() || note.is_new() {
                let note = note.get();
                _handle = if note.is_empty() {
                    DataNoteHandle::dummy()
                } else {
                    DATA.annotate(level.get(), note)
                };
            }
        }
        _ => {}
    })
}

/// Insert a data [`INFO`] note in the context.
///
/// This properties synchronizes the `note` variable with an [`DATA.inform`] entry. If
/// the `note` is empty the data note is not inserted.
///
/// [`DATA.inform`]: DATA::inform
/// [`INFO`]: DataNoteLevel::INFO
#[property(CONTEXT, default(""))]
pub fn data_info(child: impl UiNode, note: impl IntoVar<Txt>) -> impl UiNode {
    data_note(child, DataNoteLevel::INFO, note)
}

/// Insert a data [`WARN`] note in the context.
///
/// This properties synchronizes the `note` variable with an [`DATA.warn`] entry. If
/// the `note` is empty the data note is not inserted.
///
/// [`DATA.warn`]: DATA::warn
/// [`WARN`]: DataNoteLevel::WARN
#[property(CONTEXT, default(""))]
pub fn data_warn(child: impl UiNode, note: impl IntoVar<Txt>) -> impl UiNode {
    data_note(child, DataNoteLevel::WARN, note)
}

/// Insert a data [`ERROR`] note in the context.
///
/// This properties synchronizes the `note` variable with an [`DATA.invalidate`] entry. If
/// the `note` is empty the data note is not inserted.
///
/// [`DATA.invalidate`]: DATA::invalidate
/// [`ERROR`]: DataNoteLevel::ERROR
#[property(CONTEXT, default(""))]
pub fn data_error(child: impl UiNode, note: impl IntoVar<Txt>) -> impl UiNode {
    data_note(child, DataNoteLevel::ERROR, note)
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

/// Get all [`INFO`] data notes set on the context.
///
/// [`INFO`]: DataNoteLevel::INFO
#[property(CONTEXT - 1)]
pub fn get_data_info(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::INFO));
    })
}

/// Write all [`INFO`] data notes set on the context to a text.
///
/// [`INFO`]: DataNoteLevel::INFO
#[property(CONTEXT - 1)]
pub fn get_data_info_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::INFO));
    })
}

/// Gets if any [`INFO`] data notes are set on the context.
///
/// [`INFO`]: DataNoteLevel::INFO
#[property(CONTEXT - 1)]
pub fn has_data_info(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::INFO));
    })
}

/// Get all [`WARN`] data notes set on the context.
///
/// [`WARN`]: DataNoteLevel::WARN
#[property(CONTEXT - 1)]
pub fn get_data_warn(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::WARN));
    })
}

/// Write all [`WARN`] data notes set on the context to a text.
///
/// [`WARN`]: DataNoteLevel::WARN
#[property(CONTEXT - 1)]
pub fn get_data_warn_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::WARN));
    })
}

/// Gets if any [`WARN`] data notes are set on the context.
///
/// [`WARN`]: DataNoteLevel::WARN
#[property(CONTEXT - 1)]
pub fn has_data_warn(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::WARN));
    })
}

/// Get all [`ERROR`] data notes set on the context.
///
/// [`ERROR`]: DataNoteLevel::ERROR
#[property(CONTEXT - 1)]
pub fn get_data_error(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.clone_level(DataNoteLevel::ERROR));
    })
}

/// Write all [`ERROR`] data notes set on the context to a text.
///
/// [`ERROR`]: DataNoteLevel::ERROR
#[property(CONTEXT - 1)]
pub fn get_data_error_txt(child: impl UiNode, notes: impl IntoVar<Txt>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(n.level_txt(DataNoteLevel::ERROR));
    })
}

/// Gets if any [`ERROR`] data notes are set on the context.
///
/// [`ERROR`]: DataNoteLevel::ERROR
#[property(CONTEXT - 1)]
pub fn has_data_error(child: impl UiNode, any: impl IntoVar<bool>) -> impl UiNode {
    let any = any.into_var();
    with_data_notes(child, move |n| {
        let _ = any.set(n.iter().any(|n| n.level() == DataNoteLevel::ERROR));
    })
}

/// Gets all the notes of highest data level set on the context.
#[property(CONTEXT - 1)]
pub fn get_data_notes_top(child: impl UiNode, notes: impl IntoVar<DataNotes>) -> impl UiNode {
    let notes = notes.into_var();
    with_data_notes(child, move |n| {
        let _ = notes.set(if let Some(top) = n.iter().map(|n| n.level()).max() {
            n.clone_level(top)
        } else {
            DataNotes::default()
        });
    })
}

context_var! {
    /// Color pairs for note levels.
    ///
    /// The colors can be used directly as text color.
    ///
    /// Defaults set only for the named levels.
    pub static DATA_NOTE_COLORS_VAR: HashMap<DataNoteLevel, LightDark> = {
        let mut map = HashMap::new();
        // (dark, light)
        map.insert(DataNoteLevel::INFO, LightDark::new(colors::AZURE, colors::AZURE));
        map.insert(DataNoteLevel::WARN, LightDark::new(colors::ORANGE, colors::YELLOW));
        map.insert(
            DataNoteLevel::ERROR,
            LightDark::new(colors::RED, colors::WHITE.with_alpha(20.pct()).mix_normal(colors::RED)),
        );
        map
    };
}

/// Sets the data note level colors, the parent colors are fully replaced.
///
/// The colors will be used directly as text color.
///
/// This property sets the [`DATA_NOTE_COLORS_VAR`].
#[property(CONTEXT, default(DATA_NOTE_COLORS_VAR))]
pub fn replace_data_note_colors(child: impl UiNode, colors: impl IntoVar<HashMap<DataNoteLevel, LightDark>>) -> impl UiNode {
    with_context_var(child, DATA_NOTE_COLORS_VAR, colors)
}

/// Extend the data note level colors, the `colors` extend the parent colors, only entries of the same level are replaced.
///
/// The colors will be used directly as text color.
///
/// This property sets the [`DATA_NOTE_COLORS_VAR`].
#[property(CONTEXT, default(HashMap::new()))]
pub fn extend_data_note_colors(child: impl UiNode, colors: impl IntoVar<HashMap<DataNoteLevel, LightDark>>) -> impl UiNode {
    with_context_var(
        child,
        DATA_NOTE_COLORS_VAR,
        merge_var!(DATA_NOTE_COLORS_VAR, colors.into_var(), |base, over| {
            let mut base = base.clone();
            base.extend(over);
            base
        }),
    )
}

/// Node that inserts a data note color in [`DATA_NOTE_COLORS_VAR`].
pub fn with_data_note_color(child: impl UiNode, level: DataNoteLevel, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_context_var(
        child,
        DATA_NOTE_COLORS_VAR,
        merge_var!(DATA_NOTE_COLORS_VAR, color.into_var(), move |base, over| {
            let mut base = base.clone();
            base.insert(level, *over);
            base
        }),
    )
}

/// Set the data note [`INFO`] color.
///
/// The color will be used directly as text color.
///
/// [`INFO`]: DataNoteLevel::INFO
#[property(CONTEXT)]
pub fn data_info_color(child: impl UiNode, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_data_note_color(child, DataNoteLevel::INFO, color)
}

/// Set the data note [`WARN`] color.
///
/// The color will be used directly as text color.
///
/// [`WARN`]: DataNoteLevel::WARN
#[property(CONTEXT)]
pub fn data_warn_color(child: impl UiNode, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_data_note_color(child, DataNoteLevel::WARN, color)
}

/// Set the data note [`ERROR`] color.
///
/// The color will be used directly as text color.
///
/// [`ERROR`]: DataNoteLevel::ERROR
#[property(CONTEXT)]
pub fn data_error_color(child: impl UiNode, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_data_note_color(child, DataNoteLevel::ERROR, color)
}

/// Data context and validation.
///
/// This service enables data flow from a context to descendants, and from descendants up-to contexts, like an anonymous context var.
///
/// Arbitrary data can be set on a context using the [`data`] property and retrieved using [`DATA.get`] or [`DATA.req`].
/// Only one data entry and type can exist in a context, nested [`data`] properties override the parent data and type in their context.
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
    pub fn req<T: VarValue>(&self) -> ContextualizedVar<T> {
        self.get(|| panic!("expected DATA of type `{}`", std::any::type_name::<T>()))
    }

    /// Get context data of type `T` if the context data is set with the same type, or gets the `fallback` value.
    pub fn get<T: VarValue>(&self, fallback: impl Fn() -> T + Send + Sync + 'static) -> ContextualizedVar<T> {
        ContextualizedVar::new(move || {
            DATA_CTX
                .get()
                .clone_any()
                .double_boxed_any()
                .downcast::<BoxedVar<T>>()
                .map(|b| *b)
                .unwrap_or_else(|_| LocalVar(fallback()).boxed())
        })
    }

    /// Gets the current context data.
    ///
    /// Note that this does not return a contextualizing var like [`get`], it gets the data var in the calling context.
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

    /// Read-only variable that is the best color for the note level in the context of the current color scheme.
    ///
    /// If the `level` is not found, gets the nearest less than level, if no color is set in the context gets
    /// the black/white for dark/light.
    ///
    /// The color can be used directly as text color, it probably needs mixing or desaturating to use as background.
    pub fn note_color(&self, level: impl IntoVar<DataNoteLevel>) -> impl Var<Rgba> {
        merge_var!(DATA_NOTE_COLORS_VAR, level.into_var(), COLOR_SCHEME_VAR, |map, level, scheme| {
            let c = if let Some(c) = map.get(level) {
                *c
            } else {
                let mut nearest = 0u8;
                let mut color = None;

                for (l, c) in map {
                    if l.0.get() < level.0.get() && l.0.get() > nearest {
                        nearest = l.0.get();
                        color = Some(*c);
                    }
                }

                color.unwrap_or_else(|| LightDark::new(colors::WHITE, colors::BLACK))
            };
            match scheme {
                ColorScheme::Light => c.light,
                ColorScheme::Dark => c.dark,
            }
        })
    }

    /// Read-only variable that is the best color for `INFO` notes in the context of the current color scheme.
    ///
    /// The color can be used directly as text color, it probably needs mixing or desaturating to use as background.
    pub fn info_color(&self) -> impl Var<Rgba> {
        self.note_color(DataNoteLevel::INFO)
    }

    /// Read-only variable that is the best color for `WARN` notes in the context of the current color scheme.
    ///
    /// The color can be used directly as text color, it probably needs mixing or desaturating to use as background.
    pub fn warn_color(&self) -> impl Var<Rgba> {
        self.note_color(DataNoteLevel::WARN)
    }

    /// Read-only variable that is the best color for `ERROR` notes in the context of the current color scheme.
    ///
    /// The color can be used directly as text color, it probably needs mixing or desaturating to use as background.
    pub fn error_color(&self) -> impl Var<Rgba> {
        self.note_color(DataNoteLevel::ERROR)
    }
}

context_local! {
    static DATA_CTX: BoxedAnyVar = LocalVar(()).boxed_any();
    static DATA_NOTES_CTX: RwLock<DataNotesProbe> = RwLock::default();
}

/// Classifies the kind of information conveyed by a [`DataNote`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DataNoteLevel(pub NonZeroU8);
impl DataNoteLevel {
    // SAFETY: values are not zero.

    /// Entry represents useful information.
    pub const INFO: Self = Self(NonZeroU8::new(1).unwrap());
    /// Entry represents a data validation warning.
    pub const WARN: Self = Self(NonZeroU8::new(128).unwrap());
    /// Entry represents a data validation error.
    pub const ERROR: Self = Self(NonZeroU8::new(255).unwrap());

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

    /// Widget that set the annotation.
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
#[diagnostic::on_unimplemented(note = "`DataNoteValue` is implemented for all `T: Debug + Display + Send + Sync + Any")]
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
///
/// [`UiNodeOp`]: zng_wgt::prelude::UiNodeOp
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
                let notes = task::parking_lot::lock_api::RwLockWriteGuard::downgrade(notes);
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
