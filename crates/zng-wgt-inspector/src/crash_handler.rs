#![cfg(all(
    feature = "crash_handler",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]

//! Debug crash handler.

use std::path::PathBuf;
use zng_app::crash_handler::*;
use zng_ext_config::CONFIG;
use zng_ext_l10n::l10n;
use zng_ext_window::{StartPosition, WINDOWS, WindowRoot};
use zng_wgt::prelude::*;
use zng_wgt::{align, corner_radius, enabled, margin};
use zng_wgt_ansi_text::AnsiText;
use zng_wgt_button::Button;
use zng_wgt_container::{Container, padding};
use zng_wgt_dialog::{DIALOG, FileDialogFilters, FileDialogResponse};
use zng_wgt_fill::background_color;
use zng_wgt_scroll::Scroll;
use zng_wgt_stack::Stack;
use zng_wgt_stack::StackDirection;
use zng_wgt_style::Style;
use zng_wgt_style::style_fn;
use zng_wgt_text::Text;
use zng_wgt_text_input::selectable::SelectableText;
use zng_wgt_toggle::{self as toggle, Toggle};
use zng_wgt_tooltip::{Tip, tooltip};
use zng_wgt_window::Window;
use zng_wgt_wrap::Wrap;

// l10n-## Debug Crash Handler

/// Debug dialog window.
///
/// Used by `zng::app::init_debug`.
pub fn debug_dialog(args: CrashArgs) -> WindowRoot {
    let error = args.latest();
    Window! {
        title = l10n!("crash-handler/window.title", "{$app} - App Crashed", app=zng_env::about().app.clone());
        start_position = StartPosition::CenterMonitor;
        color_scheme = ColorScheme::Dark;

        on_load = hn_once!(|_| {
            // force to foreground
            let _ = WINDOWS.focus(WINDOW.id());
        });
        on_close = hn_once!(args, |_| {
            args.exit(0);
        });

        padding = 5;
        child_top = header(error), 5;
        child = panels(error);
        child_bottom = commands(args), 5;
    }
}

fn header(error: &CrashError) -> impl UiNode {
    SelectableText! {
        txt = error.message();
        margin = 10;
    }
}

fn panels(error: &CrashError) -> impl UiNode {
    let mut options = vec![ErrorPanel::Summary];
    let mut active = ErrorPanel::Summary;

    if !error.stdout.is_empty() {
        options.push(ErrorPanel::StdoutPlain);
        active = ErrorPanel::StdoutPlain;
        if !error.is_stdout_plain() {
            options.push(ErrorPanel::Stdout);
            active = ErrorPanel::Stdout;
        }
    }

    if !error.stderr.is_empty() {
        options.push(ErrorPanel::StderrPlain);
        active = ErrorPanel::StderrPlain;
        if error.is_stderr_plain() {
            options.push(ErrorPanel::Stderr);
            active = ErrorPanel::Stderr;
        }
    }

    if error.has_panic() {
        options.push(ErrorPanel::Panic);
        active = ErrorPanel::Panic;
    }
    if error.has_panic_widget() {
        options.push(ErrorPanel::Widget);
    }
    if error.minidump.is_some() {
        options.push(ErrorPanel::Minidump);
        active = ErrorPanel::Minidump;
    }

    let active = var(active);

    Container! {
        child_top = Wrap! {
            toggle::selector = toggle::Selector::single(active.clone());
            children = options.iter().map(|p| Toggle! {
                child = Text!(p.title());
                value = *p;
            }).collect::<UiVec>();
            toggle::style_fn = Style! {
                padding = (2, 4);
                corner_radius = 2;
            };
            spacing = 5;
        }, 5;
        child = presenter(active, wgt_fn!(error, |p: ErrorPanel| p.panel(&error)));
    }
}

// l10n-## Panels

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorPanel {
    Summary,
    Stdout,
    Stderr,
    StdoutPlain,
    StderrPlain,
    Panic,
    Widget,
    Minidump,
}
impl ErrorPanel {
    fn title(&self) -> Txt {
        match self {
            ErrorPanel::Summary => l10n!("crash-handler/summary.title", "Summary").get(),
            ErrorPanel::Stdout => l10n!("crash-handler/stdout.title", "Stdout").get(),
            ErrorPanel::Stderr => l10n!("crash-handler/stderr.title", "Stderr").get(),
            ErrorPanel::StdoutPlain => l10n!("crash-handler/stdout.title-plain", "Stdout (plain)").get(),
            ErrorPanel::StderrPlain => l10n!("crash-handler/stderr.title-plain", "Stderr (plain)").get(),
            ErrorPanel::Panic => l10n!("crash-handler/panic.title", "Panic").get(),
            ErrorPanel::Widget => l10n!("crash-handler/widget.title", "Widget").get(),
            ErrorPanel::Minidump => l10n!("crash-handler/minidump.title", "Minidump").get(),
        }
    }

    fn panel(&self, error: &CrashError) -> BoxedUiNode {
        match self {
            ErrorPanel::Summary => summary_panel(error).boxed(),
            ErrorPanel::Stdout => std_panel(error.stdout.clone(), "stdout").boxed(),
            ErrorPanel::Stderr => std_panel(error.stderr.clone(), "stderr").boxed(),
            ErrorPanel::StdoutPlain => std_plain_panel(error.stdout_plain(), "stdout").boxed(),
            ErrorPanel::StderrPlain => std_plain_panel(error.stderr_plain(), "stderr").boxed(),
            ErrorPanel::Panic => panic_panel(error.find_panic().unwrap()).boxed(),
            ErrorPanel::Widget => widget_panel(error.find_panic().unwrap().widget_path).boxed(),
            ErrorPanel::Minidump => minidump_panel(error.minidump.clone().unwrap()).boxed(),
        }
    }
}

fn summary_panel(error: &CrashError) -> impl UiNode {
    let s = l10n!(
        "crash-handler/summary.text",
        "Timestamp: {$timestamp}
Exit Code: {$exit_code}
Signal: {$signal}
Stderr: {$stderr_len} bytes
Stdout: {$stdout_len} bytes
Panic: {$is_panic}
Minidump: {$minidump_path}

Args: {$args}
OS: {$os}
",
        timestamp = error.unix_time(),
        exit_code = match error.code {
            Some(c) => format!("{c:#x}"),
            None => String::new(),
        },
        signal = match error.signal {
            Some(c) => format!("{c}"),
            None => String::new(),
        },
        stderr_len = error.stderr.len(),
        stdout_len = error.stdout.len(),
        is_panic = error.find_panic().is_some(),
        minidump_path = match &error.minidump {
            Some(p) => {
                let path = p.display().to_string();
                let path = path.trim_start_matches(r"\\?\");
                path.to_owned()
            }
            None => "none".to_owned(),
        },
        args = format!("{:?}", error.args),
        os = error.os.clone(),
    );
    plain_panel(s.get(), "summary")
}

fn std_panel(std: Txt, config_key: &'static str) -> impl UiNode {
    Scroll! {
        child_align = Align::TOP_START;
        background_color = colors::BLACK;
        padding = 5;
        horizontal_offset = CONFIG.get(formatx!("{config_key}.scroll.h"), 0.fct());
        vertical_offset = CONFIG.get(formatx!("{config_key}.scroll.v"), 0.fct());
        child = AnsiText! {
            txt = std;
            font_size = 0.9.em();
        }
    }
}
fn std_plain_panel(std: Txt, config_key: &'static str) -> impl UiNode {
    plain_panel(std, config_key)
}
fn panic_panel(panic: CrashPanic) -> impl UiNode {
    plain_panel(panic.to_txt(), "panic")
}
fn widget_panel(widget_path: Txt) -> impl UiNode {
    plain_panel(widget_path, "widget")
}
fn minidump_panel(path: PathBuf) -> impl UiNode {
    let path_str = path.display().to_string();
    #[cfg(windows)]
    let path_str = path_str.trim_start_matches(r"\\?\").replace('/', "\\");
    let path_txt = path_str.to_txt();
    Scroll! {
        child_align = Align::TOP_START;
        background_color = colors::BLACK;
        padding = 5;
        horizontal_offset = CONFIG.get(formatx!("minidump.scroll.h"), 0.fct());
        vertical_offset = CONFIG.get(formatx!("minidump.scroll.v"), 0.fct());
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 5;
            children = ui_vec![
                SelectableText! {
                    txt = path_txt;
                    font_size = 0.9.em();
                    // same as AnsiText
                    font_family = ["JetBrains Mono", "Consolas", "monospace"];
                },
                Stack! {
                    direction = StackDirection::top_to_bottom();
                    zng_wgt_button::style_fn = style_fn!(|_| zng_wgt_button::LinkStyle!());
                    children = ui_vec![
                        {
                            let enabled = var(true);
                            Button! {
                                child = Text!("Open Minidump");
                                on_click = async_hn!(enabled, path, |_| {
                                    open_path(enabled, path).await;
                                });
                                enabled;
                            }
                        },
                        {
                            let enabled = var(true);
                            Button! {
                                child = Text!("Open Minidump Dir");
                                on_click = async_hn!(enabled, path, |_| {
                                    open_path(enabled, path.parent().unwrap().to_owned()).await;
                                });
                            }
                        },
                        {
                            let enabled = var(true);
                            Button! {
                                child = Text!("Save Minidump");
                                tooltip = Tip!(Text!("Save copy of the minidump"));
                                on_click = async_hn!(enabled, path, |_| {
                                    save_copy(enabled, path).await;
                                });
                            }
                        },
                        {
                            let enabled = var(true);
                            Button! {
                                child = Text!("Delete Minidump");
                                on_click = async_hn!(enabled, path, |_| {
                                    remove_path(enabled, path).await;
                                });
                            }
                        },
                    ]
                }
            ]
        }
    }
}
async fn open_path(enabled: ArcVar<bool>, path: PathBuf) {
    enabled.set(false);

    #[cfg(windows)]
    let path = path.display().to_string().replace('/', "\\");

    if let Err(e) = task::wait(move || open::that_detached(path)).await {
        DIALOG
            .error(
                "",
                l10n!(
                    "crash-handler/minidump.open-error",
                    "Failed to open minidump.\n{$error}",
                    error = e.to_string()
                ),
            )
            .wait_done()
            .await;
    }

    enabled.set(true);
}
async fn save_copy(enabled: ArcVar<bool>, path: PathBuf) {
    enabled.set(false);

    let mut filters = FileDialogFilters::new();
    if let Some(ext) = path.extension() {
        // l10n-# name for the minidump file type in the save file dialog
        filters.push_filter(
            l10n!("crash-handler/minidump.save-copy-filter-name", "Minidump").get().as_str(),
            &[ext.to_string_lossy()],
        );
    }

    let r = DIALOG
        .save_file(
            l10n!("crash-handler/minidump.save-copy-title", "Save Copy"),
            path.parent().unwrap().to_owned(),
            // l10n-# default file name
            l10n!("crash-handler/minidump.save-copy-starting-name", "minidump"),
            filters,
        )
        .wait_into_rsp()
        .await;

    match r {
        FileDialogResponse::Selected(mut paths) => {
            let destiny = paths.remove(0);
            if let Err(e) = task::wait(move || std::fs::copy(path, destiny)).await {
                DIALOG
                    .error(
                        "",
                        l10n!(
                            "crash-handler/minidump.save-error",
                            "Failed so save minidump copy.\n{$error}",
                            error = format!("[copy] {e}"),
                        ),
                    )
                    .wait_done()
                    .await;
            }
        }
        FileDialogResponse::Cancel => {}
        FileDialogResponse::Error(e) => {
            DIALOG
                .error(
                    "",
                    l10n!(
                        "crash-handler/minidump.save-error",
                        "Failed so save minidump copy.\n{$error}",
                        error = format!("[dialog] {e}"),
                    ),
                )
                .wait_done()
                .await
        }
    }

    enabled.set(true);
}
async fn remove_path(enabled: ArcVar<bool>, path: PathBuf) {
    enabled.set(false);

    if let Err(e) = task::wait(move || std::fs::remove_file(path)).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            DIALOG
                .error(
                    "",
                    l10n!(
                        "crash-handler/minidump.remove-error",
                        "Failed to remove minidump.\n{$error}",
                        error = e.to_string()
                    ),
                )
                .wait_into_rsp()
                .await
        }
    }

    enabled.set(true);
}

fn plain_panel(txt: Txt, config_key: &'static str) -> impl UiNode {
    Scroll! {
        child_align = Align::TOP_START;
        background_color = colors::BLACK;
        padding = 5;
        horizontal_offset = CONFIG.get(formatx!("{config_key}.scroll.h"), 0.fct());
        vertical_offset = CONFIG.get(formatx!("{config_key}.scroll.v"), 0.fct());
        child = SelectableText! {
            txt;
            font_size = 0.9.em();
            // same as AnsiText
            font_family = ["JetBrains Mono", "Consolas", "monospace"];
        }
    }
}

fn commands(args: CrashArgs) -> impl UiNode {
    Stack! {
        spacing = 5;
        direction = StackDirection::start_to_end();
        align = Align::END;
        children = ui_vec![
            Button! {
                child = Text!("Restart App");
                on_click = hn_once!(args, |_| {
                    args.restart();
                });
            },
            Button! {
                child = Text!("Exit App");
                on_click = hn_once!(|_| {
                    args.exit(0);
                });
            }
        ];
    }
}
