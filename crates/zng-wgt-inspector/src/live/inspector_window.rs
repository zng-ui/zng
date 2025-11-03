use std::mem;

use zng_app::widget::{
    border::{BorderSide, BorderSides},
    builder::{Importance, PropertyArgs, PropertyInfo, WidgetType},
    inspector::{InspectorActualVars, InstanceItem},
};
use zng_color::Rgba;
use zng_ext_font::{FontStyle, FontWeight};
use zng_ext_input::focus::FOCUS;
use zng_ext_l10n::{l10n, lang};
use zng_ext_window::{WINDOWS, WindowRoot};
use zng_var::animation::easing;
use zng_wgt::{Wgt, border, corner_radius, margin, node::VarPresent as _, prelude::*, visibility};
use zng_wgt_container::{Container, child_align, padding};

use zng_wgt_fill::background_color;
use zng_wgt_filter::opacity;
use zng_wgt_input::{focus::focus_shortcut, gesture::click_shortcut, is_hovered};
use zng_wgt_rule_line::hr::Hr;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_size_offset::{size, width};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::Style;
use zng_wgt_text::{Text, font_family, lang};
use zng_wgt_text_input::TextInput;
use zng_wgt_toggle::{self as toggle, Toggle};
use zng_wgt_tooltip::{Tip, tooltip};
use zng_wgt_window as window;
use zng_wgt_wrap::Wrap;

#[cfg(feature = "image")]
use zng_wgt_button::Button;
#[cfg(feature = "image")]
use zng_wgt_dialog::{DIALOG, FileDialogFilters};

use super::data_model::*;

use super::HitSelect;

// l10n-## Inspector Window (always en-US)

/// New inspector window.
pub(super) fn new(
    inspected: WindowId,
    inspected_tree: InspectedTree,
    selected_wgt: Var<Option<InspectedWidget>>,
    hit_select: Var<HitSelect>,
    adorn_selected: Var<bool>,
    select_focused: Var<bool>,
) -> WindowRoot {
    let parent = WINDOWS.vars(inspected).unwrap().parent().get().unwrap_or(inspected);

    let vars = WINDOWS.vars(inspected).unwrap();

    let title = l10n!(
        "inspector/window.title",
        "{$inspected_window_title} - Inspector",
        inspected_window_title = vars.title()
    );
    let icon = vars.icon();

    let wgt_filter = var(Txt::from_static(""));

    // hit_select var is used to communicate with the `select_on_click` node on the inspected window.
    let hit_select_handle = hit_select.on_new(hn!(inspected_tree, selected_wgt, |a| {
        if let HitSelect::Select(id) = a.value {
            // clicked on a widget to select
            selected_wgt.set(inspected_tree.inspect(id));
        }
    }));

    let mut last_focused = None;
    let focus_selected = merge_var!(
        FOCUS.focused(),
        select_focused.clone(),
        clmv!(inspected_tree, selected_wgt, |focused, select| {
            if let Some(p) = focused
                && p.window_id() == inspected
            {
                last_focused = Some(p.widget_id())
            }

            if let Some(id) = last_focused
                && *select
            {
                selected_wgt.set(inspected_tree.inspect(id));
            }
        })
    );

    window::Window! {
        parent;
        title;
        icon;
        lang = lang!(en_US);
        width = 1100;
        set_inspected = inspected;
        color_scheme = ColorScheme::Dark;
        on_close = hn!(selected_wgt, |_| {
            selected_wgt.set(None);
        });
        child = Container! {
            child_top = menu(hit_select, adorn_selected, select_focused, wgt_filter.clone());
            child = Scroll! {
                toggle::selector = toggle::Selector::single_opt(selected_wgt.clone());
                child = tree_view(inspected_tree, wgt_filter.clone());
                child_align = Align::FILL_TOP;
                padding = 5;
            };
        };
        child_right = Container! {
            width = 600;
            child = selected_wgt.present(wgt_fn!(|w| { selected_view(w) }));
            background_color = SELECTED_BKG_VAR;
        };

        zng_wgt::on_deinit = hn!(|_| {
            let _keep_alive = (&hit_select_handle, &focus_selected);
        });
    }
}

/// Sets the inspected window on the inspector root widget info.
#[property(CONTEXT)]
fn set_inspected(child: impl IntoUiNode, inspected: impl IntoValue<WindowId>) -> UiNode {
    let inspected = inspected.into();
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            assert!(WIDGET.parent_id().is_none());
            info.set_meta(*INSPECTED_ID, inspected);
        }
    })
}

/// Gets the window that is inspected by the current inspector window.
pub fn inspected() -> Option<WindowId> {
    WINDOW.info().root().meta().get(*INSPECTED_ID).copied()
}

static_id! {
    pub(super) static ref INSPECTED_ID: StateId<WindowId>;
}

context_var! {
    static TREE_ITEM_BKG_HOVERED_VAR: Rgba = rgb(0.21, 0.21, 0.21);
    static TREE_ITEM_BKG_CHECKED_VAR: Rgba = rgb(0.29, 0.29, 0.29);
    static TREE_ITEM_LINE_VAR: Rgba = rgb(0.21, 0.21, 0.21);
    static WIDGET_ID_COLOR_VAR: Rgba = colors::GRAY;
    static WIDGET_MACRO_COLOR_VAR: Rgba = colors::AZURE;
    static PROPERTY_COLOR_VAR: Rgba = colors::YELLOW;
    static PROPERTY_VALUE_COLOR_VAR: Rgba = colors::ROSE.lighten(50.pct());
    static NEST_GROUP_COLOR_VAR: Rgba = colors::GRAY;
    static SELECTED_BKG_VAR: Rgba = rgb(0.15, 0.15, 0.15);
    static MENU_BKG_VAR: Rgba = rgb(0.13, 0.13, 0.13);
    pub static SELECTED_BORDER_VAR: Rgba = colors::AZURE;
}

fn menu(hit_test_select: Var<HitSelect>, adorn_selected: Var<bool>, select_focused: Var<bool>, search: Var<Txt>) -> UiNode {
    Container! {
        background_color = MENU_BKG_VAR;
        child_left = Stack! {
            padding = 4;
            spacing = 2;
            direction = StackDirection::left_to_right();
            toggle::style_fn = Style! {
                padding = 2;
                corner_radius = 2;
            };
            child_align = Align::CENTER;
            children = ui_vec![
                Toggle! {
                    child = crosshair_16x16();
                    tooltip = Tip!(Text!("select widget (Ctrl+Shift+C)"));
                    click_shortcut = shortcut!(CTRL | SHIFT + 'C');
                    checked = hit_test_select.map_bidi(
                        |c| matches!(c, HitSelect::Enabled),
                        |b| if *b { HitSelect::Enabled } else { HitSelect::Disabled },
                    );
                },
                Toggle! {
                    child = Wgt! {
                        size = 16;
                        border = {
                            widths: 3,
                            sides: SELECTED_BORDER_VAR.map_into(),
                        };
                    };
                    tooltip = Tip!(Text!("highlight selected widget"));
                    checked = adorn_selected;
                },
                Toggle! {
                    child = Wgt! {
                        size = 14;
                        corner_radius = 14;
                        border = {
                            widths: 1,
                            sides: SELECTED_BORDER_VAR.map(|c| BorderSides::dashed(*c)),
                        };
                    };
                    tooltip = Tip!(Text!("select focused widget"));
                    checked = select_focused;
                },
                #[cfg(feature = "image")]
                zng_wgt_rule_line::vr::Vr!(),
                #[cfg(feature = "image")]
                Toggle! {
                    child = zng_wgt::ICONS.get_or("menu", || Text!("☰"));
                    padding = 2;
                    checked = var(false);
                    checked_popup = {
                        let screenshot_idle = var(true);
                        wgt_fn!(screenshot_idle, |_| {
                            zng_wgt_menu::context::ContextMenu!(ui_vec![
                                Button! {
                                    child = Text!("Save Screenshot");
                                    zng_wgt_menu::icon = zng_wgt::ICONS.get("save");
                                    zng_wgt::enabled = screenshot_idle.clone();
                                    on_click = hn!(screenshot_idle, |_| {
                                        // not async_hn here because menu is dropped on click
                                        task::spawn(async_clmv!(screenshot_idle, {
                                            screenshot_idle.set(false);
                                            save_screenshot(inspected().unwrap()).await;
                                            screenshot_idle.set(true);
                                        }));
                                    });
                                },
                                Button! {
                                    child = Text!("Copy Screenshot");
                                    zng_wgt_menu::icon = zng_wgt::ICONS.get("copy");
                                    zng_wgt::enabled = screenshot_idle.clone();
                                    on_click = hn!(screenshot_idle, |_| {
                                        task::spawn(async_clmv!(screenshot_idle, {
                                            screenshot_idle.set(false);
                                            copy_screenshot(inspected().unwrap()).await;
                                            screenshot_idle.set(true);
                                        }));
                                    });
                                },
                            ])
                        })
                    };
                }
            ];
        };
        child = TextInput! {
            style_fn = zng_wgt_text_input::SearchStyle!();
            margin = (0, 0, 0, 50);
            padding = (3, 5);
            txt_align = Align::START;
            focus_shortcut = [shortcut!['S'], shortcut![CTRL + 'F'], shortcut![Find]];
            placeholder_txt = "search widgets (S)";
            txt = search;
        };
    }
}

fn crosshair_16x16() -> UiNode {
    match_node_leaf(|op| match op {
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = DipSize::splat(Dip::new(16)).to_px(LAYOUT.scale_factor());
        }
        UiNodeOp::Render { frame } => {
            let factor = frame.scale_factor();
            let a = Dip::new(2).to_px(factor);
            let b = Dip::new(16).to_px(factor);
            let m = b / Px(2) - a / Px(2);

            let color = FrameValue::Value(colors::WHITE);

            frame.push_color(PxRect::new(PxPoint::new(m, Px(0)), PxSize::new(a, b)), color);
            frame.push_color(PxRect::new(PxPoint::new(Px(0), m), PxSize::new(b, a)), color);
        }
        _ => {}
    })
}

/// Widgets tree view.
fn tree_view(tree: InspectedTree, filter: Var<Txt>) -> UiNode {
    Container! {
        font_family = ["JetBrains Mono", "Consolas", "monospace"];
        child = tree_item_view(tree.inspect_root(), filter, const_var(0u32));
    }
}

fn tree_item_view(wgt: InspectedWidget, filter: Var<Txt>, parent_desc_filter: Var<u32>) -> UiNode {
    let wgt_type = wgt.wgt_type();
    let wgt_id = wgt.id();

    let mut pass = false;
    let pass_filter = merge_var!(
        filter.clone(),
        wgt_type,
        clmv!(parent_desc_filter, |f, t| {
            let p = wgt_filter(f, *t, wgt_id);
            if p != pass {
                pass = p;
                parent_desc_filter.modify(move |c| {
                    if pass {
                        **c += 1;
                    } else {
                        **c -= 1;
                    }
                });
            }
            p
        })
    );

    let descendants_pass_filter = var(0u32);

    let prev_any_desc = std::sync::atomic::AtomicBool::new(false);
    descendants_pass_filter
        .hook(move |a| {
            let any_desc = 0 < *a.value();
            if any_desc != prev_any_desc.swap(any_desc, std::sync::atomic::Ordering::Relaxed) {
                parent_desc_filter.modify(move |c| {
                    if any_desc {
                        **c += 1;
                    } else {
                        **c -= 1;
                    }
                });
            }
            true
        })
        .perm();

    Container! {
        when !*#{pass_filter.clone()} && *#{descendants_pass_filter.clone()} == 0 {
            visibility = Visibility::Collapsed;
        }

        child = Toggle! {
            toggle::value = wgt.clone();

            style_fn = Style!(replace = true);
            padding = 2;
            when *#is_hovered {
                background_color = TREE_ITEM_BKG_HOVERED_VAR;
            }
            when *#toggle::is_checked {
                background_color = TREE_ITEM_BKG_CHECKED_VAR;
            }

            child = Wrap! {
                children = ui_vec![
                    Text! {
                        txt = wgt.wgt_macro_name();
                        font_weight = FontWeight::BOLD;
                        font_color = WIDGET_MACRO_COLOR_VAR;

                        when !*#{pass_filter.clone()} {
                            opacity = 50.pct();
                        }
                    },
                    Text!(" {{ "),
                    Text! {
                        txt = formatx!("{:#}", wgt.id());
                        font_color = WIDGET_ID_COLOR_VAR;
                    },
                    Text!(
                        wgt.descendants_len()
                            .map(|&l| if l == 0 { Txt::from_static(" }") } else { Txt::from_static("") })
                    ),
                ];
            };
        };
        child_spacing = 2;
        child_bottom =
            wgt.children()
                .present(wgt_fn!(descendants_pass_filter, |children: Vec<InspectedWidget>| {
                    let children: UiVec = children
                        .into_iter()
                        .map(|c| tree_item_view(c, filter.clone(), descendants_pass_filter.clone()))
                        .collect();
                    if children.is_empty() {
                        UiNode::nil()
                    } else {
                        Container! {
                            child = Stack! {
                                padding = (0, 0, 0, 2.em());
                                direction = StackDirection::top_to_bottom();
                                children;

                                border = {
                                    widths: (0, 0, 0, 1),
                                    sides: TREE_ITEM_LINE_VAR.map(|&c| BorderSides::new_left(BorderSide::dashed(c))),
                                };
                            };
                            child_bottom = Text!("}}");
                        }
                    }
                })),
        ;
    }
}

/// Selected widget properties, info.
fn selected_view(wgt: Option<InspectedWidget>) -> UiNode {
    if let Some(wgt) = wgt {
        Scroll! {
            mode = ScrollMode::VERTICAL;
            child_align = Align::FILL_TOP;
            padding = 4;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                font_family = ["JetBrains Mono", "Consolas", "monospace"];
                zng_wgt_text::rich_text = true;
                zng_wgt_text::txt_selectable = true;
                zng_wgt_input::cursor = zng_wgt_input::CursorIcon::Text;
                children = ui_vec![
                    Wrap! {
                        children = ui_vec![
                            Text! {
                                txt = wgt.wgt_macro_name();
                                font_size = 1.2.em();
                                font_weight = FontWeight::BOLD;
                                font_color = WIDGET_MACRO_COLOR_VAR;
                            },
                            Text! {
                                txt = formatx!(" {:#}", wgt.id());
                                font_size = 1.2.em();
                                font_color = WIDGET_ID_COLOR_VAR;
                            },
                            {
                                let parent_property = wgt.parent_property_name();
                                Wrap! {
                                    visibility = parent_property.map(|p| (!p.is_empty()).into());
                                    tooltip = Tip!(Text!("parent property"));
                                    children = ui_vec![
                                        Text!(" (in "),
                                        Text! {
                                            txt = parent_property;
                                            font_color = PROPERTY_COLOR_VAR;
                                        },
                                        Text!(")"),
                                    ];
                                }
                            },
                        ];
                    },
                    wgt.inspector_info().present(wgt_fn!(|i| {
                        if let Some(i) = i { inspector_info_view(i) } else { UiNode::nil() }
                    })),
                    Hr!(),
                    info_watchers(&wgt),
                ];
            };
        }
    } else {
        Text! {
            txt_align = Align::TOP;
            padding = 20;
            font_style = FontStyle::Italic;
            txt = l10n!("inspector/select-widget", "select a widget to inspect");
        }
    }
}

fn inspector_info_view(info: InspectedInfo) -> UiNode {
    let mut current_group = None;
    let mut group_items = UiVec::new();
    let mut out = UiVec::new();

    for item in info.items.iter() {
        match item {
            InstanceItem::Property { args, captured } => {
                let p_info = args.property();
                let user_assigned = info
                    .builder
                    .property(p_info.id)
                    .map(|p| p.importance == Importance::INSTANCE)
                    .unwrap_or_default();

                if current_group.as_ref() != Some(&p_info.group) {
                    if let Some(g) = current_group.take() {
                        out.push(nest_group_view(g, mem::take(&mut group_items)));
                    }
                    current_group = Some(p_info.group);
                }

                group_items.push(property_view(&info.actual_vars, &**args, p_info, *captured, user_assigned));
            }
            InstanceItem::Intrinsic { group, name } => {
                if current_group.as_ref() != Some(group) {
                    if let Some(g) = current_group.take() {
                        out.push(nest_group_view(g, mem::take(&mut group_items)));
                    }
                    current_group = Some(*group);
                }
                group_items.push(intrinsic_view(name));
            }
        }
    }

    if !group_items.is_empty() {
        out.push(nest_group_view(current_group.unwrap(), group_items));
    }

    Stack! {
        direction = StackDirection::top_to_bottom();
        children = out;
    }
}

fn nest_group_view(group: NestGroup, mut items: UiVec) -> UiNode {
    items.insert(
        0,
        Text! {
            txt = formatx!("// {}", group.name());
            tooltip = Tip!(Text!(l10n!("inspector/nest-group-help", "nest group")));
            margin = (10, 0, 0, 0);
            font_color = NEST_GROUP_COLOR_VAR;
        },
    );

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 3;
        children = items;
    }
}

fn value_background(value: &Var<Txt>) -> Var<Rgba> {
    let flash = var(rgba(0, 0, 0, 0));
    let mut _flashing = None;
    value
        .on_pre_new(hn!(flash, |_| {
            let h = flash.set_ease(colors::BLACK, colors::BLACK.transparent(), 500.ms(), easing::linear);
            _flashing = Some(h);
        }))
        .perm();
    flash
}

fn property_view(
    actual_vars: &InspectorActualVars,
    args: &dyn PropertyArgs,
    info: PropertyInfo,
    captured: bool,
    user_assigned: bool,
) -> UiNode {
    let mut children = ui_vec![
        Text! {
            txt = info.name;
            font_color = PROPERTY_COLOR_VAR;
            tooltip = Tip!(Text!(if captured { "captured property" } else { "property" }));
        },
        Text!(" = "),
    ];
    if info.inputs.len() == 1 {
        let value = actual_vars.get_debug(info.id, 0).unwrap_or_else(|| args.live_debug(0));
        let flash = value_background(&value);

        children.push(Text! {
            txt = value;
            paragraph_indent = (2.em(), true); // hang next lines
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
            tooltip = Tip!(Text!(if user_assigned { "instance value" } else { "intrinsic value" }));
        });
        children.push(Text!(";"));
    } else {
        children.push(Text!("{{\n"));
        for (i, input) in info.inputs.iter().enumerate() {
            children.push(Text!("    {}: ", input.name));

            let value = actual_vars.get_debug(info.id, i).unwrap_or_else(|| args.live_debug(i));
            let flash = value_background(&value);

            children.push(Text! {
                txt = value;
                font_color = PROPERTY_VALUE_COLOR_VAR;
                background_color = flash;
            });
            children.push(Text!(",\n"));
        }
        children.push(Text!("}};"));
    }

    Wrap! {
        children;
    }
}

fn intrinsic_view(name: &'static str) -> UiNode {
    Text! {
        txt = name;
        font_style = FontStyle::Italic;
        tooltip = Tip!(Text!(l10n!("inspector/intrinsic-help", "intrinsic node")));
    }
}

fn info_watchers(wgt: &InspectedWidget) -> UiNode {
    let mut children = UiVec::new();

    let mut sep = "";
    for (name, value) in INSPECTOR.build_watchers(wgt) {
        children.push(Text! {
            txt = formatx!("{sep}{name}: ");
        });
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            paragraph_indent = (2.em(), true); // hang next lines
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });
        sep = ",\n";
    }
    if !sep.is_empty() {
        // if any
        children.push(Text! {
            txt = ",";
        });
    }

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 3;
        children = ui_vec![
            Text! {
                txt = formatx!("/* INFO */");
                tooltip = Tip!(Text!(l10n!("inspector/info-help", "watched widget info")));
                font_color = NEST_GROUP_COLOR_VAR;
            },
            Wrap!(children),
        ];
    }
}

fn wgt_filter(filter: &str, wgt_ty: Option<WidgetType>, wgt_id: WidgetId) -> bool {
    if filter.is_empty() {
        return true;
    }

    if let Some(t) = wgt_ty {
        if let Some(tn) = filter.strip_suffix('!') {
            if t.name() == tn {
                return true;
            }
        } else if t.name().contains(filter) {
            return true;
        }
    }

    if wgt_id.name().contains(filter) {
        return true;
    }

    if let Some(f) = filter.strip_prefix('#')
        && let Ok(i) = f.parse::<u64>()
        && wgt_id.sequential() == i
    {
        return true;
    }

    false
}

#[cfg(feature = "image")]
async fn save_screenshot(inspected: WindowId) {
    let frame = WINDOWS.frame_image(inspected, None);

    let mut filters = FileDialogFilters::new();
    let encoders = zng_ext_image::IMAGES.available_encoders();
    for enc in &encoders {
        filters.push_filter(&enc.to_uppercase(), &[enc]);
    }
    filters.push_filter(
        l10n!("inspector/screenshot.save-dlg-filter", "Image Files").get().as_str(),
        &encoders,
    );

    let r = DIALOG.save_file(
        l10n!("inspector/screenshot.save-dlg-title", "Save Screenshot"),
        "",
        l10n!("inspector/screenshot.save-dlg-starting-name", "screenshot.png"),
        filters,
    );
    let path = match r.await {
        zng_view_api::dialog::FileDialogResponse::Selected(mut p) => p.remove(0),
        zng_view_api::dialog::FileDialogResponse::Cancel => return,
        zng_view_api::dialog::FileDialogResponse::Error(e) => {
            screenshot_error(e).await;
            return;
        }
        _ => return,
    };

    frame.wait_match(|f| !f.is_loading()).await;
    let frame = frame.get();

    if let Some(e) = frame.error() {
        screenshot_error(e).await;
    } else {
        let r = frame.save(path).await;
        if let Err(e) = r {
            screenshot_error(
                l10n!(
                    "inspector/screenshot.save-error",
                    "Screenshot save error. {$error}",
                    error = e.to_string()
                )
                .get(),
            )
            .await;
        }
    }
}

#[cfg(feature = "image")]
async fn copy_screenshot(inspected: WindowId) {
    let frame = WINDOWS.frame_image(inspected, None);

    frame.wait_match(|f| !f.is_loading()).await;
    let frame = frame.get();

    if let Some(e) = frame.error() {
        screenshot_error(e).await;
    } else {
        let r = zng_ext_clipboard::CLIPBOARD.set_image(frame).wait_rsp().await;
        if let Err(e) = r {
            screenshot_error(
                l10n!(
                    "inspector/screenshot.copy-error",
                    "Screenshot copy error. {$error}",
                    error = e.to_string()
                )
                .get(),
            )
            .await;
        }
    }
}

#[cfg(feature = "image")]
async fn screenshot_error(e: Txt) {
    DIALOG
        .error(l10n!("inspector/screenshot.error-dlg-title", "Screenshot Error"), e)
        .await;
}
