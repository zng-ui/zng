use std::fmt;

use zng::{
    button,
    label::{self, Label},
    layout::{align, padding},
    prelude::*,
    text::font_weight,
    text_input,
    window::WindowRoot,
};

pub fn form_editor() -> impl UiNode {
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
                WINDOWS.open_id(editor_id, async_clmv!(is_open, { form_editor_window(is_open) }));
            }
        });
    }
}

fn form_editor_window(is_open: Var<bool>) -> WindowRoot {
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
            id = "form";

            columns = ui_vec![grid::Column!(), grid::Column!(1.lft())];
            spacing = (5, 10);
            padding = 20;

            label::style_fn = Style! {
                text::txt_align = Align::END;
            };
            text_input::style_fn = style_fn!(|_| text_input::FieldStyle!());

            cells = ui_vec![
                Label! {
                    txt = "Name";
                    target = "field-name";
                },
                TextInput! {
                    grid::cell::column = 1;
                    id = "field-name";
                    txt = var_from("my-crate");
                    max_chars_count = 50;
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
                    txt = var_from("John Doe");
                },
                Label! {
                    grid::cell::row = 2;
                    txt = "Version";
                    target = "field-version";
                },
                TextInput! {
                    id = "field-version";
                    grid::cell::row = 2;
                    grid::cell::column = 1;
                    txt_parse = var(Version::default());
                    text_input::field_help = "help text";
                    // txt_parse_on_stop = true;
                },
                Label! {
                    grid::cell::row = 3;
                    txt = "Password";
                    target = "field-password";
                },
                TextInput! {
                    grid::cell::row = 3;
                    grid::cell::column = 1;
                    id = "field-password";
                    txt = var_from("pass");
                    obscure_txt = true;
                },
            ];
        };

        child_bottom = {
            node: Stack! {
                direction = StackDirection::start_to_end();
                padding = 10;
                align = Align::END;
                spacing = 5;
                children = ui_vec![
                    Button! {
                        child = Text!("Cancel");
                        on_click = hn!(|_| {
                            WINDOW.close();
                        });
                    },
                    Button! {
                        font_weight = FontWeight::BOLD;
                        child = Text!("Validate");
                        on_click = hn!(|_| {
                            zng::text::cmd::PARSE_CMD.notify_descendants(&WINDOW.info().get("form").unwrap());
                        });
                    }
                ]
            },
            spacing: 10,
        };
    }
}

/// Basic version type for input validation demo.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    rev: u32,
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.rev)
    }
}
impl std::str::FromStr for Version {
    type Err = Txt;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut r = Self::default();

        let mut split = s.split('.');
        if let Some(major) = split.next() {
            if !major.is_empty() {
                r.major = u32::from_str(major).map_err(|e| e.to_txt())?;
            }
        }
        if let Some(minor) = split.next() {
            if !minor.is_empty() {
                r.minor = u32::from_str(minor).map_err(|e| e.to_txt())?;
            }
        }
        if let Some(rev) = split.next() {
            if !rev.is_empty() {
                r.rev = u32::from_str(rev).map_err(|e| e.to_txt())?;
            }
        }
        if split.next().is_some() {
            return Err("expected maximum of 3 version numbers".into());
        }

        Ok(r)
    }
}
