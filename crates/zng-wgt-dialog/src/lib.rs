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

use parking_lot::Mutex;
use zng_ext_l10n::l10n;
use zng_var::ContextInitHandle;
use zng_wgt::{prelude::*, *};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_filter::drop_shadow;
use zng_wgt_input::focus::alt_focus_scope;
use zng_wgt_layer::{
    popup::{ContextCapture, Popup, POPUP},
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
            clip_to_bounds = true;

            margin = 10;
            zng_wgt_container::padding = 15;

            align = Align::CENTER;

            zng_wgt_container::child_out_top = Container! {
                corner_radius = 0;
                background_color = light_dark(rgb(0.85, 0.85, 0.85), rgb(0.15, 0.15, 0.15));
                child = presenter((), DIALOG_TITLE_VAR);
                child_align = Align::START;
                padding = (4, 8);
                zng_wgt_text::font_weight = zng_ext_font::FontWeight::BOLD;
            }, 0;

            zng_wgt_container::child_out_bottom = presenter(DIALOG_RESPONSES_VAR, wgt_fn!(|responses: Responses| {
                Wrap! {
                    corner_radius = 0;
                    background_color = light_dark(rgb(0.85, 0.85, 0.85), rgb(0.15, 0.15, 0.15));
                    children_align = Align::END;
                    zng_wgt_container::padding = 3;
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

            zng_wgt_container::child_out_left = Container! {
                child = presenter((), DIALOG_ICON_VAR);
                child_align = Align::TOP;
            }, 0;

            zng_wgt_container::child = presenter((), DIALOG_CONTENT_VAR);
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
    pub static DIALOG_BUTTON_FN_VAR: WidgetFn<DialogButtonArgs> = WidgetFn::new(default_dialog_button_fn);
    /// Dialog responses.
    pub static DIALOG_RESPONSES_VAR: Responses = Responses::ok();
    /// Dialog outer container.
    pub static DIALOG_BACKDROP_FN_VAR: WidgetFn<DialogBackdropArgs> = WidgetFn::new(default_dialog_backdrop_fn);
}

/// Default value of [`dialog_button_fn`](fn@dialog_button_fn)
pub fn default_dialog_button_fn(args: DialogButtonArgs) -> impl UiNode {
    zng_wgt_button::Button! {
        child = Text!(args.response.name.clone());
        on_click = hn_once!(|a: &zng_wgt_input::gesture::ClickArgs| {
            a.propagation().stop();
            DIALOG.respond(args.response);
        });
        focus_on_init = args.is_last;
        when args.is_last {
            style_fn = zng_wgt_button::PrimaryStyle!();
        }
    }
}

/// Default value of [`dialog_backdrop_fn`](fn@dialog_backdrop_fn)
pub fn default_dialog_backdrop_fn(args: DialogBackdropArgs) -> impl UiNode {
    Container! {
        child = args.dialog;
        background_color = colors::RED;
    }
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

/// Arguments for [`dialog_backdrop_fn`].
pub struct DialogBackdropArgs {
    /// The dialog widget.
    pub dialog: BoxedUiNode,
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

/// Widget function called when a dialog is shown in the context to create the backdrop.
///
/// Note that this property must be set on a parent widget or the window, not on the dialog widget.
#[property(CONTEXT, default(DIALOG_BACKDROP_FN_VAR))]
pub fn dialog_backdrop_fn(child: impl UiNode, backdrop: impl IntoVar<WidgetFn<DialogBackdropArgs>>) -> impl UiNode {
    with_context_var(child, DIALOG_BACKDROP_FN_VAR, backdrop)
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
            icon = Container! {
                child = ICONS.req(["dialog-info", "info"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = colors::AZURE;
                padding = 5;
            };
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
    /// Open the custom `dialog`.
    ///
    /// Returns the selected response or [`close`] if the dialog is closed without response.
    ///
    /// [`close`]: Response::close
    pub fn show(&self, dialog: impl UiNode) -> ResponseVar<Response> {
        self.show_impl(dialog.boxed())
    }

    fn show_dlg(&self, msg: BoxedVar<Txt>, title: BoxedVar<Txt>, style: zng_wgt_style::StyleBuilder) -> ResponseVar<Response> {
        self.show(Dialog! {
            style_fn = style;
            title = Text! {
                visibility = title.map(|t| Visibility::from(!t.is_empty()));
                txt = title;
            };
            content = SelectableText!(msg);
        })
    }

    /// Show an info dialog with "Ok" button.
    pub fn info(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show_dlg(msg.into_var().boxed(), title.into_var().boxed(), InfoStyle!())
            .map_response(|_| ())
    }

    /// Show a warning dialog with "Ok" button.
    pub fn warn(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show_dlg(msg.into_var().boxed(), title.into_var().boxed(), WarnStyle!())
            .map_response(|_| ())
    }

    /// Show an error dialog with "Ok" button.
    pub fn error(&self, msg: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.show_dlg(msg.into_var().boxed(), title.into_var().boxed(), ErrorStyle!())
            .map_response(|_| ())
    }

    /// Shows a question dialog with "No" and "Yes" buttons. Returns `true` for "Yes".
    pub fn question(&self, question: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.show_dlg(question.into_var().boxed(), title.into_var().boxed(), QuestionStyle!())
            .map_response(|r| r.name == "Yes")
    }

    /// Shows a question dialog with "Cancel" and "Ok" buttons. Returns `true` for "Ok".
    pub fn confirm(&self, question: impl IntoVar<Txt>, title: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.show_dlg(question.into_var().boxed(), title.into_var().boxed(), ConfirmStyle!())
            .map_response(|r| r.name == "Ok")
    }

    /// Close the contextual dialog with the response.
    pub fn respond(&self, response: Response) {
        let ctx = DIALOG_CTX.get();
        let id = *ctx.dialog_id.lock();
        if let Some(id) = id {
            ctx.responder.respond(response);
            POPUP.close_id(id);
        } else {
            tracing::error!("DIALOG.respond called outside of a dialog");
        }
    }

    fn show_impl(&self, dialog: BoxedUiNode) -> ResponseVar<Response> {
        let (responder, response) = response_var();

        let mut ctx = Some(Arc::new(DialogCtx {
            dialog_id: Mutex::new(None),
            responder,
        }));

        let dialog = DIALOG_BACKDROP_FN_VAR.get()(DialogBackdropArgs { dialog });

        let dialog = match_widget(
            dialog,
            clmv!(|c, op| {
                match &op {
                    UiNodeOp::Init => {
                        *ctx.as_ref().unwrap().dialog_id.lock() = c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id());
                        DIALOG_CTX.with_context(&mut ctx, || c.op(op));
                        // in case a non-standard dialog widget is used
                        *ctx.as_ref().unwrap().dialog_id.lock() = c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id());
                    }
                    UiNodeOp::Deinit => {}
                    _ => {
                        DIALOG_CTX.with_context(&mut ctx, || c.op(op));
                    }
                }
            }),
        );

        zng_wgt_layer::popup::CLOSE_ON_FOCUS_LEAVE_VAR.with_context_var(ContextInitHandle::new(), false, || {
            POPUP.open_config(dialog, AnchorMode::window(), ContextCapture::NoCapture)
        });

        response
    }
}

struct DialogCtx {
    dialog_id: Mutex<Option<WidgetId>>,
    responder: ResponderVar<Response>,
}
context_local! {
    static DIALOG_CTX: DialogCtx = DialogCtx {
        dialog_id: Mutex::new(None),
        responder: response_var().0,
    };
}

// !!: TODO
// * Backdrop widget
//   - No, it should be only one if there is any dialog open?
//   - If a second dialog opens it moves over the previous dialog, it does not blurs twice
//   - This should be optional, the layer example does not
// * Animate
