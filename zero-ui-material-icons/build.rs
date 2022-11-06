use std::{env, error::Error, fmt, fs, path::PathBuf};

static FONTS: &[(&str, &str, &str, &[u8], &str)] = &[
    (
        "outlined",
        "otf",
        "opentype",
        include_bytes!("fonts/MaterialIconsOutlined-Regular.otf"),
        include_str!("fonts/MaterialIconsOutlined-Regular.codepoints"),
    ),
    (
        "filled",
        "ttf",
        "truetype",
        include_bytes!("fonts/MaterialIcons-Regular.ttf"),
        include_str!("fonts/MaterialIcons-Regular.codepoints"),
    ),
    (
        "rounded",
        "otf",
        "opentype",
        include_bytes!("fonts/MaterialIconsRound-Regular.otf"),
        include_str!("fonts/MaterialIconsRound-Regular.codepoints"),
    ),
    (
        "sharp",
        "otf",
        "opentype",
        include_bytes!("fonts/MaterialIconsSharp-Regular.otf"),
        include_str!("fonts/MaterialIconsSharp-Regular.codepoints"),
    ),
    (
        "two_tone",
        "otf",
        "opentype",
        include_bytes!("fonts/MaterialIconsTwoTone-Regular.otf"),
        include_str!("fonts/MaterialIconsTwoTone-Regular.codepoints"),
    ),
];

fn main() {
    for (mod_name, _, _, _, codepoints) in FONTS {
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
    let file = doc_dir.join("zero-ui-material-icons-extensions.css");
    let doc_dir = doc_dir.join("zero-ui-material-icons-extensions");
    if !doc_dir.exists() {
        fs::create_dir(&doc_dir).unwrap();
    }

    use std::fmt::Write;
    let mut css = String::new();
    writeln!(&mut css, ".material-icons.large {{").unwrap();
    writeln!(&mut css, "   font-size: 32px;").unwrap();
    writeln!(&mut css, "}}").unwrap();

    for (mod_name, ext, format, font, _) in FONTS {
        let mut file = doc_dir.join(mod_name);
        file.set_extension(ext);
        fs::write(file, font).unwrap();

        writeln!(&mut css, "@font-face {{").unwrap();
        writeln!(&mut css, "   font-family: \"zero-ui-material-icons-extensions-{mod_name}\";").unwrap();
        writeln!(
            &mut css,
            "   src: url('/zero-ui-material-icons-extensions/{mod_name}.{ext}') format(\"{format}\");"
        )
        .unwrap();
        writeln!(&mut css, "}}").unwrap();
        writeln!(&mut css, ".material-icons.{mod_name} {{").unwrap();
        writeln!(&mut css, "   font-family: \"zero-ui-material-icons-extensions-{mod_name}\";").unwrap();
        writeln!(&mut css, "}}").unwrap();
    }
    fs::write(&file, css).unwrap();
    let html = "<link rel=\"stylesheet\" href=\"/zero-ui-material-icons-extensions.css\">";
    let mut file = file;
    file.set_extension("html");
    fs::write(file, html).unwrap();
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
