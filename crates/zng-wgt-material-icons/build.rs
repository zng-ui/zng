use std::{env, error::Error, fmt, fs, path::PathBuf};

static FONTS: &[(&str, &str, &str, &[u8], &str)] = &[
    #[cfg(feature = "outlined")]
    {
        (
            "outlined",
            "otf",
            "opentype",
            include_bytes!("fonts/MaterialIconsOutlined-Regular.otf"),
            include_str!("fonts/MaterialIconsOutlined-Regular.codepoints"),
        )
    },
    #[cfg(feature = "filled")]
    {
        (
            "filled",
            "ttf",
            "truetype",
            include_bytes!("fonts/MaterialIcons-Regular.ttf"),
            include_str!("fonts/MaterialIcons-Regular.codepoints"),
        )
    },
    #[cfg(feature = "rounded")]
    {
        (
            "rounded",
            "otf",
            "opentype",
            include_bytes!("fonts/MaterialIconsRound-Regular.otf"),
            include_str!("fonts/MaterialIconsRound-Regular.codepoints"),
        )
    },
    #[cfg(feature = "sharp")]
    {
        (
            "sharp",
            "otf",
            "opentype",
            include_bytes!("fonts/MaterialIconsSharp-Regular.otf"),
            include_str!("fonts/MaterialIconsSharp-Regular.codepoints"),
        )
    },
];

fn main() {
    for (mod_name, _, _, _, codepoints) in FONTS {
        write(codepoints, mod_name);
    }

    write_html_in_header();
}

fn write(codepoints: &str, mod_name: &str) {
    let [docs, map] = generate(codepoints, mod_name).unwrap();

    let generated = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("generated.{mod_name}.map.rs"));
    fs::write(generated, map.as_bytes()).unwrap();

    let generated = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("generated.{mod_name}.docs.txt"));
    fs::write(generated, docs.as_bytes()).unwrap();
}
fn generate(codepoints: &str, mod_name: &str) -> Result<[String; 2], Box<dyn Error>> {
    use fmt::Write;

    let mut docs = String::new();
    let mut map = phf_codegen::Map::new();

    let mut buffer = String::with_capacity(3);
    for line in codepoints.lines() {
        if let Some((name, code)) = line.split_once(' ') {
            if name.is_empty() || code.is_empty() {
                return Err("invalid codepoints file".into());
            }

            let code = u32::from_str_radix(code, 16)?;
            let code = char::from_u32(code).ok_or("invalid codepoint")?;
            buffer.push('\'');
            buffer.push(code);
            buffer.push('\'');

            writeln!(&mut docs, r#"| {name} | <span class="material-icons {mod_name}">{code}</span> |"#)?;

            map.entry(name, buffer.clone());
            buffer.clear();
        } else {
            return Err("invalid codepoints file".into());
        }
    }
    let map = format!(
        "/// Map of name to icon codepoint.\npub static MAP: phf::Map<&'static str, char> = {};",
        map.build()
    );

    Ok([docs, map])
}

fn write_html_in_header() {
    let doc_dir = doc_dir();
    let file = doc_dir.join("zng-material-icons-extensions.css");
    let doc_dir = doc_dir.join("zng-material-icons-extensions");
    if !doc_dir.exists() {
        fs::create_dir(&doc_dir).unwrap();
    }

    use std::fmt::Write;
    let mut css = String::new();

    for (mod_name, ext, format, font, _) in FONTS {
        let mut file = doc_dir.join(mod_name);
        file.set_extension(ext);
        fs::write(file, font).unwrap();

        writeln!(&mut css, "@font-face {{").unwrap();
        writeln!(&mut css, "   font-family: \"zng-material-icons-extensions-{mod_name}\";").unwrap();
        writeln!(
            &mut css,
            "   src: url('/doc/zng-material-icons-extensions/{mod_name}.{ext}') format(\"{format}\");"
        )
        .unwrap();
        writeln!(&mut css, "}}").unwrap();
        writeln!(&mut css, ".material-icons.{mod_name} {{").unwrap();
        writeln!(&mut css, "   font-family: \"zng-material-icons-extensions-{mod_name}\";").unwrap();
        writeln!(&mut css, "   font-size: 32px;").unwrap();
        writeln!(&mut css, "}}").unwrap();
    }
    fs::write(&file, css).unwrap();
    let html = "<link rel=\"stylesheet\" href=\"/doc/zng-material-icons-extensions.css\">";
    let mut file = file;
    file.set_extension("html");
    fs::write(file, html).unwrap();
}
fn doc_dir() -> PathBuf {
    let out_dir = dunce::canonicalize(PathBuf::from(std::env::var("OUT_DIR").unwrap())).unwrap();

    let mut dir = out_dir.parent().unwrap();
    while dir.file_name().unwrap() != "target" {
        dir = dir.parent().expect("failed to get 'target' dir from `OUT_DIR`");
    }
    let dir = dir.join("doc");
    fs::create_dir_all(&dir).unwrap();

    dir
}
