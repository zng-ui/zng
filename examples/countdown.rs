#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use enclose::enclose;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|ctx| {
        let count = var(20u32);
        let mut every1s = Some(ctx.sync.update_every_secs(1));
        window! {
            title: "Countdown Example";
            on_update: enclose!{ (count) move |ctx| {
                println!("on_update");

                // if timer still running.
                if let Some(l) = &every1s {
                    match l.updates(ctx.events).len() {
                        1 => {
                            // timer updated once since last update
                            let new_count = count.get(ctx.vars).saturating_sub(1);
                            count.set(ctx.vars, new_count);
                            println!("count set to {}", new_count);

                            if new_count == 0 {
                                every1s = None;// stop timer
                                println!("timer stopped");
                            }
                        }
                        0 => {
                            // timer did not update, ok
                        }
                        _ => {
                            panic!("timer updated twice in single update call")
                        }
                    }
                }
            }};
            content: example(count);
        }
    })
}

fn example(count: RcVar<u32>) -> impl Widget {
    text! {
        text: count.into_map(|&n| {
            if n > 0 {
                formatx!("{}", n)
            } else {
                "Done!".into()
            }
        });
        font_size: 32.pt();
    }
}
