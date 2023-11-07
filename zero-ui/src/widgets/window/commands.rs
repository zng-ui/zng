//! Commands that control the scoped window.

use crate::core::{event::*, gesture::*};

pub use crate::core::window::commands::*;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the current window.",
        shortcut: [shortcut!(CTRL|SHIFT+'I'), shortcut!(F12)],
    };
}

#[cfg(inspector)]
pub(super) fn inspect_node(
    child: impl crate::core::widget_instance::UiNode,
    can_inspect: impl crate::core::var::IntoVar<bool>,
) -> impl crate::core::widget_instance::UiNode {
    live_inspector::inspect_node(child, can_inspect)
    // prompt_inspector::inspect_node(child, can_inspect)
}

#[allow(unused)]
#[cfg(inspector)]
mod live_inspector {
    use crate::core::{
        context::*,
        handler::async_clmv,
        inspector::live::InspectedTree,
        var::IntoVar,
        widget_instance::UiNode,
        window::{WindowId, WINDOWS},
    };
    use zero_ui_core::hn;

    use super::*;

    pub fn inspect_node(child: impl UiNode, can_inspect: impl IntoVar<bool>) -> impl UiNode {
        let mut inspected_tree = None::<InspectedTree>;
        let inspector = WindowId::new_unique();

        let can_inspect = can_inspect.into_var();

        on_command(
            child,
            || INSPECT_CMD.scoped(WINDOW.id()),
            move || can_inspect.clone(),
            hn!(|args: &CommandArgs| {
                if !args.enabled {
                    return;
                }
                args.propagation().stop();

                if let Some(inspected) = inspector_window::inspected() {
                    // can't inspect inspector window, redirect command to inspected
                    INSPECT_CMD.scoped(inspected).notify();
                } else {
                    let inspected_tree = match &inspected_tree {
                        Some(i) => {
                            i.update(WINDOW.info());
                            i.clone()
                        }
                        None => {
                            let i = InspectedTree::new(WINDOW.info());
                            inspected_tree = Some(i.clone());
                            i
                        }
                    };
                    let inspected = WINDOW.id();

                    WINDOWS.focus_or_open(
                        inspector,
                        async_clmv!(inspected_tree, { inspector_window::new(inspected, inspected_tree) }),
                    );
                }
            }),
        )
    }

    pub mod inspector_window {
        use crate::core::inspector::live::*;
        use crate::core::{inspector::*, window::*};
        use crate::prelude::new_widget::*;
        use crate::prelude::*;

        pub fn new(inspected: WindowId, inspected_tree: InspectedTree) -> WindowRoot {
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

            let selected_wgt = var(None);

            Window! {
                parent;
                title;
                icon;
                set_inspected = inspected;
                color_scheme = ColorScheme::Dark;
                child = Scroll! {
                    child = tree_view(inspected_tree, selected_wgt);
                    child_align = Align::FILL_TOP;
                    padding = 5;
                }
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

        fn tree_view(tree: InspectedTree, _selected_wgt: ArcVar<Option<InspectedWidget>>) -> impl UiNode {
            Container! {
                text::font_family = ["JetBrains Mono", "Consolas", "monospace"];

                child = wgt_view(tree.inspect_root());
                data = tree;
            }
        }

        fn wgt_view(wgt: InspectedWidget) -> impl UiNode {
            let parent_property = wgt.parent_property_name();
            Container! {
                child = Wrap! {
                    padding = 2;
                    when *#is_hovered {
                        background_color = color_scheme_map(rgba(0.3, 0.3, 0.3, 0.2), rgba(0.7, 0.7, 0.7, 0.2));
                    }

                    children = ui_vec![
                        Text! {
                            txt = parent_property.clone();
                            font_color = colors::YELLOW;
                        },
                        Text! {
                            txt = parent_property.map(|p| Txt::from_static(if p.is_empty() { "" } else { " = " }));
                        },
                        Text! {
                            txt = wgt.wgt_type_name().map(|n| formatx!("{n}!"));
                            font_weight = FontWeight::BOLD;
                            font_color = colors::AZURE;
                        },
                        Text!(wgt.descendants_len().map(|&l| if l == 0 { Txt::from_static(" {}") } else { Txt::from_static(" {") })),
                    ]
                };

                child_insert_below = presenter(wgt.children(), wgt_fn!(|children: Vec<InspectedWidget>| {
                    let children: UiNodeVec = children.into_iter().map(wgt_view).collect();
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
                                    sides: color_scheme_map(
                                        BorderSides::new_left(BorderSide::dashed(rgba(0.3, 0.3, 0.3, 0.2))),
                                        BorderSides::new_left(BorderSide::dashed(rgba(0.7, 0.7, 0.7, 0.2))),
                                    ),
                                };
                                when *#is_hovered {
                                    border = {
                                        widths: (0, 0, 0, 1),
                                        sides: color_scheme_map(
                                            BorderSides::new_left(BorderSide::dashed(rgba(0.3, 0.3, 0.3, 1.0))),
                                            BorderSides::new_left(BorderSide::dashed(rgba(0.7, 0.7, 0.7, 1.0))),
                                        ),
                                    };
                                }
                            };
                            child_insert_below = Text!("}}"), 0;
                        }.boxed()
                        
                    }
                    
                })), 2;                
            }
        }
    }
}

#[allow(unused)]
#[cfg(inspector)]
mod prompt_inspector {
    use super::*;

    pub fn inspect_node(
        child: impl crate::core::widget_instance::UiNode,
        can_inspect: impl crate::core::var::IntoVar<bool>,
    ) -> impl crate::core::widget_instance::UiNode {
        use crate::core::{
            context::*,
            handler::{async_clmv, hn},
            inspector::prompt::WriteTreeState,
            text::Txt,
            var::var,
            window::{WindowId, WINDOWS},
        };

        let mut inspector_state = WriteTreeState::new();
        let inspector = WindowId::new_unique();
        let inspector_text = var(Txt::from_str(""));

        let can_inspect = can_inspect.into_var();

        on_command(
            child,
            || INSPECT_CMD.scoped(WINDOW.id()),
            move || can_inspect.clone(),
            hn!(|args: &CommandArgs| {
                if !args.enabled {
                    return;
                }
                args.propagation().stop();

                if let Some(inspected) = inspector_window::inspected() {
                    // can't inspect inspector window, redirect command to inspected
                    INSPECT_CMD.scoped(inspected).notify();
                } else {
                    let txt = inspector_state.ansi_string_update(&WINDOW.info());
                    inspector_text.set(txt);
                    let inspected = WINDOW.id();

                    WINDOWS.focus_or_open(
                        inspector,
                        async_clmv!(inspector_text, { inspector_window::new(inspected, inspector_text) }),
                    );
                }
            }),
        )
    }

    pub mod inspector_window {
        use crate::core::{inspector::*, window::*};
        use crate::prelude::new_widget::*;

        pub fn new(inspected: WindowId, inspector_text: ArcVar<Txt>) -> WindowRoot {
            use crate::widgets::*;

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

            Window! {
                parent;
                title;
                icon;
                set_inspected = inspected;
                color_scheme = ColorScheme::Dark;
                child = Scroll! {
                    child = AnsiText! { txt = inspector_text; };
                    child_align = Align::TOP_LEFT;
                    padding = 5;
                }
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
    }
}
