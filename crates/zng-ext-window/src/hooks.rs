use zng_app::{
    update::UPDATES,
    view_process::VIEW_PROCESS_INITED_EVENT,
    window::WindowId,
};
use zng_wgt::prelude::clmv;

use crate::{WINDOWS_SV, WindowInstanceState, WindowVars, cmd::WindowCommands};

pub(crate) fn init_window_hooks(id: WindowId, vars: &WindowVars) {
    WindowCommands::init(id, vars);
}

pub(crate) fn init_service_hooks() {
    VIEW_PROCESS_INITED_EVENT.hook(move |_| {
        // window/surface opening happens on layout
        let mut s = WINDOWS_SV.write();

        for (id, w) in s.windows.iter_mut() {
            let vars = match w.vars.as_ref() {
                Some(v) => v,
                // new request
                None => continue,
            };

            if let WindowInstanceState::Loaded { has_view } = vars.0.instance_state.get() {
                // view opens on layout
                UPDATES.layout_window(*id);

                // cleanup old view handles
                if has_view {
                    let r = w.root.as_mut().unwrap();
                    r.renderer = None;
                    r.view_headless = None;
                    r.view_window = None;

                    vars.0.instance_state.set(WindowInstanceState::Loaded { has_view: false });
                }
            }
        }

        true
    }).perm();
}
