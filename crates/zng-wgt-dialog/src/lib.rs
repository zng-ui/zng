#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Dialog widget and service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use std::{fmt, ops, sync::Arc};

use zng_ext_l10n::l10n;
use zng_wgt::{align, corner_radius, margin, modal, modal_included, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_filter::drop_shadow;
use zng_wgt_input::focus::alt_focus_scope;
use zng_wgt_layer::{
    popup::{ContextCapture, Popup, PopupState, POPUP},
    AnchorMode,
};
use zng_wgt_style::{impl_style_fn, style_fn, Style};
use zng_wgt_text::Text;
use zng_wgt_text_input::selectable::SelectableText;
use zng_wgt_wrap::Wrap;

/// A modal dialog overlay container.
#[widget($crate::Dialog)]
pub struct Dialog(Popup);
impl Dialog {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());

            modal = true;
            return_focus_on_deinit = true;

            alt_focus_scope = unset!;
            focus_click_behavior = unset!;
            modal_included = unset!;
        }
    }
}
impl_style_fn!(Dialog);

/// Dialog default style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            replace = true;

            background_color = light_dark(rgb(0.7, 0.7, 0.7), rgb(0.3, 0.3, 0.3));
            drop_shadow = {
                offset: 4,
                blur_radius: 6,
                color: colors::BLACK.with_alpha(50.pct()),
            };

            corner_radius = 8;
            margin = 10;
            zng_wgt_container::padding = 10;

            align = Align::CENTER;

            zng_wgt_container::child = presenter((), DIALOG_CONTENT_VAR);

            zng_wgt_container::child_out_top = Container! {
                child = presenter((), DIALOG_TITLE_VAR);
                child_align = Align::START;
            }, 0;
            zng_wgt_container::child_out_left = Container! {
                child = presenter((), DIALOG_ICON_VAR);
                child_align = Align::TOP;
            }, 0;
            zng_wgt_container::child_out_bottom = presenter(DIALOG_RESPONSES_VAR, wgt_fn!(|responses: Responses| {
                Wrap! {
                    children_align = Align::END;
                    children = {
                        let last = responses.len().saturating_sub(1);
                        responses.0
                            .into_iter()
                            .enumerate()
                            .map(|(i, r)| presenter(
                                DialogButtonArgs { response: r, is_last: i == last },
                                DIALOG_BUTTON_FN_VAR
                            ).boxed())
                            .collect::<UiNodeVec>()
                    };
                }
            })), 0;
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
#[derive(Debug, Clone, PartialEq)]
pub struct DialogButtonArgs {
    /// The response that must be represented by the button.
    pub response: Response,
    /// If the button is the last entry on the responses list.
    pub is_last: bool,
}

/// Dialog title widget.
///
/// Note that this takes in an widget, you can use `Text!("title")` to set to a text.
#[property(CONTEXT, default(NilUiNode), widget_impl(Dialog))]
pub fn title(child: impl UiNode, title: impl UiNode) -> impl UiNode {
    with_context_var(child, DIALOG_TITLE_VAR, WidgetFn::singleton(title))
}

/// Dialog icon widget.
///
/// Note that this takes in an widget, you can use the `ICONS` service to get an icon widget.
#[property(CONTEXT, default(NilUiNode), widget_impl(Dialog))]
pub fn icon(child: impl UiNode, icon: impl UiNode) -> impl UiNode {
    with_context_var(child, DIALOG_ICON_VAR, WidgetFn::singleton(icon))
}

/// Dialog content widget.
///
/// Note that this takes in an widget, you can use `SelectableText!("message")` for the message.
#[property(CONTEXT, default(FillUiNode), widget_impl(Dialog))]
pub fn content(child: impl UiNode, content: impl UiNode) -> impl UiNode {
    with_context_var(child, DIALOG_CONTENT_VAR, WidgetFn::singleton(content))
}

/// Dialog button generator.
#[property(CONTEXT, default(DIALOG_BUTTON_FN_VAR), widget_impl(Dialog))]
pub fn button_fn(child: impl UiNode, button: impl IntoVar<WidgetFn<DialogButtonArgs>>) -> impl UiNode {
    with_context_var(child, DIALOG_BUTTON_FN_VAR, button)
}

/// Dialog responses.
#[property(CONTEXT, default(DIALOG_RESPONSES_VAR), widget_impl(Dialog))]
pub fn responses(child: impl UiNode, responses: impl IntoVar<Responses>) -> impl UiNode {
    with_context_var(child, DIALOG_RESPONSES_VAR, responses)
}

/// Dialog info style.
///
/// Sets the info icon and a single "Ok" response.
#[widget($crate::InfoStyle)]
pub struct InfoStyle(DefaultStyle);
impl InfoStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            responses = Responses::ok();

        }
    }
}

/// Dialog warn style.
///
/// Sets the warn icon and a single "Ok" response.
#[widget($crate::WarnStyle)]
pub struct WarnStyle(DefaultStyle);

/// Dialog error style.
///
/// Sets the error icon and a single "Ok" response.
#[widget($crate::ErrorStyle)]
pub struct ErrorStyle(DefaultStyle);

/// Question style.
///
/// Sets the question icon and two "No" and "Yes" responses.
#[widget($crate::QuestionStyle)]
pub struct QuestionStyle(DefaultStyle);

/// Confirmation style.
///
/// Sets the question icon and two "Cancel" and "Ok" responses.
#[widget($crate::ConfirmStyle)]
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
        Self::new("Ok", l10n!("response-ok", "Ok"))
    }

    /// "Cancel"
    pub fn cancel() -> Self {
        Self::new("Cancel", l10n!("response-cancel", "Cancel"))
    }

    /// "Yes"
    pub fn yes() -> Self {
        Self::new("Yes", l10n!("response-yes", "Yes"))
    }
    /// "No"
    pub fn no() -> Self {
        Self::new("No", l10n!("response-no", "No"))
    }

    /// "Close"
    pub fn close() -> Self {
        Self::new("Close", l10n!("response-close", "Close"))
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

/// Dialog overlay service.
pub struct DIALOG;
impl DIALOG {
    /// Open the `dialog`.
    ///
    /// Returns the selected response or [`close`] if the dialog is closed without response.
    ///
    /// [`close`]: Response::close
    pub fn show(&self, dialog: impl UiNode) -> ResponseVar<Response> {
        self.show_impl(dialog.boxed())
    }

    /// Show an info dialog with "Ok" button.
    pub fn inform(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show(Dialog! {
            style_fn = InfoStyle!();
            title = Text!(title);
            content = SelectableText!(msg);
        })
        .map_response(|_| ())
    }

    /// Show a warning dialog with "Ok" button.
    pub fn warn(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show(Dialog! {
            style_fn = WarnStyle!();
            title = Text!(title);
            content = SelectableText!(msg);
        })
        .map_response(|_| ())
    }

    /// Show an error dialog with "Ok" button.
    pub fn error(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show(Dialog! {
            style_fn = ErrorStyle!();
            title = Text!(title);
            content = SelectableText!(msg);
        })
        .map_response(|_| ())
    }

    /// Shows a question dialog with "No" and "Yes" buttons. Returns `true` for "Yes".
    pub fn ask(&self, question: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.show(Dialog! {
            style_fn = QuestionStyle!();
            title = Text!(title);
            content = SelectableText!(question);
        })
        .map_response(|r| r.name == "Yes")
    }

    /// Shows a question dialog with "Cancel" and "Ok" buttons. Returns `true` for "Ok".
    pub fn confirm(&self, question: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.show(Dialog! {
            style_fn = InfoStyle!();
            title = Text!(title);
            content = SelectableText!(question);
        })
        .map_response(|r| r.name == "Ok")
    }

    /// Close the contextual dialog with the response.
    pub fn respond(&self, response: Response) {
        if DIALOG_RESPONDER_VAR.set(zng_var::types::Response::Done(response)).is_ok() {
            POPUP.close_id(WIDGET.id());
        } else {
            tracing::error!("DIALOG.respond called outside of a dialog");
        }
    }

    fn show_impl(&self, dialog: BoxedUiNode) -> ResponseVar<Response> {
        let (responder, response) = response_var();

        let mut ctx = Some(Arc::new(responder.clone().boxed()));
        let id = zng_var::ContextInitHandle::new();
        let dialog = match_widget(
            dialog,
            clmv!(id, |c, op| {
                DIALOG_RESPONDER_VAR.with_context(id.clone(), &mut ctx, || c.op(op));
            }),
        );

        let state = zng_wgt_layer::popup::CLOSE_ON_FOCUS_LEAVE_VAR.with_context_var(id, false, || {
            POPUP.open_config(
                dialog,
                AnchorMode::window(),
                ContextCapture::CaptureBlend {
                    filter: CaptureFilter::None,
                    over: false,
                },
            )
        });

        // if popup closes without responding set response to `Response::close()`.
        let responder_wk = responder.downgrade();
        state
            .hook(move |v| {
                let mut retain = false;
                if let Some(r) = responder_wk.upgrade() {
                    retain = true;
                    if matches!(v.value(), PopupState::Closed) {
                        retain = false;
                        r.modify(|v| {
                            if v.is_waiting() {
                                v.set(zng_var::types::Response::Done(Response::close()));
                            }
                        });
                    }
                }
                retain
            })
            .perm();
        responder.hold(state).perm();

        response
    }
}

context_var! {
    static DIALOG_RESPONDER_VAR: zng_var::types::Response<Response> = zng_var::types::Response::Waiting;
}

// !!: TODO
// * Backdrop widget
//   - No, it should be only one if there is any dialog open?
//   - If a second dialog opens it moves over the previous dialog, it does not blurs twice
//   - This should be optional, the layer example does not
// * Animate
