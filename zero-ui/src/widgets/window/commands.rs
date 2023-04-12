//! Commands that control the scoped window.

use crate::core::{context::*, event::*, gesture::*};

pub use crate::core::window::commands::*;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the current window.",
        shortcut: [shortcut!(CTRL|SHIFT+I), shortcut!(F12)],
    };
}

#[cfg(inspector)]
pub(super) fn inspect_node(
    child: impl crate::core::widget_instance::UiNode,
    can_inspect: impl crate::core::var::IntoVar<bool>,
) -> impl crate::core::widget_instance::UiNode {
    use crate::core::{
        handler::{async_clmv, hn},
        inspector::prompt::WriteTreeState,
        text::Txt,
        var::var,
        window::{WindowId, WINDOWS},
    };

    let mut inspector_state = WriteTreeState::new();
    let inspector = WindowId::new_unique();
    let inspector_text = var(Txt::empty());

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
                let txt = inspector_state.ansi_string_update(&WINDOW.widget_tree());
                inspector_text.set_ne(txt);
                let inspected = WINDOW.id();

                WINDOWS.focus_or_open(
                    inspector,
                    async_clmv!(inspector_text, { inspector_window::new(inspected, inspector_text) }),
                );
            }
        }),
    )
}

#[cfg(inspector)]
mod inspector_window {
    use crate::core::{inspector::*, window::*};
    use crate::prelude::new_widget::*;

    pub fn new(inspected: WindowId, inspector_text: ArcVar<Txt>) -> WindowCfg {
        use crate::widgets::*;

        let parent = WINDOWS.vars(inspected).unwrap().parent().get().unwrap_or(inspected);

        let tree = WINDOWS.widget_tree(inspected).unwrap();
        let title = if let Some(title) = tree.root().inspect_property(property_id!(window::title)) {
            title.downcast_var::<Text>(0).map(|t| formatx!("{t} - Inspector")).boxed()
        } else {
            var_from("Inspector").boxed()
        };
        let icon = if let Some(icon) = tree.root().inspect_property(property_id!(window::icon)) {
            icon.downcast_var::<WindowIcon>(0).clone().boxed()
        } else {
            var(WindowIcon::Default).boxed()
        };

        window! {
            parent;
            title;
            icon;
            set_inspected = inspected;
            color_scheme = ColorScheme::Dark;
            child = scroll! {
                child = ansi_text! { txt = inspector_text; };
                child_align = Align::TOP_LEFT;
                padding = 5;
            }
        }
    }

    #[property(CONTEXT)]
    fn set_inspected(child: impl UiNode, inspected: impl IntoValue<WindowId>) -> impl UiNode {
        #[ui_node(struct InspectedNode {
            child: impl UiNode,
            inspected: WindowId,
        })]
        impl UiNode for InspectedNode {
            fn info(&self, info: &mut WidgetInfoBuilder) {
                assert!(WIDGET.parent_id().is_none());
                info.meta().set(&INSPECTED_ID, self.inspected);
                self.child.info(info);
            }
        }
        InspectedNode {
            child,
            inspected: inspected.into(),
        }
    }

    /// Gets the window that is inspected by the current inspector window.
    pub fn inspected() -> Option<WindowId> {
        WINDOW.widget_tree().root().meta().get(&INSPECTED_ID).copied()
    }

    pub(super) static INSPECTED_ID: StaticStateId<WindowId> = StaticStateId::new_unique();
}
