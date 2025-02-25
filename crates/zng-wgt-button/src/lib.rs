#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Button widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use std::any::TypeId;

use colors::{ACCENT_COLOR_VAR, BASE_COLOR_VAR};
use zng_app::event::CommandParam;
use zng_var::ReadOnlyContextVar;
use zng_wgt::{base_color, border, corner_radius, is_disabled, prelude::*};
use zng_wgt_access::{AccessRole, access_role, labelled_by_child};
use zng_wgt_container::{Container, child_align, padding};
use zng_wgt_fill::background_color;
use zng_wgt_filter::{child_opacity, saturate};
use zng_wgt_input::{
    CursorIcon, cursor,
    focus::FocusableMix,
    gesture::{ClickArgs, on_click, on_disabled_click},
    is_cap_hovered, is_pressed,
    pointer_capture::{CaptureMode, capture_pointer},
};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};
use zng_wgt_text::{FONT_COLOR_VAR, Text, font_color, underline};

#[cfg(feature = "tooltip")]
use zng_wgt_tooltip::{Tip, TooltipArgs, tooltip, tooltip_fn};

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
                #[cfg(feature = "tooltip")]
                let tooltip = wgt.property(property_id!(tooltip)).is_none() && wgt.property(property_id!(tooltip_fn)).is_none();
                #[cfg(not(feature = "tooltip"))]
                let tooltip = false;
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
                            #[cfg(feature = "tooltip")]
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
    /// Optional parameter for the button to use when notifying command.
    pub static CMD_PARAM_VAR: Option<CommandParam> = None;

    /// Widget function used when `cmd` is set and `child` is not.
    pub static CMD_CHILD_FN_VAR: WidgetFn<Command> = WidgetFn::new(default_cmd_child_fn);

    /// Widget function used when `cmd` is set and `tooltip_fn`, `tooltip` are not set.
    #[cfg(feature = "tooltip")]
    pub static CMD_TOOLTIP_FN_VAR: WidgetFn<CmdTooltipArgs> = WidgetFn::new(default_cmd_tooltip_fn);

    static CMD_VAR: Option<Command> = None;
}

#[cfg(feature = "tooltip")]
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
#[cfg(feature = "tooltip")]
impl std::ops::Deref for CmdTooltipArgs {
    type Target = TooltipArgs;

    fn deref(&self) -> &Self::Target {
        &self.tooltip
    }
}

/// Default [`CMD_CHILD_FN_VAR`].
pub fn default_cmd_child_fn(cmd: Command) -> impl UiNode {
    Text!(cmd.name())
}

#[cfg(feature = "tooltip")]
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
/// [`child`]: struct@Container#method.child
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
/// If `T` is `Option<CommandParam>` the param can be dynamically unset, otherwise the value is the param.
///
/// [`cmd`]: fn@cmd
#[property(CONTEXT, default(CMD_PARAM_VAR), widget_impl(Button))]
pub fn cmd_param<T: VarValue>(child: impl UiNode, cmd_param: impl IntoVar<T>) -> impl UiNode {
    if TypeId::of::<T>() == TypeId::of::<Option<CommandParam>>() {
        let cmd_param = *cmd_param
            .into_var()
            .boxed_any()
            .double_boxed_any()
            .downcast::<BoxedVar<Option<CommandParam>>>()
            .unwrap();
        with_context_var(child, CMD_PARAM_VAR, cmd_param).boxed()
    } else {
        with_context_var(
            child,
            CMD_PARAM_VAR,
            cmd_param.into_var().map(|p| Some(CommandParam::new(p.clone()))),
        )
        .boxed()
    }
}

/// Sets the widget function used to produce the button child when [`cmd`] is set and [`child`] is not.
///
/// [`cmd`]: fn@cmd
/// [`child`]: fn@zng_wgt_container::child
#[property(CONTEXT, default(CMD_CHILD_FN_VAR), widget_impl(Button))]
pub fn cmd_child_fn(child: impl UiNode, cmd_child: impl IntoVar<WidgetFn<Command>>) -> impl UiNode {
    with_context_var(child, CMD_CHILD_FN_VAR, cmd_child)
}

#[cfg(feature = "tooltip")]
/// Sets the widget function used to produce the button tooltip when [`cmd`] is set and tooltip is not.
///
/// [`cmd`]: fn@cmd
#[property(CONTEXT, default(CMD_TOOLTIP_FN_VAR), widget_impl(Button))]
pub fn cmd_tooltip_fn(child: impl UiNode, cmd_tooltip: impl IntoVar<WidgetFn<CmdTooltipArgs>>) -> impl UiNode {
    with_context_var(child, CMD_TOOLTIP_FN_VAR, cmd_tooltip)
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

            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));

            #[easing(150.ms())]
            background_color = BASE_COLOR_VAR.rgba();
            #[easing(150.ms())]
            border = {
                widths: 1,
                sides: BASE_COLOR_VAR.rgba_into(),
            };

            when *#is_cap_hovered {
                #[easing(0.ms())]
                background_color = BASE_COLOR_VAR.shade(1);
                #[easing(0.ms())]
                border = {
                    widths: 1,
                    sides: BASE_COLOR_VAR.shade_into(2),
                };
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = BASE_COLOR_VAR.shade(2);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}

/// Primary button style.
#[widget($crate::PrimaryStyle)]
pub struct PrimaryStyle(DefaultStyle);
impl PrimaryStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            base_color = ACCENT_COLOR_VAR.map(|c| c.shade(-2));
            zng_wgt_text::font_weight = zng_ext_font::FontWeight::BOLD;
        }
    }
}

/// Button light style.
#[widget($crate::LightStyle)]
pub struct LightStyle(DefaultStyle);
impl LightStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            border = unset!;
            padding = 7;

            #[easing(150.ms())]
            background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(0.pct()));

            when *#is_cap_hovered {
                #[easing(0.ms())]
                background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
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

            font_color = light_dark(colors::BLUE, web_colors::LIGHT_BLUE);
            cursor = CursorIcon::Pointer;
            access_role = AccessRole::Link;

            when *#is_cap_hovered {
                underline = 1, LineStyle::Solid;
            }

            when *#is_pressed {
                font_color = light_dark(web_colors::BROWN, colors::YELLOW);
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
