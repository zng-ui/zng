//! Gesture events and control, [`on_click`](fn@on_click), [`click_shortcut`](fn@click_shortcut) and more.
//!
//! These events aggregate multiple lower-level events to represent a user interaction.
//! Prefer using these events over the events directly tied to an input device.

use std::{
    collections::{HashMap, hash_map},
    mem,
};

use zng_app::{
    shortcut::{GestureKey, Shortcuts},
    widget::info::{TreeFilter, iter::TreeIterator},
};
use zng_ext_input::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT},
    gesture::{CLICK_EVENT, GESTURES, ShortcutClick},
};
use zng_var::AnyVar;
use zng_view_api::{access::AccessCmdName, keyboard::Key};
use zng_wgt::{node::bind_state_info, prelude::*};

pub use zng_ext_input::gesture::ClickArgs;

event_property! {
    /// On widget click from any source and of any click count and the widget is enabled.
    ///
    /// This is the most general click handler, it raises for all possible sources of the [`CLICK_EVENT`] and any number
    /// of consecutive clicks. Use [`on_click`](fn@on_click) to handle only primary button clicks or [`on_any_single_click`](fn@on_any_single_click)
    /// to not include double/triple clicks.
    ///
    /// [`CLICK_EVENT`]: zng_ext_input::gesture::CLICK_EVENT
    #[property(EVENT)]
    pub fn on_any_click<on_pre_any_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }

    /// On widget click from any source and of any click count and the widget is disabled.
    #[property(EVENT)]
    pub fn on_disabled_click<on_pre_disabled_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_disabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }

    /// On widget click from any source but excluding double/triple clicks and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is one. Use
    /// [`on_single_click`](fn@on_single_click) to handle only primary button clicks.
    ///
    /// [`CLICK_EVENT`]: zng_ext_input::gesture::CLICK_EVENT
    #[property(EVENT)]
    pub fn on_any_single_click<on_pre_any_single_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_single() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }

    /// On widget double click from any source and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is two. Use
    /// [`on_double_click`](fn@on_double_click) to handle only primary button clicks.
    ///
    /// [`CLICK_EVENT`]: zng_ext_input::gesture::CLICK_EVENT
    #[property(EVENT)]
    pub fn on_any_double_click<on_pre_any_double_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_double() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// On widget triple click from any source and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is three. Use
    /// [`on_triple_click`](fn@on_triple_click) to handle only primary button clicks.
    ///
    /// [`CLICK_EVENT`]: zng_ext_input::gesture::CLICK_EVENT
    #[property(EVENT)]
    pub fn on_any_triple_click<on_pre_any_triple_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_triple() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// On widget click with the primary button and any click count and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary), but raises for any click count (double/triple clicks).
    /// Use [`on_any_click`](fn@on_any_click) to handle clicks from any button or [`on_single_click`](fn@on_single_click) to not include
    /// double/triple clicks.
    #[property(EVENT)]
    pub fn on_click<on_pre_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }

    /// On widget click with the primary button, excluding double/triple clicks and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is one. Use
    /// [`on_any_single_click`](fn@on_any_single_click) to handle single clicks from any button.
    #[property(EVENT)]
    pub fn on_single_click<on_pre_single_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_single() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }

    /// On widget double click with the primary button and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is two. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    #[property(EVENT)]
    pub fn on_double_click<on_pre_double_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_double() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// On widget triple click with the primary button and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is three. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    #[property(EVENT)]
    pub fn on_triple_click<on_pre_triple_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_triple() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// On widget click with the secondary/context button and the widget is enabled.
    ///
    /// This raises only if the click [is context](ClickArgs::is_context).
    #[property(EVENT)]
    pub fn on_context_click<on_pre_context_click>(child: impl IntoUiNode, handler: Handler<ClickArgs>) -> UiNode {
        const PRE: bool;
        let child = EventNodeBuilder::new(CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_context() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler);
        access_click(child)
    }
}

/// Keyboard shortcuts that focus and clicks this widget.
///
/// When any of the `shortcuts` is pressed, focus and click this widget.
#[property(CONTEXT)]
pub fn click_shortcut(child: impl IntoUiNode, shortcuts: impl IntoVar<Shortcuts>) -> UiNode {
    click_shortcut_node(child, shortcuts, ShortcutClick::Primary)
}
/// Keyboard shortcuts that focus and [context clicks](fn@on_context_click) this widget.
///
/// When any of the `shortcuts` is pressed, focus and context clicks this widget.
#[property(CONTEXT)]
pub fn context_click_shortcut(child: impl IntoUiNode, shortcuts: impl IntoVar<Shortcuts>) -> UiNode {
    click_shortcut_node(child, shortcuts, ShortcutClick::Context)
}

fn click_shortcut_node(child: impl IntoUiNode, shortcuts: impl IntoVar<Shortcuts>, kind: ShortcutClick) -> UiNode {
    let shortcuts = shortcuts.into_var();
    let mut _handle = None;

    match_node(child, move |_, op| {
        let new = match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&shortcuts);
                Some(shortcuts.get())
            }
            UiNodeOp::Deinit => {
                _handle = None;
                None
            }
            UiNodeOp::Update { .. } => shortcuts.get_new(),
            _ => None,
        };
        if let Some(s) = new {
            _handle = Some(GESTURES.click_shortcut(s, kind, WIDGET.id()));
        }
    })
}

pub(crate) fn access_click(child: impl IntoUiNode) -> UiNode {
    access_capable(child, AccessCmdName::Click)
}
fn access_capable(child: impl IntoUiNode, cmd: AccessCmdName) -> UiNode {
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op
            && let Some(mut access) = info.access()
        {
            access.push_command(cmd)
        }
    })
}

/// Defines the mnemonic char key that clicks the widget when pressed and focus is within the parent mnemonic scope.
#[derive(Debug, PartialEq, Hash, Clone)]
pub enum Mnemonic {
    /// Scope selects a char using the inner [`mnemonic_txt`] of the widget or descendants.
    ///
    /// [`mnemonic_txt`]: fn@mnemonic_txt
    Auto,
    /// Explicit alphanumeric char.
    ///
    /// The associated char must be a value that can appear in [`Key::Char`] (case indifferent), otherwise it will never match.
    ///
    /// In case the same key is set for multiple widgets in a scope the first widget (in tab order) takes it, the others
    /// fallback to `Auto`.
    ///
    /// [`Key::Char`]: zng_ext_input::keyboard::Key
    Char(char),
    /// Explicit alphanumeric key defined in the widget inner text, identified by a `marker` prefix.
    ///
    /// After the char is extracted this behaves like `Char`. If the `marker` is not found also fallback to `Auto`.
    ///
    /// The `Label!` widget automatically hides the marker (first occurrence before an alphanumeric char).
    FromTxt {
        /// Char that is before the key char.
        ///
        /// If marker is `'_'` and the text is `"_Cut"` the mnemonic is `'c'`.
        marker: char,
    },
    /// No mnemonic behavior, disabled.
    None,
}
impl_from_and_into_var! {
    /// Converts to `Char`
    fn from(c: char) -> Mnemonic {
        Mnemonic::Char(c)
    }
    /// Converts `true` to [`from_txt`] and `false` to `None`.
    ///
    /// [`from_txt`]: Mnemonic::from_txt
    fn from(from_txt: bool) -> Mnemonic {
        if from_txt { Mnemonic::from_txt() } else { Mnemonic::None }
    }
}
impl Mnemonic {
    /// `FromTxt` with default marker `'_'`.
    pub fn from_txt() -> Self {
        Self::FromTxt { marker: '_' }
    }
}

/// Defines the mnemonic char key that clicks the widget when pressed and focus is within the parent mnemonic scope.
///
/// ```
/// # macro_rules! example { () => {
/// Stack! {
///     mnemonic_scope = true;
///     alt_focus_scope = true;
///     children = ui_vec![Button! {
///         mnemonic = true;
///         child = Label!("_Open File");
///     },];
/// }
/// # }}
/// ```
///
/// In the example above the `Button!` will be clicked when focus is within the parent `Stack!` and the `O` key is pressed.
///
/// Note that `true` converts into [`Mnemonic::FromTxt`] with `_` marker, and if no valid char is defined in the inner [`mnemonic_txt`]
/// text the behavior falls back to [`Mnemonic::Auto`], so a simple `mnemonic = true` enables the most common use case for this feature.
///
/// Note the use of `Label!` instead of `Text!`, the `Label!` widget automatically sets [`mnemonic_txt`], removes the markers
/// from the rendered text and marks the mnemonic char with an underline.
///
/// The `Menu!` and related widgets automatically enables mnemonic for inner buttons, but you still must use `Label!` instead of `Text!`.
///
/// [`mnemonic_scope`]: fn@mnemonic_scope
/// [`mnemonic_txt`]: fn@mnemonic_txt
#[property(CONTEXT, default(Mnemonic::None))]
pub fn mnemonic(child: impl IntoUiNode, mnemonic: impl IntoVar<Mnemonic>) -> UiNode {
    let mnemonic = mnemonic.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            info.set_meta(*MNEMONIC_ID, mnemonic.clone());
        }
    })
}

/// Defines the inner text of a [`mnemonic`] parent widget.
///
/// Note that the `Label!` widget automatically sets this to its own `txt`, this property can override the
/// inner text. The text is used when the widget or parent mnemonic is [`FromTxt`] or [`Auto`].
///
/// [`mnemonic`]: fn@mnemonic
/// [`FromTxt`]: Mnemonic::FromTxt
/// [`Auto`]: Mnemonic::Auto
#[property(CHILD, default(Txt::default()))]
pub fn mnemonic_txt(child: impl IntoUiNode, txt: impl IntoVar<Txt>) -> UiNode {
    let txt = txt.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            info.set_meta(*MNEMONIC_TXT_ID, txt.clone());
        }
    })
}

/// Defines a mnemonic shortcut scope.
///
/// When focus is within the scope widget a [`GESTURES.click_shortcut`] is set for each [`mnemonic`] descendant.
///
/// [`mnemonic`]: fn@mnemonic
/// [`GESTURES.click_shortcut`]: GESTURES::click_shortcut
#[property(CONTEXT, default(false))]
pub fn mnemonic_scope(child: impl IntoUiNode, is_scope: impl IntoVar<bool>) -> UiNode {
    let is_scope = is_scope.into_var();
    let mut init = false;
    let update = var(());
    let mut var_subs = VarHandles::dummy();
    let mut shortcut_subs = vec![];
    let mut is_focus_within = false;
    let active_mnemonics = var(HashMap::new());
    let child = with_context_var(child, ACTIVE_MNEMONICS_VAR, active_mnemonics.read_only());
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&is_scope).sub_var(&update);
        }
        UiNodeOp::Deinit => {
            init = false;
            var_subs = VarHandles::dummy();
            shortcut_subs = vec![];
            is_focus_within = false;
            active_mnemonics.set(HashMap::new());
        }
        UiNodeOp::Info { info } => {
            if is_scope.get() {
                info.flag_meta(*MNEMONIC_SCOPE_ID);
            }
            init = true;
            WIDGET.update();
        }
        UiNodeOp::Update { .. } => {
            let mut set_shortcuts = false;
            if mem::take(&mut init) {
                var_subs.clear();
                shortcut_subs.clear();

                if is_scope.get() {
                    // sub to is_focus_within
                    let id = WIDGET.id();
                    var_subs.push(
                        FOCUS_CHANGED_EVENT.subscribe_when(UpdateOp::Update, id, move |a| a.is_focus_enter(id) || a.is_focus_leave(id)),
                    );
                    is_focus_within = FOCUS.focused().with(|f| matches!(f, Some(f) if f.contains(id)));
                    set_shortcuts = is_focus_within;

                    // sub to each descendant mnemonic properties
                    let mut var_sub = |v: &AnyVar| {
                        let update_wk = update.downgrade();
                        var_subs.push(v.hook(move |_| match update_wk.upgrade() {
                            Some(u) => {
                                u.update();
                                true
                            }
                            None => false,
                        }));
                    };
                    for d in WIDGET.info().self_and_descendants() {
                        if let Some(m) = d.meta().get(*MNEMONIC_ID) {
                            // descendant sets `mnemonic`, subscribe
                            var_sub(m.as_any());
                        }
                        if let Some(t) = d.meta().get(*MNEMONIC_TXT_ID) {
                            // descendant sets `mnemonic_txt`, subscribe
                            var_sub(t.as_any());
                        }
                    }
                }
            } else if is_scope.get() {
                // else if is inited and enabled, check is_focus_within change
                let id = WIDGET.id();
                FOCUS_CHANGED_EVENT.each_update(true, |a| {
                    // !!: don't activate if focus is in an inner scope
                    if a.is_focus_enter(id) {
                        is_focus_within = true;
                        set_shortcuts = true;
                    } else if a.is_focus_leave(id) {
                        is_focus_within = false;
                        shortcut_subs.clear();
                    }
                });
            }

            if is_focus_within && (set_shortcuts || update.is_new()) {
                // focus entered OR inited and is focus within OR is focus within and descendant state changed

                shortcut_subs.clear();

                let mut chars = HashMap::new();
                let mut auto = vec![];
                let info = WIDGET.info();
                let scope_and_descendants = info.self_and_descendants().tree_filter(|w| {
                    if w != &info && w.is_mnemonic_scope() {
                        TreeFilter::SkipAll
                    } else {
                        TreeFilter::Include
                    }
                });
                for d in scope_and_descendants {
                    if let Some(m) = d.mnemonic() {
                        // descendant sets `mnemonic`
                        let mut m = m.get();

                        // extract ::Char from inner text
                        if let Mnemonic::FromTxt { marker } = m {
                            // fallback state
                            m = Mnemonic::Auto;

                            let mnemonic_and_descendants = d.self_and_descendants().tree_filter(|w| {
                                if w != &d && d.is_mnemonic_scope() || d.mnemonic().is_some() {
                                    TreeFilter::SkipAll
                                } else {
                                    TreeFilter::Include
                                }
                            });
                            for d in mnemonic_and_descendants {
                                if let Some(txt) = d.mnemonic_txt() {
                                    let c = txt.with(|txt| {
                                        let mut return_next = false;
                                        for c in txt.chars() {
                                            if return_next {
                                                return Some(c);
                                            }
                                            return_next = c == marker;
                                        }
                                        None
                                    });
                                    if let Some(c) = c {
                                        m = Mnemonic::Char(c);
                                        break;
                                    }
                                }
                            }
                        }

                        // validate and register ::Char
                        if let Mnemonic::Char(c) = m {
                            if c.is_alphanumeric() {
                                match chars.entry(c.to_lowercase().collect::<Txt>()) {
                                    hash_map::Entry::Vacant(e) => {
                                        // valid char
                                        e.insert((d.id(), c));
                                        m = Mnemonic::None;
                                    }
                                    hash_map::Entry::Occupied(e) => {
                                        tracing::error!("both {:?} and {:?} set the same mnemonic {:?}", e.get().0, d.id(), c);
                                        m = Mnemonic::Auto;
                                    }
                                }
                            } else {
                                tracing::error!("char `{c:?}` cannot be a mnemonic, not alphanumeric");
                                m = Mnemonic::Auto;
                            }
                        }

                        // collect ::Auto
                        if let Mnemonic::Auto = m {
                            auto.push(d);
                        }
                    }
                }
                // generate best char for ::Auto
                for d in auto {
                    let mut found_txt = false;

                    let mnemonic_and_descendants = d.self_and_descendants().tree_filter(|w| {
                        if w != &d && d.is_mnemonic_scope() || d.mnemonic().is_some() {
                            TreeFilter::SkipAll
                        } else {
                            TreeFilter::Include
                        }
                    });
                    for d in mnemonic_and_descendants {
                        if let Some(txt) = d.mnemonic_txt() {
                            found_txt = true;

                            txt.with(|t| {
                                // try uppercase chars first
                                for c in t.chars() {
                                    if c.is_alphanumeric()
                                        && c.is_uppercase()
                                        && let hash_map::Entry::Vacant(e) = chars.entry(c.to_lowercase().collect::<Txt>())
                                    {
                                        e.insert((d.id(), c));
                                        return;
                                    }
                                }
                                // try other alphanumeric chars
                                for c in t.chars() {
                                    if c.is_alphanumeric()
                                        && !c.is_uppercase()
                                        && let hash_map::Entry::Vacant(e) = chars.entry(Txt::from_char(c))
                                    {
                                        e.insert((d.id(), c));
                                        return;
                                    }
                                }
                            });
                            break;
                        }
                    }
                    if !found_txt {
                        tracing::warn!(
                            "no mnemonic selected for {:?}, no `mnemonic_txt` set on it or descendants, consider using `Label!` for the inner text",
                            d.id()
                        );
                    }
                }
                // register shortcuts
                for (_, (id, c)) in chars.iter() {
                    let h = GESTURES.click_shortcut(GestureKey::Key(Key::Char(*c)), ShortcutClick::Primary, *id);
                    shortcut_subs.push(h);
                }
                active_mnemonics.modify(move |m| {
                    m.clear();
                    for (_, (id, c)) in chars {
                        m.insert(id, c);
                    }
                });
            }
        }
        _ => {}
    })
}

/// Get the active mnemonic shortcut char for this widget or ancestor.
///
/// If this widget or ancestor enables [`mnemonic`] the `state` is set to the selected mnemonic char when focus is within
/// the parent [`mnemonic_scope`].
///
/// [`mnemonic`]: fn@mnemonic
/// [`mnemonic_scope`]: fn@mnemonic_scope
#[property(WIDGET_INNER)]
pub fn get_mnemonic_char(child: impl IntoUiNode, state: impl IntoVar<Option<char>>) -> UiNode {
    bind_state_info(child, state, move |s| {
        let info = WIDGET.info();
        for w in info.self_and_ancestors() {
            let found_scope = w.is_mnemonic_scope();
            if found_scope && w != info {
                break;
            }

            if w.mnemonic().is_some() {
                let id = w.id();
                return ACTIVE_MNEMONICS_VAR.bind_map(s, move |m| m.get(&id).copied());
            }

            if found_scope {
                break;
            }
        }
        VarHandle::dummy()
    })
}

/// Gets the mnemonic mode enabled for this widget or ancestor.
///
/// If this widget or ancestor enables [`mnemonic`] the `state` is set to the mnemonic mode.
///
/// [`mnemonic`]: fn@mnemonic
#[property(WIDGET_INNER, default(var(Mnemonic::None)))]
pub fn get_mnemonic(child: impl IntoUiNode, state: impl IntoVar<Mnemonic>) -> UiNode {
    bind_state_info(child, state, move |s| {
        let info = WIDGET.info();
        for w in info.self_and_ancestors() {
            let found_scope = w.is_mnemonic_scope();
            if found_scope && w != info {
                break;
            }

            if let Some(m) = w.mnemonic() {
                return m.bind(s);
            }

            if found_scope {
                break;
            }
        }
        VarHandle::dummy()
    })
}

static_id! {
    static ref MNEMONIC_SCOPE_ID: StateId<()>;
    static ref MNEMONIC_ID: StateId<Var<Mnemonic>>;
    static ref MNEMONIC_TXT_ID: StateId<Var<Txt>>;
}

context_var! {
    /// Inside an active [`mnemonic_scope`] this context var is a read-only map of the selected `char` for each descendant of the scope.
    ///
    /// [`mnemonic_scope`]: fn@mnemonic_scope
    pub static ACTIVE_MNEMONICS_VAR: HashMap<WidgetId, char> = HashMap::new();
}

/// Extension methods for widget info about mnemonic metadata.
pub trait MnemonicWidgetInfoExt {
    /// If [`mnemonic_scope`] is enabled in the widget.
    ///
    /// [`mnemonic_scope`]: fn@mnemonic_scope
    fn is_mnemonic_scope(&self) -> bool;
    /// Reference the [`mnemonic`] set on this widget.
    ///
    /// [`mnemonic`]: fn@mnemonic
    fn mnemonic(&self) -> Option<&Var<Mnemonic>>;

    /// Reference the [`mnemonic_txt`] set on this widget.
    ///
    /// [`mnemonic_txt`]: fn@mnemonic_txt
    fn mnemonic_txt(&self) -> Option<&Var<Txt>>;
}
impl MnemonicWidgetInfoExt for WidgetInfo {
    fn is_mnemonic_scope(&self) -> bool {
        self.meta().flagged(*MNEMONIC_SCOPE_ID)
    }

    fn mnemonic(&self) -> Option<&Var<Mnemonic>> {
        self.meta().get(*MNEMONIC_ID)
    }

    fn mnemonic_txt(&self) -> Option<&Var<Txt>> {
        self.meta().get(*MNEMONIC_TXT_ID)
    }
}
