#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::Arc;

use zero_ui::core::text::{UnderlinePosition, UnderlineSkip, FONTS};
use zero_ui::core::{
    clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD},
    undo::{CommandUndoExt, REDO_CMD, UNDO_CMD},
};
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("text");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().extend(zero_ui_material_icons::MaterialFonts).run_window(async {
        let fs = var(Length::Pt(11.0));

        Window! {
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            child = z_stack(ui_vec![
                Stack! {
                    font_size = fs.easing(150.ms(), easing::linear);
                    direction = StackDirection::left_to_right();
                    align = Align::CENTER;
                    spacing = 40;
                    children = ui_vec![
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                basic(),
                                defaults(),
                            ];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                line_height(),
                                line_spacing(),
                                word_spacing(),
                                letter_spacing(),
                            ];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                decoration_lines(),
                            ]
                        }
                    ];
                },
                Container! {
                    align = Align::TOP;
                    margin = 10;
                    child = font_size(fs);
                },
                Stack! {
                    direction = StackDirection::top_to_bottom();
                    spacing = 5;
                    margin = 20;
                    align = Align::BOTTOM_RIGHT;
                    children_align = Align::RIGHT;
                    children = ui_vec![
                        text_editor(),
                        form_editor(),
                    ];
                },
            ])
        }
    })
}

fn font_size(font_size: ArcVar<Length>) -> impl UiNode {
    fn change_size(font_size: &ArcVar<Length>, change: f32) {
        font_size.modify(move |s| {
            *s.to_mut() += Length::Pt(change);
        });
    }
    Stack! {
        button::extend_style = Style! { padding = (0, 5) };
        direction = StackDirection::left_to_right();
        spacing = 5;
        corner_radius = 4;
        background_color = color_scheme_map(rgba(0, 0, 0, 40.pct()), rgba(1., 1., 1., 40.pct()));
        padding = 4;
        children = ui_vec![
            Button! {
                child = Text!("-");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!('-')];
                on_click = hn!(font_size, |_| {
                    change_size(&font_size, -1.0)
                });
            },
            Text! {
                txt = font_size.map(|s| formatx!("{s}"));
            },
            Button! {
                child = Text!("+");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!('+')];
                on_click = hn!(font_size, |_| {
                    change_size(&font_size, 1.0)
                });
            },
        ]
    }
}

fn basic() -> impl UiNode {
    section(
        "basic",
        ui_vec![
            Text!("Basic Text"),
            Strong!("Strong Text"),
            Em!("Emphasis Text"),
            Text! {
                font_color = color_scheme_map(web_colors::LIGHT_GREEN, web_colors::DARK_GREEN);
                txt = "Colored Text";

                when *#is_hovered {
                    font_color = color_scheme_map(web_colors::YELLOW, web_colors::BROWN);
                }
            },
            Text!("Emoticons 🔎👨‍💻🧐"),
        ],
    )
}

fn line_height() -> impl UiNode {
    section(
        "line_height",
        ui_vec![
            Text! {
                txt = "Default: 'Émp Giga Ç'";
                background_color = web_colors::LIGHT_BLUE;
                font_color = web_colors::BLACK;
            },
            Text! {
                txt = "150%: 'Émp Giga Ç'";
                background_color = web_colors::LIGHT_BLUE;
                font_color = web_colors::BLACK;
                line_height = 150.pct();
            },
        ],
    )
}

fn line_spacing() -> impl UiNode {
    section(
        "line_spacing",
        ui_vec![Container! {
            child = Text! {
                txt = "Hello line 1!\nHello line 2!\nHover to change `line_spacing`";
                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                txt_wrap = false;

                when *#is_hovered {
                    #[easing(150.ms())]
                    line_spacing = 30.pct();
                }
            };
            child_align = Align::TOP;
            min_height = 1.7.em() * 3.fct();
        }],
    )
}

fn word_spacing() -> impl UiNode {
    section(
        "word_spacing",
        ui_vec![Text! {
            txt = "Word spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                word_spacing = 100.pct();
            }
        }],
    )
}

fn letter_spacing() -> impl UiNode {
    section(
        "letter_spacing",
        ui_vec![Text! {
            txt = "Letter spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                letter_spacing = 30.pct();
            }
        }],
    )
}

fn decoration_lines() -> impl UiNode {
    section(
        "Decorations",
        ui_vec![
            Text! {
                txt = "Overline, 1, Dotted,\ndefault color";
                overline = 1, LineStyle::Dotted;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 1, Solid,\ndefault color";
                strikethrough = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 4, Double,\ndifferent color";
                strikethrough = 4, LineStyle::Double;
                strikethrough_color = web_colors::RED;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color";
                underline = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, skip spaces";
                underline = 1, LineStyle::Solid;
                underline_skip = UnderlineSkip::SPACES;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, descent";
                underline = 1, LineStyle::Solid;
                underline_position = UnderlinePosition::Descent;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 3, wavy,\ndifferent color, no skip";
                underline = 3, LineStyle::Wavy(1.0);
                underline_color = web_colors::GREEN;
                underline_skip = UnderlineSkip::NONE;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
            }
        ],
    )
}

fn defaults() -> impl UiNode {
    fn demo(title: &str, font_family: impl Into<FontNames>) -> impl UiNode {
        let font_family = font_family.into();

        let font_name = FONTS
            .list(
                &font_family,
                FontStyle::Normal,
                FontWeight::NORMAL,
                FontStretch::NORMAL,
                &lang!(und),
            )
            .map(|f| match f.done() {
                Some(f) => f.best().family_name().to_text(),
                None => Txt::from_str(""),
            });

        Stack! {
            direction = StackDirection::left_to_right();
            children_align = Align::BASELINE_LEFT;
            children = ui_vec![
                Text!(if title.is_empty() {
                    formatx!("{font_family}: ")
                } else {
                    formatx!("{title}: ")
                }),
                Text! {
                    txt = font_name;
                    font_family;
                }
            ];
        }
    }

    section(
        "defaults",
        ui_vec![
            // Generic
            demo("", FontName::serif()),
            demo("", FontName::sans_serif()),
            demo("", FontName::monospace()),
            demo("", FontName::cursive()),
            demo("", FontName::fantasy()),
            demo("Fallback", "not-a-font-get-fallback"),
            demo("UI", FontNames::default())
        ],
    )
}

fn section(header: &'static str, items: impl UiNodeList) -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![Text! {
            txt = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn text_editor() -> impl UiNode {
    let is_open = var(false);

    Button! {
        child = Text!(is_open.map(|&i| if i { "show text editor" } else { "open text editor" }.into()));
        style_fn = button::LinkStyle!();
        on_click = hn!(|_| {
            let editor_id = WindowId::named("text-editor");
            if is_open.get() {
                if WINDOWS.focus(editor_id).is_err() {
                    is_open.set(false);
                }
            } else {
                WINDOWS.open_id(editor_id, async_clmv!(is_open, {
                    text_editor_window(is_open)
                }));
            }
        });
    }
}

fn text_editor_window(is_open: ArcVar<bool>) -> WindowRoot {
    let editor = TextEditor::init();
    Window! {
        title = editor.title();
        on_open = hn!(is_open, |_| {
            is_open.set(true);
        });
        on_close = hn!(is_open, |_| {
            is_open.set(false);
        });
        enabled = editor.enabled();
        on_close_requested = async_hn!(editor, |args: WindowCloseRequestedArgs| {
            editor.on_close_requested(args).await;
        });
        min_width = 450;

        child_insert_above = text_editor_menu(editor.clone()), 0;

        child = Scroll! {
            mode = ScrollMode::VERTICAL;
            child_align = Align::FILL;
            scroll_to_focused_mode = None;

            // line numbers
            child_insert_start = Text! {
                padding = (7, 4);
                txt_align = Align::TOP_RIGHT;
                opacity = 80.pct();
                min_width = 24;
                txt = editor.lines.map(|s| {
                    use std::fmt::Write;
                    let mut txt = String::new();
                    match s {
                        text::LinesWrapCount::NoWrap(len) => {
                            for i in 1..=(*len).max(1) {
                                let _ = writeln!(&mut txt, "{i}");
                            }
                        },
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
            }, 0;

            // editor
            child = TextInput! {
                id = editor.input_wgt_id();
                txt = editor.txt.clone();
                accepts_tab = true;
                accepts_enter = true;
                get_caret_status = editor.caret_status.clone();
                get_lines_wrap_count = editor.lines.clone();
                border = unset!;
            };
        };

        child_insert_below = Text! {
            margin = (0, 4);
            align = Align::RIGHT;
            txt = editor.caret_status.map_to_text();
        }, 0;
    }
}

fn text_editor_menu(editor: Arc<TextEditor>) -> impl UiNode {
    let menu_width = var(units::Dip::MAX);
    let gt_700 = menu_width.map(|&w| Visibility::from(w > units::Dip::new(700)));
    let gt_600 = menu_width.map(|&w| Visibility::from(w > units::Dip::new(600)));
    let gt_500 = menu_width.map(|&w| Visibility::from(w > units::Dip::new(500)));
    Stack! {
        id = "menu";
        align = Align::FILL_TOP;
        alt_focus_scope = true;
        focus_click_behavior = FocusClickBehavior::Exit;
        spacing = 4;
        direction = StackDirection::left_to_right();
        padding = 4;
        actual_width = menu_width;
        button::extend_style = Style! {
            padding = (2, 4);
            corner_radius = 2;
            icon::ico_size = 16;
        };
        rule_line::vr::margin = 0;
        children = ui_vec![
            Button! {
                child = Icon!(zero_ui_material_icons::sharp::INSERT_DRIVE_FILE);
                child_insert_right = Text!(txt = NEW_CMD.name(); visibility = gt_500.clone()), 4;
                tooltip = Tip!(Text!(NEW_CMD.name_with_shortcut()));

                click_shortcut = NEW_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.create().await;
                });
            },
            Button! {
                child = Icon!(zero_ui_material_icons::sharp::FOLDER_OPEN);
                child_insert_right = Text!(txt = OPEN_CMD.name(); visibility = gt_500.clone()), 4;
                tooltip = Tip!(Text!(OPEN_CMD.name_with_shortcut()));

                click_shortcut = OPEN_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.open().await;
                });
            },
            Button! {
                child = Icon!(zero_ui_material_icons::sharp::SAVE);
                child_insert_right = Text!(txt = SAVE_CMD.name(); visibility = gt_500.clone()), 4;
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
                    child = Icon!(zero_ui_material_icons::sharp::SAVE_AS);
                }

                tooltip = Tip!(Text!(SAVE_AS_CMD.name_with_shortcut()));

                click_shortcut = SAVE_AS_CMD.shortcut();
                on_click = async_hn!(editor, |_| {
                    editor.save_as().await;
                });
            },
            Vr!(),
            {
                let cmd = CUT_CMD.focus_scoped();
                Button! {
                    child = Icon!(zero_ui_material_icons::sharp::CUT);
                    child_insert_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_600.clone()), 4;
                    tooltip = Tip!(Text!(cmd.flat_map(|c|c.name_with_shortcut())));
                    enabled = cmd.flat_map(|c| c.is_enabled());

                    on_click = hn!(|_| {
                        cmd.get().notify();
                    });
                }
            },
            {
                let cmd = COPY_CMD.focus_scoped();
                Button! {
                    child = Icon!(zero_ui_material_icons::sharp::COPY);
                    child_insert_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_600.clone()), 4;
                    tooltip = Tip!(Text!(cmd.flat_map(|c|c.name_with_shortcut())));
                    enabled = cmd.flat_map(|c| c.is_enabled());

                    on_click = hn!(|_| {
                        cmd.get().notify();
                    });
                }
            },
            {
                let cmd = PASTE_CMD.focus_scoped();
                Button! {
                    child = Icon!(zero_ui_material_icons::sharp::PASTE);
                    child_insert_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_600), 4;
                    tooltip = Tip!(Text!(cmd.flat_map(|c|c.name_with_shortcut())));
                    enabled = cmd.flat_map(|c| c.is_enabled());

                    on_click = hn!(|_| {
                        cmd.get().notify();
                    });
                }
            },
            Vr!(),
            {
                let cmd = UNDO_CMD.undo_scoped();
                Toggle! {
                    id = "undo-Toggle";
                    style_fn = toggle::ComboStyle!();

                    enabled = cmd.flat_map(|c| c.is_enabled());

                    child = Button! {
                        child = Icon!(zero_ui_material_icons::sharp::UNDO);
                        child_insert_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_700.clone()), 4;
                        tooltip = Tip!(Text!(cmd.flat_map(|c|c.name_with_shortcut())));

                        on_click = hn!(|a: &ClickArgs| {
                            a.propagation().stop();
                            cmd.get().notify();
                        });
                    };

                    checked_popup = wgt_fn!(|_| popup::Popup! {
                        child = undo::UndoHistory!();
                    });
                }
            },
            {
                let cmd = REDO_CMD.undo_scoped();
                Toggle! {
                    style_fn = toggle::ComboStyle!();

                    enabled = cmd.flat_map(|c| c.is_enabled());

                    child = Button! {
                        child = Icon!(zero_ui_material_icons::sharp::REDO);
                        child_insert_right = Text!(txt = cmd.flat_map(|c| c.name()); visibility = gt_700.clone()), 4;
                        tooltip = Tip!(Text!(cmd.flat_map(|c|c.name_with_shortcut())));

                        on_click = hn!(|a: &ClickArgs| {
                            a.propagation().stop();
                            cmd.get().notify();
                        });
                    };

                    checked_popup = wgt_fn!(|_| popup::Popup! {
                        child = undo::UndoHistory!(zero_ui::core::undo::UndoOp::Redo);
                    });
                }
            },
        ]
    }
}
struct TextEditor {
    input_wgt_id: WidgetId,
    file: ArcVar<Option<std::path::PathBuf>>,
    txt: ArcVar<Txt>,

    txt_touched: ArcVar<bool>,

    caret_status: ArcVar<text::CaretStatus>,
    lines: ArcVar<text::LinesWrapCount>,
    busy: ArcVar<u32>,
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

    pub fn title(&self) -> impl Var<Txt> {
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

    pub fn unsaved(&self) -> impl Var<bool> {
        let can_undo = UNDO_CMD.scoped(self.input_wgt_id).is_enabled();
        merge_var!(self.txt_touched.clone(), can_undo, |&t, &u| t && u)
    }

    pub fn enabled(&self) -> impl Var<bool> {
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

        use zero_ui::core::app::view_process::*;

        let mut dlg = FileDialog {
            title: "Open Text".into(),
            kind: FileDialogKind::OpenFile,
            ..Default::default()
        };
        dlg.push_filter("Text Files", &["txt", "md"]).push_filter("All Files", &["*"]);
        let r = WINDOWS.native_file_dialog(WINDOW.id(), dlg).wait_rsp().await;
        match r {
            FileDialogResponse::Selected(s) => {
                let r = task::wait(move || std::fs::read_to_string(&s[0])).await;
                match r {
                    Ok(t) => {
                        self.txt.set(Txt::from_str(&t));
                        self.txt_touched.set(false);
                    }
                    Err(e) => {
                        self.handle_error("reading file", e.to_string()).await;
                    }
                }
            }
            FileDialogResponse::Cancel => {}
            FileDialogResponse::Error(e) => {
                self.handle_error("opening file", e).await;
            }
        }
    }

    pub async fn save(&self) -> bool {
        if let Some(file) = self.file.get() {
            let _busy = self.enter_busy();
            self.write(file).await
        } else {
            self.save_as().await
        }
    }

    pub async fn save_as(&self) -> bool {
        let _busy = self.enter_busy();

        use zero_ui::core::app::view_process::*;

        let mut dlg = FileDialog {
            title: "Save Text".into(),
            kind: FileDialogKind::SaveFile,
            ..Default::default()
        };
        dlg.push_filter("Text", &["txt"])
            .push_filter("Markdown", &["md"])
            .push_filter("All Files", &["*"]);
        let r = WINDOWS.native_file_dialog(WINDOW.id(), dlg).wait_rsp().await;
        match r {
            FileDialogResponse::Selected(mut s) => {
                if let Some(file) = s.pop() {
                    let ok = self.write(file.clone()).await;
                    self.txt_touched.set(!ok);
                    if ok {
                        self.file.set(Some(file));
                    }
                    return ok;
                }
            }
            FileDialogResponse::Cancel => {}
            FileDialogResponse::Error(e) => {
                self.handle_error("saving file", e.to_string()).await;
            }
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

    async fn write(&self, file: std::path::PathBuf) -> bool {
        let txt = self.txt.clone();
        let r = task::wait(move || txt.with(move |txt| std::fs::write(file, txt.as_bytes()))).await;
        match r {
            Ok(()) => true,
            Err(e) => {
                self.handle_error("writing file", e.to_string()).await;
                false
            }
        }
    }

    async fn handle_unsaved(&self) -> bool {
        if !self.unsaved().get() {
            return true;
        }

        use zero_ui::core::app::view_process::*;

        let dlg = MsgDialog {
            title: "Save File?".into(),
            message: "Save file? All unsaved changes will be lost.".into(),
            icon: MsgDialogIcon::Warn,
            buttons: MsgDialogButtons::YesNo,
        };
        let r = WINDOWS.native_message_dialog(WINDOW.id(), dlg).wait_rsp().await;
        match r {
            MsgDialogResponse::Yes => self.save().await,
            MsgDialogResponse::No => true,
            _ => false,
        }
    }

    async fn handle_error(&self, context: &'static str, e: String) {
        tracing::error!("error {context}, {e}");

        use zero_ui::core::app::view_process::*;

        let dlg = MsgDialog {
            title: "Error".into(),
            message: format!("Error {context}.\n\n{e}"),
            icon: MsgDialogIcon::Error,
            buttons: MsgDialogButtons::Ok,
        };
        let _ = WINDOWS.native_message_dialog(WINDOW.id(), dlg).wait_rsp().await;
    }

    fn enter_busy(&self) -> impl Drop {
        struct BusyTracker(ArcVar<u32>);
        impl Drop for BusyTracker {
            fn drop(&mut self) {
                self.0.modify(|b| *b.to_mut() -= 1);
            }
        }
        self.busy.modify(|b| *b.to_mut() += 1);
        BusyTracker(self.busy.clone())
    }
}

fn form_editor() -> impl UiNode {
    let is_open = var(false);

    Button! {
        child = Text!(is_open.map(|&i| if i { "show form editor" } else { "open form editor" }.into()));
        style_fn = button::LinkStyle!();
        on_click = hn!(|_| {
            let editor_id = WindowId::named("form-editor");
            if is_open.get() {
                if WINDOWS.focus(editor_id).is_err() {
                    is_open.set(false);
                }
            } else {
                WINDOWS.open_id(editor_id, async_clmv!(is_open, {
                    form_editor_window(is_open)
                }));
            }
        });
    }
}

fn form_editor_window(is_open: ArcVar<bool>) -> WindowRoot {
    Window! {
        title = "Form";
        on_open = hn!(is_open, |_| {
            is_open.set(true);
        });
        on_close = hn!(is_open, |_| {
            is_open.set(false);
        });

        size = (400, 500);

        child = Grid! {
            columns = ui_vec![grid::Column!(), grid::Column!(1.lft())];
            spacing = (5, 10);
            padding = 20;

            label::extend_style = Style! {
                txt_align = Align::END;
            };
            cells = ui_vec![
                Label! {
                    txt = "Name";
                    target = "field-name";
                },
                TextInput! {
                    grid::cell::column = 1;
                    id = "field-name";
                    txt = var_from("my-crate");
                },

                Label! {
                    grid::cell::row = 1;
                    txt = "Authors";
                    target = "field-authors";
                },
                TextInput! {
                    grid::cell::row = 1;
                    grid::cell::column = 1;
                    id = "field-authors";
                    txt = var_from("Jhon Doe");
                },

                Label! {
                    grid::cell::row = 2;
                    txt = "Version";
                    target = "field-version";
                },
                TextInput! {
                    grid::cell::row = 2;
                    grid::cell::column = 1;
                    id = "field-version";
                    txt = var_from("0.1.0");
                },
            ];
        };

        child_insert_below = {
            insert: Stack! {
                padding = 10;
                align = Align::RIGHT;
                spacing = 5;
                children = ui_vec![
                    Button! {
                        child = Text!("Cancel");
                        on_click = hn!(|_| {
                            WINDOW.close();
                        });
                    }
                ]
            },
            spacing: 10,
        };
    }
}
