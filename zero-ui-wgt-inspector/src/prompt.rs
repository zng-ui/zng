pub fn inspect_node(
    child: impl zero_ui_app::widget::instance::UiNode,
    can_inspect: impl crate::core::var::IntoVar<bool>,
) -> impl zero_ui_app::widget::instance::UiNode {
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