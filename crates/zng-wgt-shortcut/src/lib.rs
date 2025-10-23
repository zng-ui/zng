#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Shortcut display widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_app::shortcut::{GestureKey, KeyGesture, ModifierGesture, Shortcut};
use zng_view_api::keyboard::Key;
use zng_wgt::prelude::*;
use zng_wgt_wrap::Wrap;

/// Display keyboard shortcuts.
#[widget($crate::ShortcutText {
    ($shortcut:expr) => {
        shortcut = $shortcut;
    }
})]
pub struct ShortcutText(WidgetBase);

impl ShortcutText {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let s = wgt.capture_var_or_default::<Shortcuts>(property_id!(shortcut));
            wgt.set_child(node(s));
        });
    }

    widget_impl! {
        /// Font size of the shortcut text.
        pub zng_wgt_text::font_size(size: impl IntoVar<zng_ext_font::FontSize>);
        /// Font color of the shortcut text.
        pub zng_wgt_text::font_color(color: impl IntoVar<Rgba>);
    }
}

/// Shortcut(s)  that must be displayed
#[property(CHILD, widget_impl(ShortcutText))]
pub fn shortcut(wgt: &mut WidgetBuilding, shortcuts: impl IntoVar<Shortcuts>) {
    let _ = shortcuts;
    wgt.expect_property_capture();
}

context_var! {
    /// Maximum number of shortcuts to display.
    ///
    /// Is `1` by default.
    pub static FIRST_N_VAR: usize = 1;

    /// Widget function that generates the outer panel.
    pub static PANEL_FN_VAR: WidgetFn<PanelFnArgs> = WidgetFn::new(default_panel_fn);

    /// Widget function that generates the separator between shortcuts.
    pub static SHORTCUTS_SEPARATOR_FN_VAR: WidgetFn<ShortcutsSeparatorFnArgs> = WidgetFn::new(default_shortcuts_separator_fn);

    /// Widget function that generates a shortcut panel.
    pub static SHORTCUT_FN_VAR: WidgetFn<ShortcutFnArgs> = WidgetFn::nil();

    /// Widget function that generates the separator between key gestures in chord shortcuts.
    pub static CHORD_SEPARATOR_FN_VAR: WidgetFn<ChordSeparatorFnArgs> = WidgetFn::new(default_chord_separator_fn);

    /// Widget function that generates the modifier view.
    pub static MODIFIER_FN_VAR: WidgetFn<ModifierFnArgs> = WidgetFn::new(default_modifier_fn);

    /// Widget function that generates the key gesture panel.
    pub static KEY_GESTURE_FN_VAR: WidgetFn<KeyGestureFnArgs> = WidgetFn::nil();

    /// Widget function that generates the separators between modifiers and keys in a key gesture.
    pub static KEY_GESTURE_SEPARATOR_FN_VAR: WidgetFn<KeyGestureSeparatorFnArgs> = WidgetFn::new(default_key_gesture_separator_fn);

    /// Widget function that generates the key view.
    pub static KEY_FN_VAR: WidgetFn<KeyFnArgs> = WidgetFn::new(default_key_fn);

    /// Widget function that generates content when there is no gesture to display.
    pub static NONE_FN_VAR: WidgetFn<NoneFnArgs> = WidgetFn::nil();
}

/// Maximum number of shortcuts to display.
///
/// This property sets the [`FIRST_N_VAR`].
#[property(CONTEXT, default(FIRST_N_VAR), widget_impl(ShortcutText))]
pub fn first_n(child: impl IntoUiNode, n: impl IntoVar<usize>) -> UiNode {
    with_context_var(child, FIRST_N_VAR, n)
}

/// Widget function that converts [`PanelFnArgs`] to a widget.
///
/// This property sets the [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(ShortcutText))]
pub fn panel_fn(child: impl IntoUiNode, panel_fn: impl IntoVar<WidgetFn<PanelFnArgs>>) -> UiNode {
    with_context_var(child, PANEL_FN_VAR, panel_fn)
}

/// Widget function that converts [`NoneFnArgs`] to a widget.
///
/// This property sets the [`NONE_FN_VAR`].
#[property(CONTEXT, default(NONE_FN_VAR), widget_impl(ShortcutText))]
pub fn none_fn(child: impl IntoUiNode, none_fn: impl IntoVar<WidgetFn<NoneFnArgs>>) -> UiNode {
    with_context_var(child, NONE_FN_VAR, none_fn)
}

/// Widget function that converts [`ShortcutsSeparatorFnArgs`] to a widget.
///
/// This is the separators between shortcuts, when [`first_n`] is more than one and the [`shortcut`] has mode them one shortcut.
///
/// Set to nil to not generate a separator.
///
/// This property sets the [`SHORTCUTS_SEPARATOR_FN_VAR`].
///
/// [`first_n`]: fn@first_n
/// [`shortcut`]: fn@shortcut
#[property(CONTEXT, default(SHORTCUTS_SEPARATOR_FN_VAR), widget_impl(ShortcutText))]
pub fn shortcuts_separator_fn(child: impl IntoUiNode, separator_fn: impl IntoVar<WidgetFn<ShortcutsSeparatorFnArgs>>) -> UiNode {
    with_context_var(child, SHORTCUTS_SEPARATOR_FN_VAR, separator_fn)
}

/// Widget function that converts [`ShortcutFnArgs`] to a widget.
///
/// Set to [`WidgetFn::nil`] or return the items as a node to pass the items directly to [`panel_fn`].
///
/// This property sets the [`SHORTCUT_FN_VAR`].
///
/// [`panel_fn`]: fn@panel_fn
#[property(CONTEXT, default(SHORTCUT_FN_VAR), widget_impl(ShortcutText))]
pub fn shortcut_fn(child: impl IntoUiNode, panel_fn: impl IntoVar<WidgetFn<ShortcutFnArgs>>) -> UiNode {
    with_context_var(child, SHORTCUT_FN_VAR, panel_fn)
}

/// Widget function that converts [`ChordSeparatorFnArgs`] to a widget.
///
/// This is the separator between the starter and complement in a [`KeyChord`].
///
/// This property sets the [`CHORD_SEPARATOR_FN_VAR`].
///
/// [`KeyChord`]: zng_app::shortcut::KeyChord
#[property(CONTEXT, default(CHORD_SEPARATOR_FN_VAR), widget_impl(ShortcutText))]
pub fn chord_separator_fn(child: impl IntoUiNode, separator_fn: impl IntoVar<WidgetFn<ChordSeparatorFnArgs>>) -> UiNode {
    with_context_var(child, CHORD_SEPARATOR_FN_VAR, separator_fn)
}

/// Widget function that converts [`KeyGestureFnArgs`] to a widget.
///
/// Set to [`WidgetFn::nil`] or return the items as a node to pass the items directly to [`shortcut_fn`].
///
/// This property sets the [`KEY_GESTURE_FN_VAR`].
///
/// [`shortcut_fn`]: fn@shortcut_fn
#[property(CONTEXT, default(KEY_GESTURE_FN_VAR), widget_impl(ShortcutText))]
pub fn key_gesture_fn(child: impl IntoUiNode, panel_fn: impl IntoVar<WidgetFn<KeyGestureFnArgs>>) -> UiNode {
    with_context_var(child, KEY_GESTURE_FN_VAR, panel_fn)
}

/// Widget function that converts [`KeyGestureSeparatorFnArgs`] to a widget.
///
/// This is the separator between the modifiers and key in a [`KeyGesture`].
///
/// This property sets the [`KEY_GESTURE_SEPARATOR_FN_VAR`].
#[property(CONTEXT, default(KEY_GESTURE_SEPARATOR_FN_VAR), widget_impl(ShortcutText))]
pub fn key_gesture_separator_fn(child: impl IntoUiNode, separator_fn: impl IntoVar<WidgetFn<KeyGestureSeparatorFnArgs>>) -> UiNode {
    with_context_var(child, KEY_GESTURE_SEPARATOR_FN_VAR, separator_fn)
}

/// Widget function that converts a [`ModifierFnArgs`] to a widget.
///
/// This is used for both the [`Shortcut::Modifier`] standalone and the [`KeyGesture::modifiers`] flags.
///
/// This property sets the [`MODIFIER_FN_VAR`].
#[property(CONTEXT, default(MODIFIER_FN_VAR), widget_impl(ShortcutText))]
pub fn modifier_fn(child: impl IntoUiNode, modifier_fn: impl IntoVar<WidgetFn<ModifierFnArgs>>) -> UiNode {
    with_context_var(child, MODIFIER_FN_VAR, modifier_fn)
}

/// Widget function that converts a [`KeyFnArgs`] to a widget.
///  
/// This property sets the [`KEY_FN_VAR`].
#[property(CONTEXT, default(KEY_FN_VAR), widget_impl(ShortcutText))]
pub fn key_fn(child: impl IntoUiNode, key_fn: impl IntoVar<WidgetFn<KeyFnArgs>>) -> UiNode {
    with_context_var(child, KEY_FN_VAR, key_fn)
}

/// Arguments for [`panel_fn`].
///
/// [`panel_fn`]: fn@panel_fn
#[non_exhaustive]
pub struct PanelFnArgs {
    /// Shortcut and shortcut separator items.
    pub items: UiVec,

    /// If the single item in `items` is the [`none_fn`].
    ///
    /// [`none_fn`]: fn@none_fn
    pub is_none: bool,

    /// The shortcuts that where used to generate the `items`.
    pub shortcuts: Shortcuts,
}

/// Arguments for [`none_fn`].
///
/// [`none_fn`]: fn@none_fn
#[non_exhaustive]
pub struct NoneFnArgs {}

/// Arguments for [`shortcuts_separator_fn`].
///
/// [`shortcuts_separator_fn`]: fn@shortcuts_separator_fn
#[non_exhaustive]
pub struct ShortcutsSeparatorFnArgs {}

/// Arguments for [`shortcut_fn`].
///
/// [`shortcut_fn`]: fn@shortcut_fn
#[non_exhaustive]
pub struct ShortcutFnArgs {
    /// Modifier, key and separator items.
    pub items: UiVec,
    /// The shortcut.
    ///
    /// The `items` where instantiated from components of this shortcut.
    pub shortcut: Shortcut,
}

/// Arguments for [`chord_separator_fn`].
///
/// [`chord_separator_fn`]: fn@chord_separator_fn
#[non_exhaustive]
pub struct ChordSeparatorFnArgs {}

/// Arguments for [`key_gesture_fn`].
///
/// [`key_gesture_fn`]: fn@key_gesture_fn
#[non_exhaustive]
pub struct KeyGestureFnArgs {
    /// Modifier, key and separator items.
    pub items: UiVec,
    /// The key gesture.
    ///
    /// The `items` where instantiated from components of this gesture.
    pub gesture: KeyGesture,
}

/// Arguments for [`modifier_fn`].
///
/// [`modifier_fn`]: fn@modifier_fn
#[non_exhaustive]
pub struct ModifierFnArgs {
    /// The modifier.
    pub modifier: ModifierGesture,
    /// If is actually the [`Shortcut::Modifier`] press and release gesture.
    ///
    /// If `false` is actually a [`ModifiersState`] flag extracted from [`KeyGesture::modifiers`].
    ///
    /// [`ModifiersState`]: zng_app::shortcut::ModifiersState
    pub is_standalone: bool,
}

/// Arguments for [`key_fn`].
///
/// [`key_fn`]: fn@key_fn
pub struct KeyFnArgs {
    /// The key.
    pub key: GestureKey,
}
impl KeyFnArgs {
    /// If the `key` is an invalid value that indicates a editing shortcut.
    ///
    /// Widget function should return an invisible, but not collapsed blank space, recommended `Text!(" ")` without any style.
    pub fn is_editing_blank(&self) -> bool {
        matches!(&self.key, GestureKey::Key(Key::Unidentified))
    }
}

/// Arguments for [`key_gesture_separator_fn`].
///
/// [`key_gesture_separator_fn`]: fn@key_gesture_separator_fn
#[non_exhaustive]
pub struct KeyGestureSeparatorFnArgs {
    /// If the separator will be placed between two modifiers.
    ///
    /// When this is `false` the separator is placed between a modifier and the key.
    pub between_modifiers: bool,
}

/// Default value for [`PANEL_FN_VAR`].
///
/// For zero items returns nil, for one item just returns the item, for more returns a `Wrap!`.
pub fn default_panel_fn(mut args: PanelFnArgs) -> UiNode {
    match args.items.len() {
        0 => UiNode::nil(),
        1 => args.items.remove(0),
        _ => Wrap!(args.items),
    }
}

/// Default value for [`SHORTCUTS_SEPARATOR_FN_VAR`].
///
/// Returns `Text!(" or ")`.
pub fn default_shortcuts_separator_fn(_: ShortcutsSeparatorFnArgs) -> UiNode {
    zng_wgt_text::Text!(" or ")
}

/// Default value for [`CHORD_SEPARATOR_FN_VAR`].
///
/// Returns `Text!(", ")`.
pub fn default_chord_separator_fn(_: ChordSeparatorFnArgs) -> UiNode {
    zng_wgt_text::Text!(", ")
}

/// Default value for [`KEY_GESTURE_SEPARATOR_FN_VAR`].
///
/// Returns `Text!("+")`.
pub fn default_key_gesture_separator_fn(_: KeyGestureSeparatorFnArgs) -> UiNode {
    zng_wgt_text::Text!("+")
}

/// Default value for [`MODIFIER_FN_VAR`].
///
/// Returns a [`keycap`] with the [`modifier_txt`].
pub fn default_modifier_fn(args: ModifierFnArgs) -> UiNode {
    keycap(modifier_txt(args.modifier), args.is_standalone)
}

/// Default value for [`KEY_FN_VAR`].
///
/// Returns a [`keycap`] with the [`key_txt`].
pub fn default_key_fn(args: KeyFnArgs) -> UiNode {
    if args.is_editing_blank() {
        zng_wgt_text::Text!(" ")
    } else {
        keycap(key_txt(args.key), false)
    }
}

/// Widget used b the [`default_modifier_fn`] and [`default_key_fn`] to render a `Text!` styled to look like a keycap.
pub fn keycap(txt: Var<Txt>, is_standalone_modifier: bool) -> UiNode {
    zng_wgt_text::Text! {
        txt;
        font_family = ["Consolas", "Lucida Console", "monospace"];
        zng_wgt::border = {
            widths: if is_standalone_modifier { 0.2 } else { 0.08 }.em().max(1.dip()),
            sides: expr_var! {
                let base = *#{zng_wgt_text::FONT_COLOR_VAR};
                let color = match #{zng_color::COLOR_SCHEME_VAR} {
                    ColorScheme::Dark => colors::BLACK.with_alpha(70.pct()).mix_normal(base),
                    ColorScheme::Light => colors::WHITE.with_alpha(70.pct()).mix_normal(base),
                    _ => base.with_alpha(30.pct()),
                };
                BorderSides::new_all((
                    color,
                    if is_standalone_modifier {
                        BorderStyle::Double
                    } else {
                        BorderStyle::Solid
                    },
                ))
            },
        };
        zng_wgt_fill::background_color = zng_color::COLOR_SCHEME_VAR.map(|c| match c {
            ColorScheme::Dark => colors::BLACK,
            ColorScheme::Light => colors::WHITE,
            _ => zng_color::colors::BLACK.with_alpha(100.pct()),
        });
        zng_wgt::corner_radius = 0.2.em();
        txt_align = Align::START;
        zng_wgt::align = Align::START;
        zng_wgt_container::padding = (0, 0.20.em(), -0.15.em(), 0.20.em());
        zng_wgt::margin = (0, 0, -0.10.em(), 0);
    }
}

fn node(shortcut: Var<Shortcuts>) -> UiNode {
    match_node(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&shortcut)
                .sub_var(&PANEL_FN_VAR)
                .sub_var(&SHORTCUTS_SEPARATOR_FN_VAR)
                .sub_var(&SHORTCUT_FN_VAR)
                .sub_var(&MODIFIER_FN_VAR)
                .sub_var(&CHORD_SEPARATOR_FN_VAR)
                .sub_var(&KEY_FN_VAR)
                .sub_var(&KEY_GESTURE_SEPARATOR_FN_VAR)
                .sub_var(&KEY_GESTURE_FN_VAR)
                .sub_var(&FIRST_N_VAR)
                .sub_var(&NONE_FN_VAR);
            *c.node() = generate(shortcut.get());
            c.init();
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { updates } => {
            if shortcut.is_new()
                || PANEL_FN_VAR.is_new()
                || SHORTCUTS_SEPARATOR_FN_VAR.is_new()
                || SHORTCUT_FN_VAR.is_new()
                || MODIFIER_FN_VAR.is_new()
                || CHORD_SEPARATOR_FN_VAR.is_new()
                || KEY_FN_VAR.is_new()
                || KEY_GESTURE_SEPARATOR_FN_VAR.is_new()
                || KEY_GESTURE_FN_VAR.is_new()
                || FIRST_N_VAR.is_new()
                || NONE_FN_VAR.is_new()
            {
                c.deinit();
                *c.node() = generate(shortcut.get());
                c.init();
            } else {
                c.update(updates);
            }
        }
        _ => {}
    })
}

fn generate(mut shortcut: Shortcuts) -> UiNode {
    let panel_fn = PANEL_FN_VAR.get();
    let shortcuts_separator_fn = SHORTCUTS_SEPARATOR_FN_VAR.get();
    let shortcut_fn = SHORTCUT_FN_VAR.get();
    let modifier_fn = MODIFIER_FN_VAR.get();
    let chord_separator_fn = CHORD_SEPARATOR_FN_VAR.get();
    let separator_fn = KEY_GESTURE_SEPARATOR_FN_VAR.get();
    let gesture_fn = KEY_GESTURE_FN_VAR.get();
    let key_fn = KEY_FN_VAR.get();
    let first_n = FIRST_N_VAR.get();

    shortcut.truncate(first_n);

    let mut items = ui_vec![];
    for shortcut in shortcut.iter() {
        if !items.is_empty()
            && let Some(sep) = shortcuts_separator_fn.call_checked(ShortcutsSeparatorFnArgs {})
        {
            items.push(sep);
        }

        fn gesture(
            out: &mut UiVec,
            gesture: KeyGesture,
            separator_fn: &WidgetFn<KeyGestureSeparatorFnArgs>,
            modifier_fn: &WidgetFn<ModifierFnArgs>,
            key_fn: &WidgetFn<KeyFnArgs>,
            gesture_fn: &WidgetFn<KeyGestureFnArgs>,
        ) {
            let mut gesture_items = ui_vec![];

            macro_rules! gen_modifier {
                ($has:ident, $Variant:ident) => {
                    if gesture.modifiers.$has()
                        && let Some(n) = modifier_fn.call_checked(ModifierFnArgs {
                            modifier: ModifierGesture::$Variant,
                            is_standalone: false,
                        })
                    {
                        if !gesture_items.is_empty()
                            && let Some(s) = separator_fn.call_checked(KeyGestureSeparatorFnArgs {
                                between_modifiers: true,
                            })
                        {
                            gesture_items.push(s)
                        }
                        gesture_items.push(n);
                    }
                };
            }
            gen_modifier!(has_super, Super);
            gen_modifier!(has_ctrl, Ctrl);
            gen_modifier!(has_shift, Shift);
            gen_modifier!(has_alt, Alt);

            if let Some(n) = key_fn.call_checked(KeyFnArgs { key: gesture.key.clone() }) {
                if !gesture_items.is_empty()
                    && let Some(s) = separator_fn.call_checked(KeyGestureSeparatorFnArgs { between_modifiers: false })
                {
                    gesture_items.push(s);
                }
                gesture_items.push(n);
            }

            if gesture_fn.is_nil() {
                out.append(&mut gesture_items);
            } else {
                let gesture = gesture_fn.call(KeyGestureFnArgs {
                    items: gesture_items,
                    gesture,
                });
                out.push(gesture);
            }
        }

        let mut shortcut_items = ui_vec![];
        match shortcut {
            Shortcut::Gesture(g) => gesture(&mut shortcut_items, g.clone(), &separator_fn, &modifier_fn, &key_fn, &gesture_fn),
            Shortcut::Chord(c) => {
                gesture(
                    &mut shortcut_items,
                    c.starter.clone(),
                    &separator_fn,
                    &modifier_fn,
                    &key_fn,
                    &gesture_fn,
                );
                if !shortcut_items.is_empty()
                    && let Some(s) = chord_separator_fn.call_checked(ChordSeparatorFnArgs {})
                {
                    shortcut_items.push(s);
                }
                gesture(
                    &mut shortcut_items,
                    c.complement.clone(),
                    &separator_fn,
                    &modifier_fn,
                    &key_fn,
                    &gesture_fn,
                );
            }
            Shortcut::Modifier(g) => {
                if let Some(m) = modifier_fn.call_checked(ModifierFnArgs {
                    modifier: *g,
                    is_standalone: true,
                }) {
                    shortcut_items.push(m);
                }
            }
        }
        if shortcut_fn.is_nil() {
            items.append(&mut shortcut_items);
        } else {
            let mut s = shortcut_fn.call(ShortcutFnArgs {
                items: shortcut_items,
                shortcut: shortcut.clone(),
            });
            if let Some(flat) = s.downcast_mut::<UiVec>() {
                items.append(flat);
            } else {
                items.push(s);
            }
        }
    }

    let mut is_none = false;
    if items.is_empty() {
        let none_fn = NONE_FN_VAR.get();
        if let Some(n) = none_fn.call_checked(NoneFnArgs {}) {
            items.push(n);
            is_none = true;
        }
    }

    panel_fn.call(PanelFnArgs {
        items,
        is_none,
        shortcuts: shortcut,
    })
}

mod l10n_helper {
    use super::*;
    use zng_ext_l10n::*;

    fn path(file: &'static str) -> LangFilePath {
        LangFilePath::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION").parse().unwrap(), file)
    }

    pub fn l10n(file: &'static str, name: &'static str) -> Var<Txt> {
        let path = path(file);
        let os_msg = L10N.message(path.clone(), name, std::env::consts::OS, "!FALLBACK").build();
        let generic_msg = L10N.message(path, name, "", name).build();
        l10n_expr(os_msg, generic_msg)
    }

    pub fn os_or(file: &'static str, name: &'static str, generic: Var<Txt>) -> Var<Txt> {
        let path = path(file);
        let os_msg = L10N.message(path.clone(), name, std::env::consts::OS, "!FALLBACK").build();
        l10n_expr(os_msg, generic)
    }

    fn l10n_expr(os_msg: Var<Txt>, generic_msg: Var<Txt>) -> Var<Txt> {
        expr_var! {
            let os_msg = #{os_msg};
            if os_msg == "!FALLBACK" {
                #{generic_msg}.clone()
            } else {
                os_msg.clone()
            }
        }
    }
}

/// Gets the localized modifier name.
pub fn modifier_txt(modifier: ModifierGesture) -> Var<Txt> {
    // l10n-modifiers-### Modifier key names
    // l10n-modifiers-###
    // l10n-modifiers-### * The ID is the `ModifierGesture` variant name. [1]
    // l10n-modifiers-### * An OS generic text must be provided, optional OS specific text can be set as attributes.
    // l10n-modifiers-### * OS attribute is a `std::env::consts::OS` value. [2]
    // l10n-modifiers-###
    // l10n-modifiers-### [1]: https://zng-ui.github.io/doc/zng/gesture/enum.ModifierGesture.html
    // l10n-modifiers-### [2]: https://doc.rust-lang.org/std/env/consts/constant.OS.html
    use zng_ext_l10n::*;
    match modifier {
        ModifierGesture::Super => match std::env::consts::OS {
            "windows" => l10n!("modifiers/Super.windows", "⊞Win"),
            "macos" => l10n!("modifiers/Super.macos", "⌘Command"),
            _ => l10n_helper::os_or("modifiers", "Super", l10n!("modifiers/Super", "Super")),
        },
        ModifierGesture::Ctrl => match std::env::consts::OS {
            "macos" => l10n!("modifiers/Ctrl.macos", "^Control"),
            _ => l10n_helper::os_or("modifiers", "Ctrl", l10n!("modifiers/Ctrl", "Ctrl")),
        },
        ModifierGesture::Shift => l10n_helper::os_or("modifiers", "Shift", l10n!("modifiers/Shift", "⇧Shift")),
        ModifierGesture::Alt => match std::env::consts::OS {
            "macos" => l10n!("modifiers/Alt.macos", "⌥Option"),
            _ => l10n_helper::os_or("modifiers", "Alt", l10n!("modifiers/Alt", "Alt")),
        },
    }
}

/// Gets the localized key name.
pub fn key_txt(key: GestureKey) -> Var<Txt> {
    // l10n-keys-### Valid gesture key names
    // l10n-keys-###
    // l10n-keys-### * The ID is the `Key` variant name. [1]
    // l10n-keys-### * An OS generic text must be provided, optional OS specific text can be set as attributes.
    // l10n-keys-### * OS attribute is a `std::env::consts::OS` value. [2]
    // l10n-keys-### * L10n not include Char, Str, modifiers and composite keys.
    // l10n-keys-###
    // l10n-keys-### Note: This file does not include all valid keys, see [1] for a full list.
    // l10n-keys-###
    // l10n-keys-### [1]: https://zng-ui.github.io/doc/zng/keyboard/enum.Key.html
    // l10n-keys-### [2]: https://doc.rust-lang.org/std/env/consts/constant.OS.html

    use zng_ext_l10n::*;
    if !key.is_valid() {
        return const_var(Txt::from_static(""));
    }
    match key {
        GestureKey::Key(key) => match key {
            Key::Char(c) => c.to_uppercase().to_txt().into_var(),
            Key::Str(s) => s.into_var(),
            Key::Enter => match std::env::consts::OS {
                "macos" => l10n!("keys/Enter.macos", "↵Return"),
                _ => l10n_helper::os_or("keys", "Enter", l10n!("keys/Enter", "↵Enter")),
            },
            Key::Backspace => match std::env::consts::OS {
                "macos" => l10n!("keys/Backspace.macos", "Delete"),
                _ => l10n_helper::os_or("keys", "Backspace", l10n!("keys/Backspace", "←Backspace")),
            },
            Key::Delete => match std::env::consts::OS {
                "macos" => l10n!("keys/Delete.macos", "Forward Delete"),
                _ => l10n_helper::os_or("keys", "Delete", l10n!("keys/Delete", "Delete")),
            },
            Key::Tab => l10n_helper::os_or("keys", "Tab", l10n!("keys/Tab", "⭾Tab")),
            Key::ArrowDown => l10n_helper::os_or("keys", "ArrowDown", l10n!("keys/ArrowDown", "↓")),
            Key::ArrowLeft => l10n_helper::os_or("keys", "ArrowLeft", l10n!("keys/ArrowLeft", "←")),
            Key::ArrowRight => l10n_helper::os_or("keys", "ArrowRight", l10n!("keys/ArrowRight", "→")),
            Key::ArrowUp => l10n_helper::os_or("keys", "ArrowUp", l10n!("keys/ArrowUp", "↑")),
            Key::PageDown => l10n_helper::os_or("keys", "PageDown", l10n!("keys/PageDown", "PgDn")),
            Key::PageUp => l10n_helper::os_or("keys", "PageUp", l10n!("keys/PageUp", "PgUp")),
            Key::Cut => l10n_helper::os_or("keys", "Cut", l10n!("keys/Cut", "Cut")),
            Key::Copy => l10n_helper::os_or("keys", "Copy", l10n!("keys/Copy", "Copy")),
            Key::Paste => l10n_helper::os_or("keys", "Paste", l10n!("keys/Paste", "Paste")),
            Key::Undo => l10n_helper::os_or("keys", "Undo", l10n!("keys/Undo", "Undo")),
            Key::Redo => l10n_helper::os_or("keys", "Redo", l10n!("keys/Redo", "Redo")),
            Key::ContextMenu => l10n_helper::os_or("keys", "ContextMenu", l10n!("keys/ContextMenu", "≣Context Menu")),
            Key::Escape => l10n_helper::os_or("keys", "Escape", l10n!("keys/Escape", "Esc")),
            Key::Find => l10n_helper::os_or("keys", "Find", l10n!("keys/Find", "Find")),
            Key::Help => l10n_helper::os_or("keys", "Help", l10n!("keys/Help", "?Help")),
            Key::ZoomIn => l10n_helper::os_or("keys", "ZoomIn", l10n!("keys/ZoomIn", "+Zoom In")),
            Key::ZoomOut => l10n_helper::os_or("keys", "ZoomOut", l10n!("keys/ZoomOut", "-Zoom Out")),
            Key::Eject => l10n_helper::os_or("keys", "Eject", l10n!("keys/Eject", "⏏Eject")),
            Key::PrintScreen => l10n_helper::os_or("keys", "PrintScreen", l10n!("keys/PrintScreen", "PrtSc")),
            Key::Close => l10n_helper::os_or("keys", "Close", l10n!("keys/Close", "Close")),
            Key::New => l10n_helper::os_or("keys", "New", l10n!("keys/New", "New")),
            Key::Open => l10n_helper::os_or("keys", "Open", l10n!("keys/Open", "Open")),
            Key::Print => l10n_helper::os_or("keys", "Print", l10n!("keys/Open", "Print")),
            Key::Save => l10n_helper::os_or("keys", "Save", l10n!("keys/Save", "Save")),
            key => l10n_helper::l10n("keys", key.name()),
        },
        GestureKey::Code(key_code) => formatx!("{key_code:?}").into_var(),
    }
}
