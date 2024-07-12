//! Dialog widget.

use core::fmt;
use std::ops;

use zng_ext_input::focus::{DirectionalNav, TabNav};
use zng_ext_l10n::l10n;
use zng_wgt::{modal, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_filter::drop_shadow;
use zng_wgt_input::focus::{directional_nav, tab_nav, FocusableMix};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

/// A modal dialog overlay container.
#[widget($crate::dialog::Dialog)]
pub struct Dialog(FocusableMix<StyleMix<Container>>);
impl Dialog {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());

            directional_nav = DirectionalNav::Cycle;
            tab_nav = TabNav::Cycle;
            modal = true;
            focus_on_init = true;
            return_focus_on_deinit = true;
        }
    }
}
impl_style_fn!(Dialog);

/// Dialog default style.
#[widget($crate::dialog::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            replace = true;

            // same as window
            background_color = light_dark(rgb(0.9, 0.9, 0.9), rgb(0.1, 0.1, 0.1));
            drop_shadow = {
                offset: 2,
                blur_radius: 2,
                color: colors::BLACK.with_alpha(50.pct()),
            };
        }
    }
}

context_var! {
    /// Title widget, usually placed as `child_out_top`.
    pub static DIALOG_TITLE_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Icon widget, usually placed as `child_out_start`.
    pub static DIALOG_ICON_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Content widget, usually the dialog child.
    pub static DIALOG_CONTENT_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Dialog response button generator, usually placed as `child_out_bottom`.
    pub static DIALOG_BUTTON_FN_VAR: WidgetFn<DialogButtonArgs> = WidgetFn::nil();
    /// Dialog responses.
    pub static DIALOG_RESPONSES_VAR: Responses = Responses::ok();
}

/// Arguments for [`button_fn`].
///
/// [`button_fn`]: fn@button_fn
pub struct DialogButtonArgs {
    /// The response that must be represented by the button.
    pub response: Response,
    /// If the button is the last entry on the responses list.
    pub is_last: bool,
}

/// Dialog title widget.
///
/// Note that this takes in an widget, you can use `Text!("title")` to set to a text.
///
/// Sets the [`DIALOG_TITLE_VAR`].
#[property(CONTEXT, default(DIALOG_TITLE_VAR), widget_impl(Dialog))]
pub fn title(child: impl UiNode, title: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    with_context_var(child, DIALOG_TITLE_VAR, title)
}

/// Dialog icon widget.
///
/// Note that this takes in an widget, you can use the `ICONS` service to get an icon widget.
///
/// Sets the [`DIALOG_ICON_VAR`].
#[property(CONTEXT, default(DIALOG_ICON_VAR), widget_impl(Dialog))]
pub fn icon(child: impl UiNode, icon: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    with_context_var(child, DIALOG_ICON_VAR, icon)
}

/// Dialog content widget.
///
/// Note that this takes in an widget, you can use `SelectableText!("message")` for the message.
///
/// Sets the [`DIALOG_CONTENT_VAR`].
#[property(CONTEXT, default(DIALOG_CONTENT_VAR), widget_impl(Dialog))]
pub fn content(child: impl UiNode, content: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    with_context_var(child, DIALOG_CONTENT_VAR, content)
}

/// Dialog button generator.
///
/// Sets the [`DIALOG_BUTTON_FN_VAR`].
#[property(CONTEXT, default(DIALOG_BUTTON_FN_VAR), widget_impl(Dialog))]
pub fn button_fn(child: impl UiNode, button: impl IntoVar<WidgetFn<DialogButtonArgs>>) -> impl UiNode {
    with_context_var(child, DIALOG_BUTTON_FN_VAR, button)
}

/// Dialog responses.
///
/// Sets the [`DIALOG_RESPONSES_VAR`].
#[property(CONTEXT, default(DIALOG_RESPONSES_VAR), widget_impl(Dialog))]
pub fn responses(child: impl UiNode, responses: impl IntoVar<Responses>) -> impl UiNode {
    with_context_var(child, DIALOG_RESPONSES_VAR, responses)
}

/// Dialog info style.
///
/// Sets the info icon and a single "Ok" response.
#[widget($crate::dialog::InfoStyle)]
pub struct InfoStyle(DefaultStyle);

/// Dialog warn style.
///
/// Sets the warn icon and a single "Ok" response.
#[widget($crate::dialog::WarnStyle)]
pub struct WarnStyle(DefaultStyle);

/// Dialog error style.
///
/// Sets the error icon and a single "Ok" response.
#[widget($crate::dialog::ErrorStyle)]
pub struct ErrorStyle(DefaultStyle);

/// Question style.
///
/// Sets the question icon and two "No" and "Yes" responses.
#[widget($crate::dialog::QuestionStyle)]
pub struct QuestionStyle(DefaultStyle);

/// Confirmation style.
///
/// Sets the question icon and two "Cancel" and "Ok" responses.
#[widget($crate::dialog::ConfirmStyle)]
pub struct ConfirmStyle(DefaultStyle);

/// Dialog response.
#[derive(Clone)]
pub struct Response {
    /// Response identifying name.
    pub name: Txt,
    /// Response button label.
    pub label: BoxedVar<Txt>,
}
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.name)
    }
}
impl PartialEq for Response {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Response {
    /// New from name and label.
    pub fn new(name: impl Into<Txt>, label: impl IntoVar<Txt>) -> Self {
        Self {
            name: name.into(),
            label: label.into_var().boxed(),
        }
    }

    /// "Ok"
    pub fn ok() -> Self {
        Self::new("Ok", l10n!("dialog/response-ok", "Ok"))
    }

    /// "Cancel"
    pub fn cancel() -> Self {
        Self::new("Cancel", l10n!("dialog/response-cancel", "Cancel"))
    }

    /// "Yes"
    pub fn yes() -> Self {
        Self::new("Yes", l10n!("dialog/response-yes", "Yes"))
    }
    /// "No"
    pub fn no() -> Self {
        Self::new("No", l10n!("dialog/response-no", "No"))
    }

    /// "Close"
    pub fn close() -> Self {
        Self::new("Close", l10n!("dialog/response-close", "Close"))
    }
}

/// Response labels.
#[derive(Clone, PartialEq, Debug)]
pub struct Responses(pub Vec<Response>);
impl Responses {
    /// new with first response.
    pub fn new(r: impl Into<Response>) -> Self {
        Self(vec![r.into()])
    }

    /// With response.
    pub fn with(mut self, response: impl Into<Response>) -> Self {
        self.push(response.into());
        self
    }

    /// "Ok"
    pub fn ok() -> Self {
        Response::ok().into()
    }

    /// "Close"
    pub fn close() -> Self {
        Response::close().into()
    }

    /// "No", "Yes"
    pub fn no_yes() -> Self {
        vec![Response::no(), Response::yes()].into()
    }

    /// "Cancel", "Ok"
    pub fn cancel_ok() -> Self {
        vec![Response::cancel(), Response::ok()].into()
    }
}
impl ops::Deref for Responses {
    type Target = Vec<Response>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for Responses {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl_from_and_into_var! {
    fn from(response: Response) -> Responses {
        Responses::new(response)
    }
    fn from(responses: Vec<Response>) -> Responses {
        Responses(responses)
    }
}

// !!: TODO
// * Outer background?
//   - No, it should be only one if there is any dialog open?
//   - If a second dialog opens it moves over the previous dialog, it does not blurs twice
//   - This should be optional, the layer example does not
// * WidgetListFn?

/// Dialog overlay service.
pub struct DIALOG;
impl DIALOG {
    /// Open the `dialog`.
    ///
    /// Returns the selected response or [`close`] if the dialog is closed without response.
    ///
    /// [`close`]: Response::close
    pub fn open(&self, _dialog: impl UiNode) -> ResponseVar<Response> {
        todo!()
    }

    /// Close the contextual dialog with the response.
    pub fn respond(&self, _response: Response) {
        todo!()
    }
}
