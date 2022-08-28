use std::{env, error::Error, fmt, fs, path::PathBuf};

static FONTS: &[(&str, &[u8], &str)] = &[
    (
        "outlined",
        include_bytes!("fonts/MaterialIconsOutlined-Regular.otf"),
        include_str!("fonts/MaterialIconsOutlined-Regular.codepoints"),
    ),
    (
        "filled",
        include_bytes!("fonts/MaterialIcons-Regular.ttf"),
        include_str!("fonts/MaterialIcons-Regular.codepoints"),
    ),
    (
        "rounded",
        include_bytes!("fonts/MaterialIconsRound-Regular.otf"),
        include_str!("fonts/MaterialIconsRound-Regular.codepoints"),
    ),
    (
        "sharp",
        include_bytes!("fonts/MaterialIconsSharp-Regular.otf"),
        include_str!("fonts/MaterialIconsSharp-Regular.codepoints"),
    ),
    (
        "two_tone",
        include_bytes!("fonts/MaterialIconsTwoTone-Regular.otf"),
        include_str!("fonts/MaterialIconsTwoTone-Regular.codepoints"),
    ),
];

fn main() {
    for (mod_name, _, codepoints) in FONTS {
        write(codepoints, mod_name);
    }

    write_html_in_header();
}

fn write(codepoints: &str, mod_name: &str) {
    let code = generate(codepoints, mod_name).unwrap();

    let generated = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("generated.{mod_name}.rs"));
    fs::write(generated, code.as_bytes()).unwrap();
}
fn generate(codepoints: &str, mod_name: &str) -> Result<String, Box<dyn Error>> {
    use fmt::Write;

    let mut s = String::new();

    let mut all = vec![];
    for line in codepoints.lines() {
        if let Some((name, code)) = line.split_once(' ') {
            if name.is_empty() || code.is_empty() {
                return Err("invalid codepoints file".into());
            }

            let name = if name.chars().next().unwrap().is_ascii_digit() {
                format!("N{name}").to_uppercase()
            } else {
                name.to_uppercase()
            };

            let code = u32::from_str_radix(code, 16)?;
            let code = char::from_u32(code).ok_or("invalid codepoint")?;

            writeln!(&mut s)?;
            writeln!(&mut s, r#"/// <span class="material-icons {mod_name}">{code}</span>"#)?;
            writeln!(&mut s, r#"/// "#)?;
            writeln!(&mut s, r#"/// <span class="material-icons large {mod_name}">{code}</span>"#)?;
            writeln!(
                &mut s,
                r#"pub const {name}: MaterialIcon = MaterialIcon {{ font: meta::FONT_NAME, name: "{name}", code: '{code}', }};"#
            )?;

            all.push(name);
        } else {
            return Err("invalid codepoints file".into());
        }
    }

    writeln!(&mut s)?;
    writeln!(&mut s, "/// All icons.")?;
    writeln!(&mut s, "pub fn all() -> Vec<MaterialIcon> {{")?;
    write!(&mut s, "    vec![")?;
    for name in all {
        write!(&mut s, "{name}")?;
        write!(&mut s, ", ")?;
    }
    writeln!(&mut s, "]")?;
    writeln!(&mut s, "}}")?;

    Ok(s)
}

fn write_html_in_header() {
    let doc_dir = doc_dir();
    let file = doc_dir.join("zero-ui-material-icons-extensions.html");
    let doc_dir = doc_dir.join("zero-ui-material-icons-extensions");
    if !doc_dir.exists() {
        fs::create_dir(&doc_dir).unwrap();
    }

    use std::fmt::Write;
    let mut css = String::new();
    writeln!(&mut css, ".material-icons.large {{").unwrap();
    writeln!(&mut css, "   font-size: 32px;").unwrap();
    writeln!(&mut css, "}}").unwrap();

    for (mod_name, font, _) in FONTS {
        let file = doc_dir.join(mod_name);
        fs::write(file, font).unwrap();


        writeln!(&mut css, ".material-icons.{mod_name} {{").unwrap();
        writeln!(&mut css, "   font-family: url('zero-ui-material-icons-extensions/{mod_name}');").unwrap();
        writeln!(&mut css, "}}").unwrap();
    }
    fs::write(&file, "<style>\n{css}\n</style>").unwrap();
}
fn doc_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap()).canonicalize().unwrap();

    let mut dir = out_dir.parent().unwrap();
    while dir.file_name().unwrap() != "target" {
        dir = dir.parent().expect("failed to get 'target' dir from `OUT_DIR`");
    }
    let dir = dir.join("doc");
    fs::create_dir_all(&dir).unwrap();

    dir
}
