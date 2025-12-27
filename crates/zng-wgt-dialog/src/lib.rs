#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Dialog widget and service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use std::{fmt, ops, path::PathBuf, sync::Arc};

use bitflags::bitflags;
use parking_lot::Mutex;
use zng_app::view_process::VIEW_PROCESS;
use zng_ext_l10n::l10n;
use zng_ext_window::{WINDOW_CLOSE_REQUESTED_EVENT, WINDOWS};
use zng_var::{ContextInitHandle, animation::easing};
use zng_view_api::dialog as native_api;
use zng_wgt::{node::VarPresent as _, prelude::*, *};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_filter::drop_shadow;
use zng_wgt_input::focus::FocusableMix;
use zng_wgt_layer::{
    AnchorMode,
    popup::{ContextCapture, POPUP, POPUP_CLOSE_REQUESTED_EVENT},
};
use zng_wgt_style::{Style, StyleMix, impl_named_style_fn, impl_style_fn};
use zng_wgt_text::Text;
use zng_wgt_text_input::selectable::SelectableText;
use zng_wgt_wrap::Wrap;

pub mod backdrop;

pub use zng_view_api::dialog::{DialogCapability as NativeDialogCapacity, FileDialogFilters, FileDialogResponse};

/// A modal dialog overlay container.
#[widget($crate::Dialog)]
pub struct Dialog(FocusableMix<StyleMix<Container>>);
impl Dialog {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        self.widget_builder()
            .push_build_action(|b| b.push_intrinsic(NestGroup::EVENT, "dialog-closing", dialog_closing_node));

        widget_set! {
            self;

            focus_on_init = true;
            return_focus_on_deinit = true;

            when *#is_close_delaying {
                interactive = false;
            }
        }
    }

    widget_impl! {
        /// If a respond close was requested for this dialog and it is just awaiting for the [`popup::close_delay`].
        ///
        /// The close delay is usually set on the backdrop widget style.
        ///
        /// [`popup::close_delay`]: fn@zng_wgt_layer::popup::close_delay
        pub zng_wgt_layer::popup::is_close_delaying(state: impl IntoVar<bool>);

        /// An attempt to close the dialog was made without setting the response.
        ///
        /// Dialogs must only close using [`DIALOG.respond`](DIALOG::respond).
        pub on_dialog_close_canceled(args: Handler<DialogCloseCanceledArgs>);
    }
}
impl_style_fn!(Dialog, DefaultStyle);

fn dialog_closing_node(child: impl IntoUiNode) -> UiNode {
    match_node(child, move |_, op| {
        match op {
            UiNodeOp::Init => {
                // layers receive events after window content, so we subscribe directly
                let id = WIDGET.id();
                let ctx = DIALOG_CTX.get();
                let default_response = DEFAULT_RESPONSE_VAR.current_context();
                let responder = ctx.responder.clone();
                let handle = WINDOW_CLOSE_REQUESTED_EVENT.on_pre_event(hn!(|args| {
                    // a window is closing
                    if responder.get().is_waiting() {
                        // dialog has no response

                        let path = WINDOWS.widget_info(id).unwrap().path();
                        if args.windows.contains(&path.window_id()) {
                            // is closing dialog parent window

                            if let Some(default) = default_response.get() {
                                // has default response
                                responder.respond(default);
                                // in case the window close is canceled by other component
                                zng_wgt_layer::popup::POPUP_CLOSE_CMD
                                    .scoped(path.window_id())
                                    .notify_param(path.widget_id());
                            } else {
                                // no default response, cancel close
                                args.propagation().stop();
                                DIALOG_CLOSE_CANCELED_EVENT.notify(DialogCloseCanceledArgs::now(path));
                            }
                        }
                    }
                }));
                WIDGET.push_event_handle(handle);
                WIDGET.sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on(update) {
                    // dialog is closing
                    let ctx = DIALOG_CTX.get();
                    if ctx.responder.get().is_waiting() {
                        // dialog has no response
                        if let Some(r) = DEFAULT_RESPONSE_VAR.get() {
                            ctx.responder.respond(r);
                        } else {
                            args.propagation().stop();
                            DIALOG_CLOSE_CANCELED_EVENT.notify(DialogCloseCanceledArgs::now(WIDGET.info().path()));
                        }
                    }
                }
            }
            _ => (),
        }
    })
}

event_args! {
    /// Arguments for [`DIALOG_CLOSE_CANCELED_EVENT`].
    pub struct DialogCloseCanceledArgs {
        /// Dialog widget.
        pub target: WidgetPath,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }
}
event! {
    /// An attempt to close the dialog was made without setting the response.
    ///
    /// Dialogs must only close using [`DIALOG.respond`](DIALOG::respond).
    pub static DIALOG_CLOSE_CANCELED_EVENT: DialogCloseCanceledArgs;
}
event_property! {
    // An attempt to close the dialog was made without setting the response.
    ///
    /// Dialogs must only close using [`DIALOG.respond`](DIALOG::respond).
    pub fn dialog_close_canceled {
        event: DIALOG_CLOSE_CANCELED_EVENT,
        args: DialogCloseCanceledArgs,
    }
}

/// Dialog default style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        let highlight_color = var(colors::BLACK.transparent());
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
                child = TITLE_VAR.present_data(());
                child_align = Align::START;
                padding = (4, 8);
                zng_wgt_text::font_weight = zng_ext_font::FontWeight::BOLD;
            };

            zng_wgt_container::child_out_bottom = RESPONSES_VAR.present(wgt_fn!(|responses: Responses| {
                Wrap! {
                    corner_radius = 0;
                    background_color = light_dark(rgb(0.85, 0.85, 0.85), rgb(0.15, 0.15, 0.15));
                    children_align = Align::END;
                    zng_wgt_container::padding = 3;
                    spacing = 3;
                    children = {
                        let last = responses.len().saturating_sub(1);
                        responses.0.into_iter().enumerate().map(move |(i, r)| {
                            presenter(
                                DialogButtonArgs {
                                    response: r,
                                    is_last: i == last,
                                },
                                BUTTON_FN_VAR,
                            )
                        })
                    };
                }
            }));

            zng_wgt_container::child_out_left = Container! {
                child = ICON_VAR.present_data(());
                child_align = Align::TOP;
            };

            zng_wgt_container::child = CONTENT_VAR.present_data(());

            #[easing(250.ms())]
            zng_wgt_filter::opacity = 30.pct();
            #[easing(250.ms())]
            zng_wgt_transform::transform = Transform::new_translate_y(-10).scale(98.pct());
            when *#is_inited && !*#zng_wgt_layer::popup::is_close_delaying {
                zng_wgt_filter::opacity = 100.pct();
                zng_wgt_transform::transform = Transform::identity();
            }

            zng_wgt_fill::foreground_highlight = {
                offsets: 0,
                widths: 2,
                sides: highlight_color.map_into(),
            };
            on_dialog_close_canceled = hn!(highlight_color, |_| {
                let c = colors::ACCENT_COLOR_VAR.rgba().get();
                let mut repeats = 0;
                highlight_color
                    .sequence(move |cv| {
                        repeats += 1;
                        if repeats <= 2 {
                            cv.set_ease(c, c.with_alpha(0.pct()), 120.ms(), easing::linear)
                        } else {
                            zng_var::animation::AnimationHandle::dummy()
                        }
                    })
                    .perm();
            });
        }
    }
}

context_var! {
    /// Title widget, usually placed as `child_out_top`.
    pub static TITLE_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Icon widget, usually placed as `child_out_start`.
    pub static ICON_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Content widget, usually the dialog child.
    pub static CONTENT_VAR: WidgetFn<()> = WidgetFn::nil();
    /// Dialog response button generator, usually placed as `child_out_bottom`.
    pub static BUTTON_FN_VAR: WidgetFn<DialogButtonArgs> = WidgetFn::new(default_button_fn);
    /// Dialog responses.
    pub static RESPONSES_VAR: Responses = Responses::ok();
    /// Dialog response when closed without setting a response.
    pub static DEFAULT_RESPONSE_VAR: Option<Response> = None;
    /// Defines what native dialogs are used on a context.
    pub static NATIVE_DIALOGS_VAR: DialogKind = DIALOG.native_dialogs();
}

/// Default value of [`button_fn`](fn@button_fn)
pub fn default_button_fn(args: DialogButtonArgs) -> UiNode {
    zng_wgt_button::Button! {
        child = Text!(args.response.label.clone());
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

/// Arguments for [`button_fn`].
///
/// [`button_fn`]: fn@button_fn
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DialogButtonArgs {
    /// The response that must be represented by the button.
    pub response: Response,
    /// If the button is the last entry on the responses list.
    pub is_last: bool,
}
impl DialogButtonArgs {
    /// New args.
    pub fn new(response: Response, is_last: bool) -> Self {
        Self { response, is_last }
    }
}

/// Dialog title widget.
///
/// Note that this takes in an widget, you can use `Text!("title")` to set to a text.
#[property(CONTEXT, default(UiNode::nil()), widget_impl(Dialog))]
pub fn title(child: impl IntoUiNode, title: impl IntoUiNode) -> UiNode {
    with_context_var(child, TITLE_VAR, WidgetFn::singleton(title))
}

/// Dialog icon widget.
///
/// Note that this takes in an widget, you can use the `ICONS` service to get an icon widget.
#[property(CONTEXT, default(UiNode::nil()), widget_impl(Dialog))]
pub fn icon(child: impl IntoUiNode, icon: impl IntoUiNode) -> UiNode {
    with_context_var(child, ICON_VAR, WidgetFn::singleton(icon))
}

/// Dialog content widget.
///
/// Note that this takes in an widget, you can use `SelectableText!("message")` for the message.
#[property(CONTEXT, default(FillUiNode), widget_impl(Dialog))]
pub fn content(child: impl IntoUiNode, content: impl IntoUiNode) -> UiNode {
    with_context_var(child, CONTENT_VAR, WidgetFn::singleton(content))
}

/// Dialog button generator.
#[property(CONTEXT, default(BUTTON_FN_VAR), widget_impl(Dialog, DefaultStyle))]
pub fn button_fn(child: impl IntoUiNode, button: impl IntoVar<WidgetFn<DialogButtonArgs>>) -> UiNode {
    with_context_var(child, BUTTON_FN_VAR, button)
}

/// Dialog responses.
#[property(CONTEXT, default(RESPONSES_VAR), widget_impl(Dialog))]
pub fn responses(child: impl IntoUiNode, responses: impl IntoVar<Responses>) -> UiNode {
    with_context_var(child, RESPONSES_VAR, responses)
}

/// Dialog response when closed without setting a response.
#[property(CONTEXT, default(DEFAULT_RESPONSE_VAR), widget_impl(Dialog))]
pub fn default_response(child: impl IntoUiNode, response: impl IntoVar<Option<Response>>) -> UiNode {
    with_context_var(child, DEFAULT_RESPONSE_VAR, response)
}

/// Defines what native dialogs are used by dialogs opened on the context.
///
/// Sets [`NATIVE_DIALOGS_VAR`].
#[property(CONTEXT, default(NATIVE_DIALOGS_VAR))]
pub fn native_dialogs(child: impl IntoUiNode, dialogs: impl IntoVar<DialogKind>) -> UiNode {
    with_context_var(child, NATIVE_DIALOGS_VAR, dialogs)
}

/// Dialog info style.
///
/// Sets the info icon and a single "Ok" response.
#[widget($crate::InfoStyle)]
pub struct InfoStyle(DefaultStyle);
impl_named_style_fn!(info, InfoStyle);
impl InfoStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = INFO_STYLE_FN_VAR;
            icon = Container! {
                child = ICONS.req(["dialog-info", "info"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = colors::AZURE;
                padding = 5;
            };
            default_response = Response::ok();
        }
    }
}

/// Dialog warn style.
///
/// Sets the warn icon and a single "Ok" response.
#[widget($crate::WarnStyle)]
pub struct WarnStyle(DefaultStyle);
impl_named_style_fn!(warn, WarnStyle);
impl WarnStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = WARN_STYLE_FN_VAR;
            icon = Container! {
                child = ICONS.req(["dialog-warn", "warning"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = colors::ORANGE;
                padding = 5;
            };
        }
    }
}

/// Dialog error style.
///
/// Sets the error icon and a single "Ok" response.
#[widget($crate::ErrorStyle)]
pub struct ErrorStyle(DefaultStyle);
impl_named_style_fn!(error, ErrorStyle);
impl ErrorStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = ERROR_STYLE_FN_VAR;
            icon = Container! {
                child = ICONS.req(["dialog-error", "error"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = rgb(209, 29, 29);
                padding = 5;
            };
        }
    }
}

/// Question style.
///
/// Sets the question icon and two "No" and "Yes" responses.
#[widget($crate::AskStyle)]
pub struct AskStyle(DefaultStyle);
impl_named_style_fn!(ask, AskStyle);
impl AskStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = ASK_STYLE_FN_VAR;
            icon = Container! {
                child = ICONS.req(["dialog-question", "question-mark"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = colors::AZURE;
                padding = 5;
            };
            responses = Responses::no_yes();
        }
    }
}

/// Confirmation style.
///
/// Sets the question icon and two "Cancel" and "Ok" responses.
#[widget($crate::ConfirmStyle)]
pub struct ConfirmStyle(DefaultStyle);
impl_named_style_fn!(confirm, ConfirmStyle);
impl ConfirmStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = CONFIRM_STYLE_FN_VAR;
            icon = Container! {
                child = ICONS.req(["dialog-confirm", "question-mark"]);
                zng_wgt_size_offset::size = 48;
                zng_wgt_text::font_color = colors::ORANGE;
                padding = 5;
            };
            responses = Responses::cancel_ok();
        }
    }
}

/// Dialog response.
#[derive(Clone)]
#[non_exhaustive]
pub struct Response {
    /// Response identifying name.
    pub name: Txt,
    /// Response button label.
    pub label: Var<Txt>,
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
            label: label.into_var(),
        }
    }

    /// "ok"
    pub fn ok() -> Self {
        Self::new("ok", l10n!("response-ok", "Ok"))
    }

    /// "cancel"
    pub fn cancel() -> Self {
        Self::new("cancel", l10n!("response-cancel", "Cancel"))
    }

    /// "yes"
    pub fn yes() -> Self {
        Self::new("yes", l10n!("response-yes", "Yes"))
    }
    /// "no"
    pub fn no() -> Self {
        Self::new("no", l10n!("response-no", "No"))
    }

    /// "close"
    pub fn close() -> Self {
        Self::new("close", l10n!("response-close", "Close"))
    }
}
impl_from_and_into_var! {
    fn from(native: native_api::MsgDialogResponse) -> Response {
        match native {
            native_api::MsgDialogResponse::Ok => Response::ok(),
            native_api::MsgDialogResponse::Yes => Response::yes(),
            native_api::MsgDialogResponse::No => Response::no(),
            native_api::MsgDialogResponse::Cancel => Response::cancel(),
            native_api::MsgDialogResponse::Error(e) => Response {
                name: Txt::from_static("native-error"),
                label: const_var(e),
            },
            _ => unimplemented!(),
        }
    }
    fn from(response: Response) -> Option<Response>;
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

/// Dialog service.
///
/// The non-custom dialog methods can be configured to open as native dialogs instead of the custom overlay dialogs.
///
/// # Panics
///
/// All dialog methods panic is not called inside a window.
pub struct DIALOG;
impl DIALOG {
    /// Show an info dialog with "Ok" button.
    pub fn info(&self, title: impl IntoVar<Txt>, msg: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.message(
            msg.into_var(),
            title.into_var(),
            DialogKind::INFO,
            &|| InfoStyle!(),
            native_api::MsgDialogIcon::Info,
            native_api::MsgDialogButtons::Ok,
        )
        .map_response(|_| ())
    }

    /// Show a warning dialog with "Ok" button.
    pub fn warn(&self, title: impl IntoVar<Txt>, msg: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.message(
            msg.into_var(),
            title.into_var(),
            DialogKind::WARN,
            &|| WarnStyle!(),
            native_api::MsgDialogIcon::Warn,
            native_api::MsgDialogButtons::Ok,
        )
        .map_response(|_| ())
    }

    /// Show an error dialog with "Ok" button.
    pub fn error(&self, title: impl IntoVar<Txt>, msg: impl IntoVar<Txt>) -> ResponseVar<()> {
        self.message(
            msg.into_var(),
            title.into_var(),
            DialogKind::ERROR,
            &|| ErrorStyle!(),
            native_api::MsgDialogIcon::Error,
            native_api::MsgDialogButtons::Ok,
        )
        .map_response(|_| ())
    }

    /// Shows a question dialog with "No" and "Yes" buttons. Returns `true` for "Yes".
    pub fn ask(&self, title: impl IntoVar<Txt>, question: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.message(
            question.into_var(),
            title.into_var(),
            DialogKind::ASK,
            &|| AskStyle!(),
            native_api::MsgDialogIcon::Info,
            native_api::MsgDialogButtons::YesNo,
        )
        .map_response(|r| r.name == "yes")
    }

    /// Shows a question dialog with "Cancel" and "Ok" buttons. Returns `true` for "Ok".
    pub fn confirm(&self, title: impl IntoVar<Txt>, question: impl IntoVar<Txt>) -> ResponseVar<bool> {
        self.message(
            question.into_var(),
            title.into_var(),
            DialogKind::CONFIRM,
            &|| ConfirmStyle!(),
            native_api::MsgDialogIcon::Warn,
            native_api::MsgDialogButtons::OkCancel,
        )
        .map_response(|r| r.name == "ok")
    }

    /// Shows a native file picker dialog configured to select one existing file.
    pub fn open_file(
        &self,
        title: impl IntoVar<Txt>,
        starting_dir: impl Into<PathBuf>,
        starting_name: impl IntoVar<Txt>,
        filters: impl Into<FileDialogFilters>,
    ) -> ResponseVar<FileDialogResponse> {
        WINDOWS.native_file_dialog(
            WINDOW.id(),
            native_api::FileDialog::new(
                title.into_var().get(),
                starting_dir.into(),
                starting_name.into_var().get(),
                filters.into().build(),
                native_api::FileDialogKind::OpenFile,
            ),
        )
    }

    /// Shows a native file picker dialog configured to select one or more existing files.
    pub fn open_files(
        &self,
        title: impl IntoVar<Txt>,
        starting_dir: impl Into<PathBuf>,
        starting_name: impl IntoVar<Txt>,
        filters: impl Into<FileDialogFilters>,
    ) -> ResponseVar<FileDialogResponse> {
        WINDOWS.native_file_dialog(
            WINDOW.id(),
            native_api::FileDialog::new(
                title.into_var().get(),
                starting_dir.into(),
                starting_name.into_var().get(),
                filters.into().build(),
                native_api::FileDialogKind::OpenFiles,
            ),
        )
    }

    /// Shows a native file picker dialog configured to select one file path that does not exist yet.
    pub fn save_file(
        &self,
        title: impl IntoVar<Txt>,
        starting_dir: impl Into<PathBuf>,
        starting_name: impl IntoVar<Txt>,
        filters: impl Into<FileDialogFilters>,
    ) -> ResponseVar<FileDialogResponse> {
        WINDOWS.native_file_dialog(
            WINDOW.id(),
            native_api::FileDialog::new(
                title.into_var().get(),
                starting_dir.into(),
                starting_name.into_var().get(),
                filters.into().build(),
                native_api::FileDialogKind::SaveFile,
            ),
        )
    }

    /// Shows a native file picker dialog configured to select one existing directory.
    pub fn select_folder(
        &self,
        title: impl IntoVar<Txt>,
        starting_dir: impl Into<PathBuf>,
        starting_name: impl IntoVar<Txt>,
    ) -> ResponseVar<FileDialogResponse> {
        WINDOWS.native_file_dialog(
            WINDOW.id(),
            native_api::FileDialog::new(
                title.into_var().get(),
                starting_dir.into(),
                starting_name.into_var().get(),
                "",
                native_api::FileDialogKind::SelectFolder,
            ),
        )
    }

    /// Shows a native file picker dialog configured to select one or more existing directories.
    pub fn select_folders(
        &self,
        title: impl IntoVar<Txt>,
        starting_dir: impl Into<PathBuf>,
        starting_name: impl IntoVar<Txt>,
    ) -> ResponseVar<FileDialogResponse> {
        WINDOWS.native_file_dialog(
            WINDOW.id(),
            native_api::FileDialog::new(
                title.into_var().get(),
                starting_dir.into(),
                starting_name.into_var().get(),
                "",
                native_api::FileDialogKind::SelectFolders,
            ),
        )
    }

    /// Open the custom `dialog`.
    ///
    /// Returns the selected response or [`close`] if the dialog is closed without response.
    ///
    /// [`close`]: Response::close
    pub fn custom(&self, dialog: impl IntoUiNode) -> ResponseVar<Response> {
        self.show_impl(dialog.into_node())
    }
}

impl DIALOG {
    /// Variable that defines what native dialogs are used when the dialog methods are called in window contexts.
    ///
    /// The [`native_dialogs`](fn@native_dialogs) context property can also be used to override the config just for some widgets.
    ///
    /// Note that some dialogs only have the native implementation as of this release.
    pub fn native_dialogs(&self) -> Var<DialogKind> {
        DIALOG_SV.read().native_dialogs.clone()
    }

    /// Native dialogs implemented by the current view-process.
    pub fn available_native_dialogs(&self) -> NativeDialogCapacity {
        VIEW_PROCESS.info().dialog
    }
}
bitflags! {
    /// Dialog kind options.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub struct DialogKind: u32 {
        /// [`DIALOG.info`](DIALOG::info)
        const INFO = 0b0000_0000_0000_0001;
        /// [`DIALOG.warn`](DIALOG::warn)
        const WARN = 0b0000_0000_0000_0010;
        /// [`DIALOG.error`](DIALOG::error)
        const ERROR = 0b0000_0000_0000_0100;
        /// [`DIALOG.ask`](DIALOG::ask)
        const ASK = 0b0000_0000_0000_1000;
        /// [`DIALOG.confirm`](DIALOG::confirm)
        const CONFIRM = 0b0000_0000_0001_0000;

        /// [`DIALOG.open_file`](DIALOG::open_file)
        const OPEN_FILE = 0b1000_0000_0000_0000;
        /// [`DIALOG.open_files`](DIALOG::open_files)
        const OPEN_FILES = 0b0100_0000_0000_0000;
        /// [`DIALOG.save_file`](DIALOG::save_file)
        const SAVE_FILE = 0b0010_0000_0000_0000;

        /// [`DIALOG.select_folder`](DIALOG::select_folder)
        const SELECT_FOLDER = 0b0001_0000_0000_0000;
        /// [`DIALOG.select_folders`](DIALOG::select_folders)
        const SELECT_FOLDERS = 0b0000_1000_0000_0000;

        /// All message dialogs.
        const MESSAGE = Self::INFO.bits() | Self::WARN.bits() | Self::ERROR.bits() | Self::ASK.bits() | Self::CONFIRM.bits();
        /// All file system dialogs.
        const FILE = Self::OPEN_FILE.bits()
            | Self::OPEN_FILES.bits()
            | Self::SAVE_FILE.bits()
            | Self::SELECT_FOLDER.bits()
            | Self::SELECT_FOLDERS.bits();
    }
}
impl_from_and_into_var! {
    fn from(empty_or_all: bool) -> DialogKind {
        if empty_or_all { DialogKind::all() } else { DialogKind::empty() }
    }
}

impl DIALOG {
    /// Close the contextual dialog with the `response``.
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

    /// Try to close the contextual dialog without directly setting a response.
    ///
    /// If the dialog has no [`default_response`](fn@default_response) the
    /// [`on_dialog_close_canceled`](fn@on_dialog_close_canceled) event notifies instead of closing.
    pub fn respond_default(&self) {
        let ctx = DIALOG_CTX.get();
        let id = *ctx.dialog_id.lock();
        if let Some(id) = id {
            POPUP.close_id(id);
        } else {
            tracing::error!("DIALOG.respond called outside of a dialog");
        }
    }

    fn message(
        &self,
        msg: Var<Txt>,
        title: Var<Txt>,
        kind: DialogKind,
        style: &dyn Fn() -> zng_wgt_style::StyleBuilder,
        native_icon: native_api::MsgDialogIcon,
        native_buttons: native_api::MsgDialogButtons,
    ) -> ResponseVar<Response> {
        if NATIVE_DIALOGS_VAR.get().contains(kind) {
            WINDOWS
                .native_message_dialog(
                    WINDOW.id(),
                    native_api::MsgDialog::new(title.get(), msg.get(), native_icon, native_buttons),
                )
                .map_response(|r| r.clone().into())
        } else {
            self.custom(Dialog! {
                style_fn = style();
                title = Text! {
                    visibility = title.map(|t| Visibility::from(!t.is_empty()));
                    txt = title;
                };
                content = SelectableText!(msg);
            })
        }
    }

    fn show_impl(&self, dialog: UiNode) -> ResponseVar<Response> {
        let (responder, response) = response_var();

        let mut ctx = Some(Arc::new(DialogCtx {
            dialog_id: Mutex::new(None),
            responder,
        }));

        let dialog = backdrop::DialogBackdrop!(dialog);

        let dialog = match_widget(
            dialog,
            clmv!(|c, op| {
                match &op {
                    UiNodeOp::Init => {
                        *ctx.as_ref().unwrap().dialog_id.lock() = c.node().as_widget().map(|mut w| w.id());
                        DIALOG_CTX.with_context(&mut ctx, || c.op(op));
                        // in case a non-standard dialog widget is used
                        *ctx.as_ref().unwrap().dialog_id.lock() = c.node().as_widget().map(|mut w| w.id());
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

struct DialogService {
    native_dialogs: Var<DialogKind>,
}
app_local! {
    static DIALOG_SV: DialogService = DialogService {
        native_dialogs: var(DialogKind::FILE),
    };
}
