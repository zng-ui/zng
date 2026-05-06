//! Label text.

use zng_app::property_args;
use zng_ext_input::{focus::FOCUS, gesture::CLICK_EVENT};
use zng_wgt::prelude::*;
use zng_wgt_input::{
    focus::FocusableMix,
    gesture::{Mnemonic, get_mnemonic, get_mnemonic_char, mnemonic_txt},
};
use zng_wgt_style::{Style, StyleMix, impl_style_fn};

#[doc(hidden)]
pub use zng_wgt::prelude::formatx as __formatx;
use zng_wgt_text::node::TEXT;

/// Styleable and focusable read-only text widget.
///
/// Optionally can be the label of a [`target`](#method.target) widget, if set the target widget is focused when the label is focused.
///
/// # Shorthand
///
/// The widget macro supports the shorthand that sets the `txt` and `target` properties.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::prelude::*;
/// # use zng_wgt_text_input::label::*;
/// #
/// # fn main() {
/// # let _scope = zng_app::APP.minimal();
/// let label = Label!("txt", "target");
/// # }
/// ```
#[widget($crate::label::Label {
    ($txt:expr, $target:expr $(,)?) => {
        txt = $txt;
        target = $target;
    };
    ($txt:literal) => {
        txt = $crate::label::__formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
})]
pub struct Label(FocusableMix<StyleMix<zng_wgt_text::Text>>); // TODO(breaking) remove FocusableMix
impl Label {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        // this used to be true when Label! was only really useful with a `target`,
        // so as a fallback when it had no target it was at least focusable
        //
        // now Label! primary use case is mnemonic shortcuts so this changes
        widget_set! {
            self;
            focusable = false;
        }

        // replace the txt with one that removes the mnemonic marker
        self.widget_builder().push_pre_build_action(|wgt| {
            let mut mnemonic_data = None;
            if let Some(txt) = wgt.property_mut(property_id!(zng_wgt_text::txt))
                && !*txt.captured
            {
                let t = txt.args.downcast_var::<Txt>(0);

                let mnemonic = var(Mnemonic::None);
                mnemonic_data = Some((t.clone(), mnemonic.clone()));
                let t = expr_var! {
                    let t = #{t};
                    if let Mnemonic::FromTxt { marker } = #{mnemonic} {
                        let mut prev_is_marker = false;
                        for (i, c) in t.char_indices() {
                            if c == *marker {
                                prev_is_marker = true;
                            } else if prev_is_marker && c.is_alphanumeric() {
                                return formatx!("{}{}", &t[..i - 1], &t[i..]);
                            }
                        }
                    }
                    t.clone()
                };
                *txt.args = property_args!(zng_wgt_text::txt = t);
            }
            if let Some((raw_txt, mnemonic)) = mnemonic_data {
                wgt.push_intrinsic(NestGroup::WIDGET_INNER, "get_mnemonic", move |c| get_mnemonic(c, mnemonic.clone()));
                wgt.push_intrinsic(NestGroup::CHILD, "mnemonic_txt", move |c| mnemonic_txt(c, raw_txt.clone()));
            }
        });
    }
}
impl_style_fn!(Label, DefaultStyle);

/// Default label style.
#[widget($crate::label::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;
        }
    }
}

/// Defines the widget the label is for.
///
/// When the label is pressed the widget or the first focusable child of the widget is focused.
/// Accessibility metadata is also set so the target widget is marked as *labelled-by* this widget in the view-process.
#[property(CONTEXT, widget_impl(Label))]
pub fn target(child: impl IntoUiNode, target: impl IntoVar<WidgetId>) -> UiNode {
    let target = target.into_var();
    let mut prev_target = None::<WidgetId>;

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&target).sub_event_when(&CLICK_EVENT, |a| a.is_primary());
        }
        UiNodeOp::Info { info } => {
            c.info(info);
            if let Some(mut a) = info.access() {
                let target = target.get();
                prev_target = Some(target);
                a.set_labels(target);
            }
        }
        UiNodeOp::Update { updates } => {
            if let Some(id) = target.get_new() {
                if let Some(id) = prev_target.take() {
                    UPDATES.update_info(id);
                }
                UPDATES.update_info(id);
                WIDGET.update_info();
            }

            c.update(updates);

            if CLICK_EVENT.any_update(true, |a| a.is_primary()) {
                FOCUS.focus_widget_or_enter(target.get(), true, false);
            }
        }
        _ => {}
    })
}

/// Draw underline for the first occurrence of the mnemonic char in text.
///
/// When enabled this overrides [`underline_skip`], only the first char defined by [`get_mnemonic_char`] is underlined.
///
/// Note that the [`underline`] must still be set otherwise no underline is rendered.
///
/// [`underline_skip`]: fn@zng_wgt_text::underline_skip
/// [`underline`]: fn@zng_wgt_text::underline
/// [`get_mnemonic_char`]: fn@get_mnemonic_char
#[property(FILL, widget_impl(Label))]
pub fn mnemonic_underline(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let m_char = var(None);
    let child = get_mnemonic_char(child, m_char.clone());

    let enabled = enabled.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&enabled).sub_var_layout(&m_char);
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);

            if enabled.get()
                && let Some(c) = m_char.get()
            {
                let r = TEXT.resolved();
                let mut ci = None;
                for (i, tc) in r.segmented_text.text().char_indices() {
                    if c.to_lowercase().eq(tc.to_lowercase()) {
                        ci = Some(i);
                        break;
                    }
                }
                if let Some(i) = ci {
                    let l = TEXT.laidout();
                    let start = l.shaped_text.snap_caret_line(i.into());
                    let mut end = start;
                    end.index += c.len_utf8();
                    let u = l.shaped_text.highlight_underlines(start..end, r.segmented_text.text()).collect();
                    TEXT.set_underlines(u);
                }
            }
        }
        _ => {}
    })
}
