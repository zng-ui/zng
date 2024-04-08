#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Button widget.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use std::ops;

use zng_app::event::CommandParam;
use zng_var::ReadOnlyContextVar;
use zng_wgt::{border, corner_radius, is_disabled, prelude::*};
use zng_wgt_access::{access_role, labelled_by_child, AccessRole};
use zng_wgt_container::{child_align, padding, Container};
use zng_wgt_fill::background_color;
use zng_wgt_filter::{child_opacity, saturate};
use zng_wgt_input::{
    cursor,
    focus::FocusableMix,
    gesture::{on_click, on_disabled_click, ClickArgs},
    is_cap_hovered, is_pressed,
    pointer_capture::{capture_pointer, CaptureMode},
    CursorIcon,
};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};
use zng_wgt_text::{font_color, underline, Text};
use zng_wgt_tooltip::{tooltip, tooltip_fn, Tip, TooltipArgs};

/// A clickable container.
///
/// # Shorthand
///
/// The `Button!` macro provides a shorthand init that sets the command, `Button!(SOME_CMD)`.
#[widget($crate::Button {
    ($cmd:expr) => {
        cmd = $cmd;
    };
})]
pub struct Button(FocusableMix<StyleMix<Container>>);
impl Button {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            capture_pointer = true;
            labelled_by_child = true;
        }

        self.widget_builder().push_build_action(|wgt| {
            if let Some(cmd) = wgt.capture_var::<Command>(property_id!(Self::cmd)) {
                if wgt.property(property_id!(Self::child)).is_none() {
                    wgt.set_child(presenter(cmd.clone(), CMD_CHILD_FN_VAR));
                }

                let enabled = wgt.property(property_id!(zng_wgt::enabled)).is_none();
                let visibility = wgt.property(property_id!(zng_wgt::visibility)).is_none();
                wgt.push_intrinsic(
                    NestGroup::CONTEXT,
                    "cmd-context",
                    clmv!(cmd, |mut child| {
                        if enabled {
                            child = zng_wgt::enabled(child, cmd.flat_map(|c| c.is_enabled())).boxed();
                        }
                        if visibility {
                            child = zng_wgt::visibility(child, cmd.flat_map(|c| c.has_handlers()).map_into()).boxed();
                        }

                        with_context_var(child, CMD_VAR, cmd.map(|c| Some(*c)))
                    }),
                );

                let on_click = wgt.property(property_id!(Self::on_click)).is_none();
                let on_disabled_click = wgt.property(property_id!(on_disabled_click)).is_none();
                let tooltip = wgt.property(property_id!(tooltip)).is_none() && wgt.property(property_id!(tooltip_fn)).is_none();
                if on_click || on_disabled_click || tooltip {
                    wgt.push_intrinsic(
                        NestGroup::EVENT,
                        "cmd-event",
                        clmv!(cmd, |mut child| {
                            if on_click {
                                child = self::on_click(
                                    child,
                                    hn!(cmd, |args: &ClickArgs| {
                                        let cmd = cmd.get();
                                        if cmd.is_enabled_value() {
                                            if let Some(param) = CMD_PARAM_VAR.get() {
                                                cmd.notify_param(param);
                                            } else {
                                                cmd.notify();
                                            }
                                            args.propagation().stop();
                                        }
                                    }),
                                )
                                .boxed();
                            }
                            if on_disabled_click {
                                child = self::on_disabled_click(
                                    child,
                                    hn!(cmd, |args: &ClickArgs| {
                                        let cmd = cmd.get();
                                        if !cmd.is_enabled_value() {
                                            if let Some(param) = CMD_PARAM_VAR.get() {
                                                cmd.notify_param(param);
                                            } else {
                                                cmd.notify();
                                            }
                                            args.propagation().stop();
                                        }
                                    }),
                                )
                                .boxed();
                            }
                            if tooltip {
                                child = self::tooltip_fn(
                                    child,
                                    merge_var!(cmd, CMD_TOOLTIP_FN_VAR, |cmd, tt_fn| {
                                        if tt_fn.is_nil() {
                                            WidgetFn::nil()
                                        } else {
                                            wgt_fn!(cmd, tt_fn, |tooltip| { tt_fn(CmdTooltipArgs { tooltip, cmd }) })
                                        }
                                    }),
                                )
                                .boxed();
                            }
                            child
                        }),
                    );
                }
            }
        });
    }

    widget_impl! {
        /// Button click event.
        pub on_click(handler: impl WidgetHandler<ClickArgs>);

        /// If pointer interaction with other widgets is blocked while the button is pressed.
        ///
        /// Enabled by default in this widget.
        pub capture_pointer(mode: impl IntoVar<CaptureMode>);
    }
}
impl_style_fn!(Button);

context_var! {
    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(0.18, 0.18, 0.18), rgb(0.82, 0.82, 0.82));

    /// Optional parameter for the button to use when notifying command.
    pub static CMD_PARAM_VAR: Option<CommandParam> = None;

    /// Widget function used when `cmd` is set and `child` is not.
    pub static CMD_CHILD_FN_VAR: WidgetFn<Command> = WidgetFn::new(default_cmd_child_fn);

    /// Widget function used when `cmd` is set and `tooltip_fn`, `tooltip` are not set.
    pub static CMD_TOOLTIP_FN_VAR: WidgetFn<CmdTooltipArgs> = WidgetFn::new(default_cmd_tooltip_fn);

    static CMD_VAR: Option<Command> = None;
}

/// Arguments for [`cmd_tooltip_fn`].
///
/// [`cmd_tooltip_fn`]: fn@cmd_tooltip_fn
#[derive(Clone)]
pub struct CmdTooltipArgs {
    /// The tooltip arguments.
    pub tooltip: TooltipArgs,
    /// The command.
    pub cmd: Command,
}
impl ops::Deref for CmdTooltipArgs {
    type Target = TooltipArgs;

    fn deref(&self) -> &Self::Target {
        &self.tooltip
    }
}

/// Default [`CMD_CHILD_FN_VAR`].
pub fn default_cmd_child_fn(cmd: Command) -> impl UiNode {
    Text!(cmd.name())
}

/// Default [`CMD_TOOLTIP_FN_VAR`].
pub fn default_cmd_tooltip_fn(args: CmdTooltipArgs) -> impl UiNode {
    let info = args.cmd.info();
    let has_info = info.map(|s| !s.is_empty());
    let shortcut = args.cmd.shortcut().map(|s| match s.first() {
        Some(s) => s.to_txt(),
        None => Txt::from(""),
    });
    let has_shortcut = shortcut.map(|s| !s.is_empty());
    Tip! {
        child = Text! {
            zng_wgt::visibility = has_info.map_into();
            txt = info;
        };
        child_bottom = {
            node: Text! {
                font_weight = zng_ext_font::FontWeight::BOLD;
                zng_wgt::visibility = has_shortcut.map_into();
                txt = shortcut;
            },
            spacing: 4,
        };

        zng_wgt::visibility = expr_var!((*#{has_info} || *#{has_shortcut}).into())
    }
}

/// Sets the [`Command`] the button represents.
///
/// When this is set the button widget sets these properties if they are not set:
///
/// * [`child`]: Set to a widget produced by [`cmd_child_fn`](fn@cmd_child_fn), by default is `Text!(cmd.name())`.
/// * [`tooltip_fn`]: Set to a widget function provided by [`cmd_tooltip_fn`](fn@cmd_tooltip_fn), by default it
///    shows the command info and first shortcut.
/// * [`enabled`]: Set to `cmd.is_enabled()`.
/// * [`visibility`]: Set to `cmd.has_handlers().into()`.
/// * [`on_click`]: Set to a handler that notifies the command if `cmd.is_enabled()`.
/// * [`on_disabled_click`]: Set to a handler that notifies the command if `!cmd.is_enabled()`.
///
/// [`child`]: struct@Button#child
/// [`tooltip_fn`]: fn@tooltip_fn
/// [`Command`]: zng_app::event::Command
/// [`enabled`]: fn@zng_wgt::enabled
/// [`visibility`]: fn@zng_wgt::visibility
/// [`on_click`]: fn@on_click
/// [`on_disabled_click`]: fn@on_disabled_click
#[property(CHILD, capture, widget_impl(Button))]
pub fn cmd(cmd: impl IntoVar<Command>) {}

/// Optional command parameter for the button to use when notifying [`cmd`].
///
/// [`cmd`]: fn@cmd
#[property(CONTEXT, default(CMD_PARAM_VAR), widget_impl(Button))]
pub fn cmd_param(child: impl UiNode, cmd_param: impl IntoVar<Option<CommandParam>>) -> impl UiNode {
    with_context_var(child, CMD_PARAM_VAR, cmd_param)
}

/// Sets the widget function used to produce the button child when [`cmd`] is set and [`child`] is not.
///
/// [`cmd`]: fn@cmd
/// [`child`]: fn@zng_wgt_container::child
#[property(CONTEXT, default(CMD_CHILD_FN_VAR), widget_impl(Button))]
pub fn cmd_child_fn(child: impl UiNode, cmd_child: impl IntoVar<WidgetFn<Command>>) -> impl UiNode {
    with_context_var(child, CMD_CHILD_FN_VAR, cmd_child)
}

/// Sets the widget function used to produce the button tooltip when [`cmd`] is set and tooltip is not.
///
/// [`cmd`]: fn@cmd
#[property(CONTEXT, default(CMD_TOOLTIP_FN_VAR), widget_impl(Button))]
pub fn cmd_tooltip_fn(child: impl UiNode, cmd_tooltip: impl IntoVar<WidgetFn<CmdTooltipArgs>>) -> impl UiNode {
    with_context_var(child, CMD_TOOLTIP_FN_VAR, cmd_tooltip)
}

/// Sets the colors used to compute all background and border colors in the button style.
///
/// Sets the [`BASE_COLORS_VAR`].
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Create a [`color_scheme_highlight`] of `0.08`.
pub fn color_scheme_hovered(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
    color_scheme_highlight(pair, 0.08)
}

/// Create a [`color_scheme_highlight`] of `0.16`.
pub fn color_scheme_pressed(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
    color_scheme_highlight(pair, 0.16)
}

/// Button default style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            replace = true;

            access_role = AccessRole::Button;

            padding = (7, 15);
            corner_radius = 4;
            child_align = Align::CENTER;

            #[easing(150.ms())]
            background_color = color_scheme_pair(BASE_COLORS_VAR);

            #[easing(150.ms())]
            border = {
                widths: 1,
                sides: color_scheme_pair(BASE_COLORS_VAR).map_into()
            };

            when *#is_cap_hovered {
                #[easing(0.ms())]
                background_color = color_scheme_hovered(BASE_COLORS_VAR);
                #[easing(0.ms())]
                border = {
                    widths: 1,
                    sides: color_scheme_pressed(BASE_COLORS_VAR).map_into(),
                };
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = color_scheme_pressed(BASE_COLORS_VAR);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}

/// Button context.
pub struct BUTTON;
impl BUTTON {
    /// The [`cmd`] value, if set.
    ///
    /// [`cmd`]: fn@cmd
    pub fn cmd(&self) -> ReadOnlyContextVar<Option<Command>> {
        CMD_VAR.read_only()
    }

    /// The [`cmd_param`] value.
    ///
    /// [`cmd_param`]: fn@cmd_param
    pub fn cmd_param(&self) -> ReadOnlyContextVar<Option<CommandParam>> {
        CMD_PARAM_VAR.read_only()
    }
}

/// Button link style.
///
/// Looks like a web hyperlink.
#[widget($crate::LinkStyle)]
pub struct LinkStyle(Style);
impl LinkStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;

            font_color = color_scheme_map(web_colors::LIGHT_BLUE, colors::BLUE);
            cursor = CursorIcon::Pointer;
            access_role = AccessRole::Link;

            when *#is_cap_hovered {
                underline = 1, LineStyle::Solid;
            }

            when *#is_pressed {
                font_color = color_scheme_map(colors::YELLOW, web_colors::BROWN);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
