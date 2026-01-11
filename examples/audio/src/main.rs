//! Audio loading and playback.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use zng::{audio::AUDIOS, prelude::*};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        // allow any audio source
        AUDIOS.limits().set(zng::audio::AudioLimits::none());

        // open unnamed output stream, don't cache because widgets will hold it
        let output = AUDIOS.open_output(false).wait_rsp().await;
        output.stop();

        Window! {
            title = "Audio Example";

            child = Button! {
                child = Text!("Open Audio");
                layout::align = Align::CENTER;
                widget::enabled = output.state().map(|s| s.is_stopped());
                on_click = async_hn!(output, |_| {
                    if let Some(path) = open_dialog().await {
                        let audio = AUDIOS.read(path);
                        audio.wait_match(|a| a.is_loaded()).await;
                        output.cue(audio.get());
                    }
                });
            };

            // playback controls
            child_bottom = Stack! {
                layout::align = Align::CENTER;
                direction = StackDirection::left_to_right();
                spacing = 5;
                layout::margin = 5;
                zng::icon::ico_size = 20;
                children = ui_vec![
                    Button! {
                        child = ICONS.req("material/filled/play-arrow");
                        widget::visibility = output.state().map(|s| (!s.is_playing()).into());
                        tooltip = Tip!(Text!("Play"));
                        on_click = hn!(output, |_| {
                            output.play();
                        });
                    },
                    Button! {
                        child = ICONS.req("material/filled/pause");
                        tooltip = Tip!(Text!("Pause"));
                        widget::visibility = output.state().map(|s| s.is_playing().into());
                        on_click = hn!(output, |_| {
                            output.pause();
                        });
                    },
                    Button! {
                        child = ICONS.req("material/filled/stop");
                        tooltip = Tip!(Text!("Stop"));
                        widget::enabled = output.state().map(|s| !s.is_stopped());
                        on_click = hn!(|_| {
                            output.stop();
                        });
                    }
                ];
            };
        }
    });
}

async fn open_dialog() -> Option<PathBuf> {
    DIALOG
        .open_file("Open Audio", std::env::current_dir().unwrap_or_default(), "", {
            let mut f = dialog::FileDialogFilters::default();
            f.push_filter(
                "Audio Files",
                AUDIOS.available_formats().iter().flat_map(|i| i.file_extensions_iter()),
            );
            f.push_filter("All Files", ["*"]);
            f
        })
        .wait_rsp()
        .await
        .into_path()
        .unwrap_or_else(|e| {
            tracing::error!("open file dialog error, {e}");
            None
        })
}
