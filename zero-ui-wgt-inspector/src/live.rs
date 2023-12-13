use zero_ui_app::access::ACCESS_CLICK_EVENT;
use zero_ui_ext_input::{
    gesture::CLICK_EVENT,
    mouse::{MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT},
    touch::{TOUCHED_EVENT, TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT},
};
use zero_ui_ext_window::{WINDOW_Ext as _, WINDOWS};
use zero_ui_view_api::window::CursorIcon;
use zero_ui_wgt::prelude::*;

use crate::INSPECT_CMD;

pub fn inspect_node(can_inspect: impl IntoVar<bool>) -> impl UiNode {
    let mut inspected_tree = None::<data_model::InspectedTree>;
    let inspector = WindowId::new_unique();

    let selected_wgt = var(None);
    let hit_select = var(HitSelect::Disabled);
    let show_selected = var(true);

    let can_inspect = can_inspect.into_var();
    let mut cmd_handle = CommandHandle::dummy();

    enum InspectorUpdateOnly {
        Info,
        Render,
    }

    let child = match_node_leaf(clmv!(selected_wgt, hit_select, show_selected, |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&can_inspect);
            cmd_handle = INSPECT_CMD.scoped(WINDOW.id()).subscribe_wgt(can_inspect.get(), WIDGET.id());
        }
        UiNodeOp::Update { .. } => {
            if let Some(e) = can_inspect.get_new() {
                cmd_handle.set_enabled(e);
            }
        }
        UiNodeOp::Info { .. } => {
            if inspected_tree.is_some() {
                if WINDOWS.is_open(inspector) {
                    INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Info);
                } else if !WINDOWS.is_opening(inspector) {
                    inspected_tree = None;
                }
            }
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = INSPECT_CMD.scoped(WINDOW.id()).on_unhandled(update) {
                args.propagation().stop();

                if let Some(u) = args.param::<InspectorUpdateOnly>() {
                    if let Some(i) = &inspected_tree {
                        match u {
                            InspectorUpdateOnly::Info => i.update(WINDOW.info()),
                            InspectorUpdateOnly::Render => i.update_render(),
                        }
                    }
                } else if let Some(inspected) = inspector_window::inspected() {
                    // can't inspect inspector window, redirect command to inspected
                    INSPECT_CMD.scoped(inspected).notify();
                } else {
                    let inspected_tree = match &inspected_tree {
                        Some(i) => {
                            i.update(WINDOW.info());
                            i.clone()
                        }
                        None => {
                            let i = data_model::InspectedTree::new(WINDOW.info());
                            inspected_tree = Some(i.clone());
                            i
                        }
                    };

                    let inspected = WINDOW.id();
                    WINDOWS.focus_or_open(
                        inspector,
                        async_clmv!(inspected_tree, selected_wgt, hit_select, show_selected, {
                            inspector_window::new(inspected, inspected_tree, selected_wgt, hit_select, show_selected)
                        }),
                    );
                }
            }
        }
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Render);
        }
        _ => {}
    }));

    let child = adorn_selected(child, selected_wgt, show_selected);
    select_on_click(child, hit_select)
}

fn adorn_selected(child: impl UiNode, selected_wgt: impl Var<Option<data_model::InspectedWidget>>, enabled: impl Var<bool>) -> impl UiNode {
    use inspector_window::SELECTED_BORDER_VAR;

    let selected_info = selected_wgt.flat_map(|s| {
        if let Some(s) = s {
            s.info().map(|i| Some(i.clone())).boxed()
        } else {
            var(None).boxed()
        }
    });
    let transform_id = SpatialFrameId::new_unique();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&selected_info)
                .sub_var_render(&enabled)
                .sub_var_render(&SELECTED_BORDER_VAR);
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            if !enabled.get() {
                return;
            }
            selected_info.with(|w| {
                if let Some(w) = w {
                    let bounds = w.bounds_info();
                    let transform = bounds.inner_transform();
                    let size = bounds.inner_size();

                    frame.push_reference_frame(transform_id.into(), transform.into(), false, false, |frame| {
                        let widths = Dip::new(3).to_px(frame.scale_factor());
                        frame.push_border(
                            PxRect::from_size(size).inflate(widths, widths),
                            PxSideOffsets::new_all_same(widths),
                            SELECTED_BORDER_VAR.get().into(),
                            PxCornerRadius::default(),
                        );
                    });
                }
            });
        }
        _ => {}
    })
}

fn select_on_click(child: impl UiNode, hit_select: impl Var<HitSelect>) -> impl UiNode {
    // when `pending` we need to block interaction with window content, as if a modal
    // overlay was opened, but we can't rebuild info, and we actually want the click target,
    // so we only manually block common pointer events.

    let mut click_handle = EventHandles::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&hit_select);
        }
        UiNodeOp::Update { .. } => {
            if let Some(h) = hit_select.get_new() {
                if matches!(h, HitSelect::Enabled) {
                    WINDOW.vars().cursor().set(CursorIcon::Crosshair);
                    click_handle.push(MOUSE_INPUT_EVENT.subscribe(WIDGET.id()));
                    click_handle.push(TOUCH_INPUT_EVENT.subscribe(WIDGET.id()));
                } else {
                    WINDOW.vars().cursor().set(CursorIcon::Default);
                    click_handle.clear();
                }
            }
        }
        UiNodeOp::Event { update } => {
            if matches!(hit_select.get(), HitSelect::Enabled) {
                let mut select = None;

                if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = MOUSE_WHEEL_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = ACCESS_CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = TOUCHED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_TAP_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_TRANSFORM_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_LONG_PRESS_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                }

                if let Some(id) = select {
                    let _ = hit_select.set(HitSelect::Select(id));
                }
            }
        }
        _ => {}
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HitSelect {
    Disabled,
    Enabled,
    Select(WidgetId),
}

mod inspector_window {
    use std::mem;

    use zero_ui_app::widget::{
        border::{BorderSide, BorderSides},
        builder::{Importance, PropertyArgs, PropertyInfo, WidgetType},
        inspector::{InspectorContext, InstanceItem, WidgetInfoInspectorExt as _},
        OnVarArgs,
    };
    use zero_ui_color::RenderColor;
    use zero_ui_ext_font::{FontStyle, FontWeight};
    use zero_ui_ext_l10n::lang;
    use zero_ui_ext_window::{WindowIcon, WindowRoot, WINDOWS};
    use zero_ui_var::{animation::easing, var_from};
    use zero_ui_wgt::{border, corner_radius, margin, prelude::*, visibility, Wgt};
    use zero_ui_wgt_access::{access_role, AccessRole};
    use zero_ui_wgt_container::{child_align, padding, Container};
    use zero_ui_wgt_fill::{background, background_color};
    use zero_ui_wgt_filter::opacity;
    use zero_ui_wgt_input::{focus::focus_shortcut, gesture::click_shortcut, is_hovered};
    use zero_ui_wgt_rule_line::hr::Hr;
    use zero_ui_wgt_scroll::{Scroll, ScrollMode};
    use zero_ui_wgt_size_offset::{size, width};
    use zero_ui_wgt_stack::{Stack, StackDirection};
    use zero_ui_wgt_style::{Style, StyleFn};
    use zero_ui_wgt_text::{font_family, lang, Text};
    use zero_ui_wgt_text_input::TextInput;
    use zero_ui_wgt_toggle::{self as toggle, Toggle};
    use zero_ui_wgt_tooltip::{tooltip, Tip};
    use zero_ui_wgt_view::{presenter, wgt_fn};
    use zero_ui_wgt_window as window;
    use zero_ui_wgt_wrap::Wrap;

    use super::data_model::*;

    use super::HitSelect;

    pub(super) fn new(
        inspected: WindowId,
        inspected_tree: InspectedTree,
        selected_wgt: impl Var<Option<InspectedWidget>>,
        hit_select: impl Var<HitSelect>,
        adorn_selected: impl Var<bool>,
    ) -> WindowRoot {
        let parent = WINDOWS.vars(inspected).unwrap().parent().get().unwrap_or(inspected);

        let tree = WINDOWS.widget_tree(inspected).unwrap();
        let title = if let Some(title) = tree.root().inspect_property(property_id!(window::title)) {
            title.downcast_var::<Txt>(0).map(|t| formatx!("{t} - Inspector")).boxed()
        } else {
            var_from("Inspector").boxed()
        };
        let icon = if let Some(icon) = tree.root().inspect_property(property_id!(window::icon)) {
            icon.downcast_var::<WindowIcon>(0).clone().boxed()
        } else {
            var(WindowIcon::Default).boxed()
        };

        let wgt_filter = var(Txt::from_static(""));

        let data_handle = hit_select.on_new(app_hn!(inspected_tree, selected_wgt, |a: &OnVarArgs<HitSelect>, _| {
            if let HitSelect::Select(id) = a.value {
                let _ = selected_wgt.set(inspected_tree.inspect(id));
            }
        }));

        window::Window! {
            parent;
            title;
            icon;
            lang = lang!(en_US);
            width = 1100;
            set_inspected = inspected;
            color_scheme = ColorScheme::Dark;
            on_close = hn!(selected_wgt, |_| {
                let _ = selected_wgt.set(None);
            });
            child = Container! {
                child_insert_above = menu(hit_select, adorn_selected, wgt_filter.clone()), 0;
                child = Scroll! {
                    toggle::selector = toggle::Selector::single_opt(selected_wgt.clone());
                    child = tree_view(inspected_tree, wgt_filter.clone());
                    child_align = Align::FILL_TOP;
                    padding = 5;
                };
            };
            child_insert_right = Container! {
                width = 600;
                child = presenter(selected_wgt, wgt_fn!(|w| {
                    selected_view(w).boxed()
                }));
                background_color = SELECTED_BKG_VAR;
            }, 0;

            zero_ui_wgt_data::data = data_handle; // keep alive
        }
    }

    #[property(CONTEXT)]
    fn set_inspected(child: impl UiNode, inspected: impl IntoValue<WindowId>) -> impl UiNode {
        let inspected = inspected.into();
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op {
                assert!(WIDGET.parent_id().is_none());
                info.set_meta(&INSPECTED_ID, inspected);
            }
        })
    }

    /// Gets the window that is inspected by the current inspector window.
    pub fn inspected() -> Option<WindowId> {
        WINDOW.info().root().meta().get(&INSPECTED_ID).copied()
    }

    pub(super) static INSPECTED_ID: StaticStateId<WindowId> = StaticStateId::new_unique();

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

    fn menu(hit_test_select: impl Var<HitSelect>, adorn_selected: impl Var<bool>, search: impl Var<Txt>) -> impl UiNode {
        Container! {
            background_color = MENU_BKG_VAR;
            child_insert_left = Stack! {
                padding = 4;
                spacing = 2;
                direction = StackDirection::left_to_right();
                toggle::extend_style = Style! {
                    padding = 2;
                    corner_radius = 2;
                };
                child_align = Align::CENTER;
                children = ui_vec![
                    Toggle! {
                        child = crosshair_16x16();
                        tooltip = Tip!(Text!("select widget (Ctrl+Shift+C)"));
                        click_shortcut = shortcut!(CTRL|SHIFT+'C');
                        checked = hit_test_select.map_bidi(
                            |c| matches!(c, HitSelect::Enabled),
                            |b| if *b { HitSelect::Enabled } else { HitSelect::Disabled }
                        );
                    },
                    Toggle! {
                        child = Wgt! {
                            size = 16;
                            border = {
                                widths: 3,
                                sides: SELECTED_BORDER_VAR.map_into(),
                            }
                        };
                        tooltip = Tip!(Text!("highlight selected widget"));
                        checked = adorn_selected;
                    },

                ]
            }, 0;
            child = TextInput! {
                access_role = AccessRole::SearchBox;
                margin = (0, 0, 0, 50);
                padding = (3, 5);
                focus_shortcut = [shortcut!['S'], shortcut![CTRL+'F'], shortcut![Find]];
                background = Text! {
                    padding = (4, 6); // +1 border
                    txt = "search widgets (S)";
                    opacity = 50.pct();
                    visibility = search.map(|t| t.is_empty().into());
                };
                txt = search;
            }
        }
    }

    fn crosshair_16x16() -> impl UiNode {
        match_node_leaf(|op| match op {
            UiNodeOp::Layout { final_size, .. } => {
                *final_size = DipSize::splat(Dip::new(16)).to_px(LAYOUT.scale_factor());
            }
            UiNodeOp::Render { frame } => {
                let factor = frame.scale_factor();
                let a = Dip::new(2).to_px(factor);
                let b = Dip::new(16).to_px(factor);
                let m = b / Px(2) - a / Px(2);

                let color = FrameValue::Value(RenderColor::from(colors::WHITE));

                frame.push_color(PxRect::new(PxPoint::new(m, Px(0)), PxSize::new(a, b)), color);
                frame.push_color(PxRect::new(PxPoint::new(Px(0), m), PxSize::new(b, a)), color);
            }
            _ => {}
        })
    }

    fn tree_view(tree: InspectedTree, filter: impl Var<Txt>) -> impl UiNode {
        Container! {
            font_family = ["JetBrains Mono", "Consolas", "monospace"];
            child = tree_item_view(tree.inspect_root(), filter, LocalVar(0u32).boxed());
        }
    }

    fn tree_item_view(wgt: InspectedWidget, filter: impl Var<Txt>, parent_desc_filter: BoxedVar<u32>) -> impl UiNode {
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
                    let _ = parent_desc_filter.modify(move |c| {
                        if pass {
                            *c.to_mut() += 1;
                        } else {
                            *c.to_mut() -= 1;
                        }
                    });
                }
                p
            })
        );

        let descendants_pass_filter = var(0u32).boxed();

        let prev_any_desc = std::sync::atomic::AtomicBool::new(false);
        descendants_pass_filter
            .hook(Box::new(move |a| {
                let any_desc = 0 < *a.value().as_any().downcast_ref::<u32>().unwrap();
                if any_desc != prev_any_desc.swap(any_desc, std::sync::atomic::Ordering::Relaxed) {
                    let _ = parent_desc_filter.modify(move |c| {
                        if any_desc {
                            *c.to_mut() += 1;
                        } else {
                            *c.to_mut() -= 1;
                        }
                    });
                }
                true
            }))
            .perm();

        Container! {
            when !*#{pass_filter.clone()} && *#{descendants_pass_filter.clone()} == 0 {
                visibility = Visibility::Collapsed;
            }

            child = Toggle! {
                toggle::value = wgt.clone();

                style_fn = StyleFn::nil();
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
                        Text!(wgt.descendants_len().map(|&l| if l == 0 { Txt::from_static(" }") } else { Txt::from_static("") })),
                    ];
                }
            };

            child_insert_below = presenter(wgt.children(), wgt_fn!(descendants_pass_filter, |children: Vec<InspectedWidget>| {
                let children: UiNodeVec = children.into_iter().map(|c| {
                    tree_item_view(c, filter.clone(), descendants_pass_filter.clone())
                }).collect();
                if children.is_empty() {
                    NilUiNode.boxed()
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
                        child_insert_below = Text!("}}"), 0;
                    }.boxed()

                }

            })), 2;
        }
    }

    fn selected_view(wgt: Option<InspectedWidget>) -> impl UiNode {
        if let Some(wgt) = wgt {
            Scroll! {
                mode = ScrollMode::VERTICAL;
                child_align = Align::FILL_TOP;
                padding = 4;
                child = Stack! {
                    direction = StackDirection::top_to_bottom();
                    font_family = ["JetBrains Mono", "Consolas", "monospace"];
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
                                        ]
                                    }
                                },
                            ]
                        },
                        presenter(wgt.inspector_info(), wgt_fn!(|i| {
                            if let Some(i) = i {
                                inspector_info_view(i).boxed()
                            } else {
                                NilUiNode.boxed()
                            }
                        })),
                        Hr!(),
                        info_watchers(&wgt),
                    ]
                }
            }
        } else {
            Text! {
                txt_align = Align::TOP;
                padding = 20;
                font_style = FontStyle::Italic;
                txt = formatx!("select a widget to inspect");
            }
        }
    }

    fn inspector_info_view(info: InspectedInfo) -> impl UiNode {
        let mut current_group = None;
        let mut group_items = UiNodeVec::new();
        let mut out = UiNodeVec::new();
        let ctx = &info.context;

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

                    group_items.push(property_view(ctx, &**args, p_info, *captured, user_assigned));
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

    fn nest_group_view(group: NestGroup, mut items: UiNodeVec) -> impl UiNode {
        items.insert(
            0,
            Text! {
                txt = formatx!("// {}", group.name());
                tooltip = Tip!(Text!("nest group"));
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

    fn value_background(value: &BoxedVar<Txt>) -> impl Var<Rgba> {
        let flash = var(rgba(0, 0, 0, 0));
        let mut _flashing = None;
        value
            .on_pre_new(app_hn!(flash, |_, _| {
                let h = flash.set_ease(colors::BLACK, colors::BLACK.transparent(), 500.ms(), easing::linear);
                _flashing = Some(h);
            }))
            .perm();
        flash
    }

    fn property_view(
        ctx: &InspectorContext,
        args: &dyn PropertyArgs,
        info: PropertyInfo,
        captured: bool,
        user_assigned: bool,
    ) -> impl UiNode {
        // TODO, indicators for user or widget set properties.
        let mut ctx = ctx.latest_capture();
        let mut children = ui_vec![
            Text! {
                txt = info.name;
                font_color = PROPERTY_COLOR_VAR;
                tooltip = Tip!(Text!(if captured {
                    "captured property"
                } else {
                    "property"
                }));
            },
            Text!(" = "),
        ];
        if info.inputs.len() == 1 {
            let value = ctx.with_context(|| args.live_debug(0));
            let flash = value_background(&value);

            children.push(Text! {
                txt = value;
                font_color = PROPERTY_VALUE_COLOR_VAR;
                background_color = flash;
                tooltip = Tip!(Text!(if user_assigned {
                    "instance value"
                } else {
                    "intrinsic value"
                }))
            });
            children.push(Text!(";"));
        } else {
            children.push(Text!("{{\n"));
            for (i, input) in info.inputs.iter().enumerate() {
                children.push(Text!("    {}: ", input.name));

                let value = ctx.with_context(|| args.live_debug(i));
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

    fn intrinsic_view(name: &'static str) -> impl UiNode {
        Text! {
            txt = name;
            font_style = FontStyle::Italic;
            tooltip = Tip!(Text!("intrinsic node"));
        }
    }

    fn info_watchers(wgt: &InspectedWidget) -> impl UiNode {
        let mut children = UiNodeVec::new();
        children.push(Text! {
            txt = "interactivity: ";
        });

        let value = wgt.info().map(|i| formatx!("{:?}", i.interactivity())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });

        children.push(Text! {
            txt = ",\nvisibility: ";
        });
        let value = wgt.render_watcher(|i| formatx!("{:?}", i.visibility())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });

        children.push(Text! {
            txt = ",\ninner_bounds: ";
        });
        let value = wgt.render_watcher(|i| formatx!("{:?}", i.bounds_info().inner_bounds())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });
        children.push(Text! {
            txt = ",";
        });

        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 3;
            children = ui_vec![
                Text! {
                    txt = formatx!("/* INFO */");
                    tooltip = Tip!(Text!("watched widget info"));
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

        if let Some(f) = filter.strip_prefix('#') {
            if let Ok(i) = f.parse::<u64>() {
                if wgt_id.sequential() == i {
                    return true;
                }
            }
        }

        false
    }
}

mod data_model {
    use std::{fmt, ops, sync::Arc};

    use parking_lot::Mutex;
    use zero_ui_app::widget::{
        builder::WidgetType,
        info::WidgetInfoTree,
        inspector::{InspectorInfo, WidgetInfoInspectorExt},
    };
    use zero_ui_var::{types::WeakArcVar, WeakVar};
    use zero_ui_view_api::window::FrameId;
    use zero_ui_wgt::prelude::*;

    #[derive(Default)]
    pub struct InspectedTreeData {
        widgets: IdMap<WidgetId, InspectedWidget>,
        latest_frame: Option<ArcVar<FrameId>>,
    }

    /// Represents an actively inspected widget tree.
    #[derive(Clone)]
    pub struct InspectedTree {
        tree: ArcVar<WidgetInfoTree>,
        data: Arc<Mutex<InspectedTreeData>>,
    }
    impl fmt::Debug for InspectedTree {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("InspectedTree")
                .field("tree", &self.tree.get())
                .finish_non_exhaustive()
        }
    }
    impl PartialEq for InspectedTree {
        fn eq(&self, other: &Self) -> bool {
            self.tree.var_ptr() == other.tree.var_ptr()
        }
    }
    impl InspectedTree {
        /// Initial inspection.
        pub fn new(tree: WidgetInfoTree) -> Self {
            Self {
                data: Arc::new(Mutex::new(InspectedTreeData::default())),
                tree: var(tree),
            }
        }

        /// Update inspection.
        ///
        /// # Panics
        ///
        /// Panics if info is not for the same window ID.
        pub fn update(&self, tree: WidgetInfoTree) {
            assert_eq!(self.tree.with(|t| t.window_id()), tree.window_id());

            // update and retain
            self.tree.set(tree.clone());

            let mut data = self.data.lock();
            let mut removed = false;
            for (k, v) in data.widgets.iter() {
                if let Some(w) = tree.get(*k) {
                    v.update(w);
                } else {
                    v.removed.set(true);
                    removed = true;
                }
            }
            // update can drop children inspectors so we can't update inside the retain closure.
            data.widgets
                .retain(|k, v| v.info.strong_count() > 1 && (!removed || tree.get(*k).is_some()));

            if let Some(f) = &data.latest_frame {
                if f.strong_count() == 1 {
                    data.latest_frame = None;
                } else {
                    f.set(tree.stats().last_frame);
                }
            }
        }

        /// Update all render watcher variables.
        pub fn update_render(&self) {
            let mut data = self.data.lock();
            if let Some(f) = &data.latest_frame {
                if f.strong_count() == 1 {
                    data.latest_frame = None;
                } else {
                    f.set(self.tree.with(|t| t.stats().last_frame));
                }
            }
        }

        /// Create a weak reference to this tree.
        pub fn downgrade(&self) -> WeakInspectedTree {
            WeakInspectedTree {
                tree: self.tree.downgrade(),
                data: Arc::downgrade(&self.data),
            }
        }

        /// Gets a widget inspector if the widget is in the latest info.
        pub fn inspect(&self, widget_id: WidgetId) -> Option<InspectedWidget> {
            match self.data.lock().widgets.entry(widget_id) {
                IdEntry::Occupied(e) => Some(e.get().clone()),
                IdEntry::Vacant(e) => self.tree.with(|t| {
                    t.get(widget_id)
                        .map(|w| e.insert(InspectedWidget::new(w, self.downgrade())).clone())
                }),
            }
        }

        /// Gets a widget inspector for the root widget.
        pub fn inspect_root(&self) -> InspectedWidget {
            self.inspect(self.tree.with(|t| t.root().id())).unwrap()
        }

        /// Latest frame updated using [`update_render`].
        ///
        /// [`update_render`]: Self::update_render
        pub fn last_frame(&self) -> impl Var<FrameId> {
            let mut data = self.data.lock();
            data.latest_frame
                .get_or_insert_with(|| var(self.tree.with(|t| t.stats().last_frame)))
                .clone()
        }
    }

    /// Represents a weak reference to a [`InspectedTree`].
    #[derive(Clone)]
    pub struct WeakInspectedTree {
        tree: WeakArcVar<WidgetInfoTree>,
        data: std::sync::Weak<Mutex<InspectedTreeData>>,
    }
    impl WeakInspectedTree {
        /// Try to get a strong reference to the inspected tree.
        pub fn upgrade(&self) -> Option<InspectedTree> {
            Some(InspectedTree {
                tree: self.tree.upgrade()?,
                data: self.data.upgrade()?,
            })
        }
    }

    struct InspectedWidgetCache {
        tree: WeakInspectedTree,
        children: Option<BoxedVar<Vec<InspectedWidget>>>,
        parent_property_name: Option<BoxedVar<Txt>>,
    }

    /// Represents an actively inspected widget.
    ///
    /// See [`InspectedTree::inspect`].
    #[derive(Clone)]
    pub struct InspectedWidget {
        info: ArcVar<WidgetInfo>,
        removed: ArcVar<bool>,
        cache: Arc<Mutex<InspectedWidgetCache>>,
    }
    impl fmt::Debug for InspectedWidget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("InspectedWidget")
                .field("info", &self.info.get())
                .field("removed", &self.removed.get())
                .finish_non_exhaustive()
        }
    }
    impl PartialEq for InspectedWidget {
        fn eq(&self, other: &Self) -> bool {
            self.info.var_ptr() == other.info.var_ptr()
        }
    }
    impl Eq for InspectedWidget {}
    impl InspectedWidget {
        /// Initial inspection.
        fn new(info: WidgetInfo, tree: WeakInspectedTree) -> Self {
            Self {
                info: var(info),
                removed: var(false),
                cache: Arc::new(Mutex::new(InspectedWidgetCache {
                    tree,
                    children: None,
                    parent_property_name: None,
                })),
            }
        }

        /// Update inspection.
        ///
        /// # Panics
        ///
        /// Panics if info is not for the same widget ID.
        fn update(&self, info: WidgetInfo) {
            assert_eq!(self.info.with(|i| i.id()), info.id());
            self.info.set(info);

            let mut cache = self.cache.lock();
            if let Some(c) = &cache.children {
                if c.strong_count() == 1 {
                    cache.children = None;
                }
            }
            if let Some(c) = &cache.parent_property_name {
                if c.strong_count() == 1 {
                    cache.parent_property_name = None;
                }
            }
        }

        // /// If this widget inspector is permanently disconnected and will not update.
        // ///
        // /// This is set to `true` when an inspected widget is not found after an update, when `true`
        // /// this inspector will not update even if the same widget ID is re-inserted in another update.
        // pub fn removed(&self) -> impl Var<bool> {
        //     self.removed.read_only()
        // }

        /// Latest info.
        pub fn info(&self) -> impl Var<WidgetInfo> {
            self.info.read_only()
        }

        /// Widget id.
        pub fn id(&self) -> WidgetId {
            self.info.with(|i| i.id())
        }

        // /// Count of ancestor widgets.
        // pub fn depth(&self) -> impl Var<usize> {
        //     self.info.map(|w| w.depth()).actual_var()
        // }

        /// Count of descendant widgets.
        pub fn descendants_len(&self) -> impl Var<usize> {
            self.info.map(|w| w.descendants_len()).actual_var()
        }

        /// Widget type, if the widget was built with inspection info.
        pub fn wgt_type(&self) -> impl Var<Option<WidgetType>> {
            self.info.map(|w| Some(w.inspector_info()?.builder.widget_type())).actual_var()
        }

        /// Widget macro name, or `"<widget>!"` if widget was not built with inspection info.
        pub fn wgt_macro_name(&self) -> impl Var<Txt> {
            self.info
                .map(|w| match w.inspector_info().map(|i| i.builder.widget_type()) {
                    Some(t) => formatx!("{}!", t.name()),
                    None => Txt::from_static("<widget>!"),
                })
                .actual_var()
        }

        /// Gets the parent's property that has this widget as an input.
        ///
        /// Is an empty string if the widget is not inserted by any property.
        pub fn parent_property_name(&self) -> impl Var<Txt> {
            let mut cache = self.cache.lock();
            cache
                .parent_property_name
                .get_or_insert_with(|| {
                    self.info
                        .map(|w| {
                            Txt::from_static(
                                w.parent_property()
                                    .map(|(p, _)| w.parent().unwrap().inspect_property(p).unwrap().property().name)
                                    .unwrap_or(""),
                            )
                        })
                        .actual_var()
                        .boxed()
                })
                .clone()
        }

        /// Inspect the widget children.
        pub fn children(&self) -> impl Var<Vec<InspectedWidget>> {
            let mut cache = self.cache.lock();
            let cache = &mut *cache;
            cache
                .children
                .get_or_insert_with(|| {
                    let tree = cache.tree.clone();
                    self.info
                        .map(move |w| {
                            if let Some(tree) = tree.upgrade() {
                                assert_eq!(&tree.tree.get(), w.tree());

                                w.children().map(|w| tree.inspect(w.id()).unwrap()).collect()
                            } else {
                                vec![]
                            }
                        })
                        .actual_var()
                        .boxed()
                })
                .clone()
        }

        /// Inspect the builder, properties and intrinsic nodes that make up the widget.
        ///
        /// Is `None` when the widget is built without inspector info collection.
        pub fn inspector_info(&self) -> impl Var<Option<InspectedInfo>> {
            self.info.map(move |w| w.inspector_info().map(InspectedInfo)).actual_var().boxed()
        }

        /// Create a variable that probes info after every frame is rendered.
        pub fn render_watcher<T: VarValue>(&self, mut probe: impl FnMut(&WidgetInfo) -> T + Send + 'static) -> impl Var<T> {
            merge_var!(
                self.info.clone(),
                self.cache.lock().tree.upgrade().unwrap().last_frame(),
                move |w, _| probe(w)
            )
        }
    }

    /// [`InspectorInfo`] that can be placed in a variable.
    #[derive(Clone)]
    pub struct InspectedInfo(pub Arc<InspectorInfo>);
    impl fmt::Debug for InspectedInfo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.0, f)
        }
    }
    impl PartialEq for InspectedInfo {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }
    impl ops::Deref for InspectedInfo {
        type Target = InspectorInfo;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
