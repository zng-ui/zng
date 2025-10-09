use std::{path::PathBuf, sync::Arc};

use zng::{
    app::{NEW_CMD, OPEN_CMD, SAVE_AS_CMD, SAVE_CMD},
    button,
    clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD},
    color::filter::opacity,
    focus::{FocusClickBehavior, alt_focus_scope, focus_click_behavior},
    gesture::click_shortcut,
    icon::{self, ICONS},
    layout::{Dip, align, margin, padding},
    prelude::*,
    rule_line,
    scroll::ScrollMode,
    undo::UNDO_CMD,
    widget::{Visibility, corner_radius, enabled, visibility},
    window::WindowRoot,
};

pub fn text_editor() -> UiNode {
    let is_open = var(false);

    Button! {
        child = Text!(
            is_open.map(|&i| if i { "show text editor" } else { "open text editor" }.into())
        );
        style_fn = button::LinkStyle!();
        on_click = hn!(|_| {
            let editor_id = WindowId::named("text-editor");
            if is_open.get() {
                if WINDOWS.focus(editor_id).is_err() {
                    is_open.set(false);
                }
            } else {
                WINDOWS.open_id(editor_id, async_clmv!(is_open, { text_editor_window(is_open) }));
            }
        });
    }
}

fn text_editor_window(is_open: Var<bool>) -> WindowRoot {
    let editor = TextEditor::init();
    Window! {
        title = editor.title();
        on_open = hn!(is_open, |_| {
            is_open.set(true);
        });
        on_close = hn!(is_open, |_| {
            is_open.set(false);
        });
        on_close_requested = async_hn!(editor, |args| {
            editor.on_close_requested(args).await;
        });
        min_width = 450;

        child_top = text_editor_menu(editor.clone()), 0;

        child = Scroll! {
            mode = ScrollMode::VERTICAL;
            child_align = Align::FILL;
            scroll_to_focused_mode = None;
            enabled = editor.enabled();

            // line numbers
            child_start =
                Text! {
                    padding = (7, 4);
                    txt_align = Align::TOP_RIGHT;
                    opacity = 80.pct();
                    layout::min_width = 24;
                    txt = editor.lines.map(|s| {
                        use std::fmt::Write;
                        let mut txt = String::new();
                        match s {
                            text::LinesWrapCount::NoWrap(len) => {
                                for i in 1..=(*len).max(1) {
                                    let _ = writeln!(&mut txt, "{i}");
                                }
                            }
                            text::LinesWrapCount::Wrap(counts) => {
                                for (i, &c) in counts.iter().enumerate() {
                                    let _ = write!(&mut txt, "{}", i + 1);
                                    for _ in 0..c {
                                        txt.push('\n');
                                    }
                                }
                            }
                        }
                        Txt::from_str(&txt)
                    });
                },
                0,
            ;

            // editor
            child = TextInput! {
                id = editor.input_wgt_id();
                txt = editor.txt.clone();
                accepts_tab = true;
                accepts_enter = true;
                get_caret_status = editor.caret_status.clone();
                get_lines_wrap_count = editor.lines.clone();
                widget::border = unset!;
            };
        };

        child_bottom =
            Text! {
                margin = (0, 4);
                align = Align::RIGHT;
                txt = editor.caret_status.map_to_txt();
            },
            0,
        ;
    }
}

fn text_editor_menu(editor: Arc<TextEditor>) -> UiNode {
    let menu_width = var(Dip::MAX);
    let gt_700 = menu_width.map(|&w| Visibility::from(w > Dip::new(700)));
    let gt_600 = menu_width.map(|&w| Visibility::from(w > Dip::new(600)));
    let gt_500 = menu_width.map(|&w| Visibility::from(w > Dip::new(500)));

    let clipboard_btn = clmv!(gt_600, |cmd: zng::event::Command| {
        let cmd = cmd.focus_scoped();
        Button! {
            child = cmd.flat_map(|c| c.icon()).present_data(());
            child_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_600.clone()), 4;
            tooltip = Tip!(Text!(cmd.flat_map(|c| c.name_with_shortcut())));
            visibility = true;
            cmd;
        }
    });

    let undo_combo = clmv!(gt_700, |op: zng::undo::UndoOp| {
        let cmd = op.cmd().undo_scoped();

        Toggle! {
            style_fn = toggle::ComboStyle!();

            widget::enabled = cmd.flat_map(|c| c.is_enabled());

            child = Button! {
                child = cmd.flat_map(|c| c.icon()).present_data(());
                child_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_700.clone()), 4;
                tooltip = Tip!(Text!(cmd.flat_map(|c| c.name_with_shortcut())));
                on_click = hn!(|a| {
                    a.propagation().stop();
                    cmd.get().notify();
                });
            };

            checked_popup = wgt_fn!(|_| popup::Popup! {
                child = zng::undo::history::UndoHistory!(op);
            });
        }
    });

    Stack! {
        id = "menu";
        align = Align::FILL_TOP;
        alt_focus_scope = true;
        focus_click_behavior = FocusClickBehavior::Exit;
        spacing = 4;
        direction = StackDirection::left_to_right();
        padding = 4;
        enabled = editor.enabled();
        layout::actual_width = menu_width;
        button::style_fn = Style! {
            padding = (2, 4);
            corner_radius = 2;
            icon::ico_size = 16;
        };
        rule_line::vr::margin = 0;
        children = ui_vec![
            Button! {
                child = ICONS.req("material/sharp/insert-drive-file");
                child_right = Text!(txt = NEW_CMD.name(); visibility = gt_500.clone()), 4;
                tooltip = Tip!(Text!(NEW_CMD.name_with_shortcut()));

                click_shortcut = NEW_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.create().await;
                });
            },
            Button! {
                child = ICONS.req("material/sharp/folder-open");
                child_right = Text!(txt = OPEN_CMD.name(); visibility = gt_500.clone()), 4;
                tooltip = Tip!(Text!(OPEN_CMD.name_with_shortcut()));

                click_shortcut = OPEN_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.open().await;
                });
            },
            Button! {
                child = ICONS.req("material/sharp/save");
                child_right = Text!(txt = SAVE_CMD.name(); visibility = gt_500.clone()), 4;
                tooltip = Tip!(Text!(SAVE_CMD.name_with_shortcut()));

                enabled = editor.unsaved();
                click_shortcut = SAVE_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.save().await;
                });
            },
            Button! {
                child = Text!(SAVE_AS_CMD.name());
                when #{gt_500}.is_collapsed() {
                    child = ICONS.req("material/sharp/save-as");
                }

                tooltip = Tip!(Text!(SAVE_AS_CMD.name_with_shortcut()));

                click_shortcut = SAVE_AS_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.save_as().await;
                });
            },
            Vr!(),
            clipboard_btn(CUT_CMD),
            clipboard_btn(COPY_CMD),
            clipboard_btn(PASTE_CMD),
            Vr!(),
            undo_combo(zng::undo::UndoOp::Undo),
            undo_combo(zng::undo::UndoOp::Redo),
        ];
    }
}
struct TextEditor {
    input_wgt_id: WidgetId,
    file: Var<Option<std::path::PathBuf>>,
    txt: Var<Txt>,

    txt_touched: Var<bool>,

    caret_status: Var<text::CaretStatus>,
    lines: Var<text::LinesWrapCount>,
    busy: Var<u32>,
}
impl TextEditor {
    pub fn init() -> Arc<Self> {
        let txt = var(Txt::from_static(""));
        let unsaved = var(false);
        txt.bind_map(&unsaved, |_| true).perm();
        Arc::new(Self {
            input_wgt_id: WidgetId::new_unique(),
            file: var(None),
            txt,
            txt_touched: unsaved,
            caret_status: var(text::CaretStatus::none()),
            lines: var(text::LinesWrapCount::NoWrap(0)),
            busy: var(0),
        })
    }

    pub fn input_wgt_id(&self) -> WidgetId {
        self.input_wgt_id
    }

    pub fn title(&self) -> Var<Txt> {
        merge_var!(self.unsaved(), self.file.clone(), |u, f| {
            let mut t = "Text Example - Editor".to_owned();
            if *u {
                t.push('*');
            }
            if let Some(f) = f {
                use std::fmt::Write;
                let _ = write!(&mut t, " - {}", f.display());
            }
            Txt::from_str(&t)
        })
    }

    pub fn unsaved(&self) -> Var<bool> {
        let can_undo = UNDO_CMD.scoped(self.input_wgt_id).is_enabled();
        merge_var!(self.txt_touched.clone(), can_undo, |&t, &u| t && u)
    }

    pub fn enabled(&self) -> Var<bool> {
        self.busy.map(|&b| b == 0)
    }

    pub async fn create(&self) {
        let _busy = self.enter_busy();

        if self.handle_unsaved().await {
            self.txt.set(Txt::from_static(""));
            self.file.set(None);
            self.txt_touched.set(false);
        }
    }

    pub async fn open(&self) {
        let _busy = self.enter_busy();

        if !self.handle_unsaved().await {
            return;
        }

        let (init_dir, init_name, filters) = self.file_dialog_data();
        let r = DIALOG
            .open_file("Open Text", init_dir, init_name, filters)
            .wait_rsp()
            .await
            .into_path();
        match r {
            Ok(Some(file)) => {
                let r = task::wait(clmv!(file, || std::fs::read_to_string(file))).await;
                match r {
                    Ok(t) => {
                        self.txt.set(Txt::from_str(&t));
                        self.txt_touched.set(false);
                        self.file.set(file);
                    }
                    Err(e) => {
                        self.handle_error("reading file", e.to_txt()).await;
                    }
                }
            }
            Err(e) => {
                self.handle_error("opening file", e).await;
            }
            _ => {}
        }
    }

    pub async fn save(&self) -> bool {
        if let Some(file) = self.file.get() {
            let _busy = self.enter_busy();
            let ok = self.write(file).await;
            self.txt_touched.set(!ok);
            ok
        } else {
            self.save_as().await
        }
    }

    pub async fn save_as(&self) -> bool {
        let _busy = self.enter_busy();

        let (init_dir, init_name, filters) = self.file_dialog_data();
        let r = DIALOG
            .save_file("Save Text", init_dir, init_name, filters)
            .wait_rsp()
            .await
            .into_path();
        match r {
            Ok(Some(file)) => {
                let ok = self.write(file.clone()).await;
                self.txt_touched.set(!ok);
                if ok {
                    self.file.set(Some(file));
                }
                return ok;
            }
            Err(e) => {
                self.handle_error("saving file", e.to_txt()).await;
            }
            _ => {}
        }

        false // cancel
    }

    pub async fn on_close_requested(&self, args: WindowCloseRequestedArgs) {
        if self.unsaved().get() {
            args.propagation().stop();
            if self.handle_unsaved().await {
                self.txt_touched.set(false);
                WINDOW.close();
            }
        }
    }

    fn file_dialog_data(&self) -> (PathBuf, Txt, dialog::FileDialogFilters) {
        let mut dlg_dir = std::env::current_dir().unwrap_or_default();
        let mut dlg_name = Txt::from("text.md");
        if let Some(p) = self.file.get() {
            if let Some(n) = p.file_name() {
                dlg_name = n.to_string_lossy().to_txt();
            }
            if let Some(p) = p.parent() {
                p.clone_into(&mut dlg_dir);
            }
        }

        let mut f = dialog::FileDialogFilters::default();
        f.push_filter("Text Files", &["txt", "md"]);
        f.push_filter("Text File", &["txt"]);
        f.push_filter("Markdown File", &["md"]);
        f.push_filter("All Files", &["*"]);

        (dlg_dir, dlg_name, f)
    }

    async fn write(&self, file: std::path::PathBuf) -> bool {
        let txt = self.txt.clone();
        let r = task::wait(move || txt.with(move |txt| std::fs::write(file, txt.as_bytes()))).await;
        match r {
            Ok(()) => true,
            Err(e) => {
                self.handle_error("writing file", e.to_txt()).await;
                false
            }
        }
    }

    async fn handle_unsaved(&self) -> bool {
        if !self.unsaved().get() {
            return true;
        }

        let r = DIALOG
            .custom(dialog::Dialog! {
                style_fn = dialog::WarnStyle!();
                title = Text!("Save File?");
                content = SelectableText!("Save file? All unsaved changes will be lost.");
                responses = vec![
                    dialog::Response::cancel(),
                    dialog::Response::new("Discard", "Discard"),
                    dialog::Response::new("Save", "Save"),
                ];
            })
            .wait_rsp()
            .await;
        match r.name.as_str() {
            "Discard" => true,
            "Save" => self.save().await,
            _ => false,
        }
    }

    async fn handle_error(&self, context: &'static str, e: Txt) {
        tracing::error!("error {context}, {e}");
        DIALOG.error("Error", formatx!("Error {context}.\n\n{e}")).wait_rsp().await;
    }

    fn enter_busy(&self) -> impl Drop {
        struct BusyTracker(Var<u32>);
        impl Drop for BusyTracker {
            fn drop(&mut self) {
                self.0.modify(|b| **b -= 1);
            }
        }
        self.busy.modify(|b| **b += 1);
        BusyTracker(self.busy.clone())
    }
}
