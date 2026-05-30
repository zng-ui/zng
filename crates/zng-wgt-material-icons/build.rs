use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap, HashSet},
    env,
    error::Error,
    fmt, fs,
    path::PathBuf,
};

static FONTS: &[(&str, &[u8], &str)] = &[
    #[cfg(feature = "outlined")]
    {
        (
            "outlined",
            include_bytes!("fonts/MaterialIconsOutlined-Regular.ttf"),
            include_str!("fonts/MaterialIconsOutlined-Regular.codepoints"),
        )
    },
    #[cfg(feature = "filled")]
    {
        (
            "filled",
            include_bytes!("fonts/MaterialIcons-Regular.ttf"),
            include_str!("fonts/MaterialIcons-Regular.codepoints"),
        )
    },
    #[cfg(feature = "rounded")]
    {
        (
            "rounded",
            include_bytes!("fonts/MaterialIconsRound-Regular.ttf"),
            include_str!("fonts/MaterialIconsRound-Regular.codepoints"),
        )
    },
    #[cfg(feature = "sharp")]
    {
        (
            "sharp",
            include_bytes!("fonts/MaterialIconsSharp-Regular.ttf"),
            include_str!("fonts/MaterialIconsSharp-Regular.codepoints"),
        )
    },
];

fn main() {
    let mut allow = read_subset_allow();
    let no_filter = allow.is_empty();

    for (mod_name, font, codepoints) in FONTS {
        let allow = if no_filter {
            None
        } else {
            Some(allow.remove(*mod_name).unwrap_or_default())
        };

        write(codepoints, mod_name, font, allow);
    }

    write_html_in_header();
}

fn write(codepoints: &str, mod_name: &str, font: &'static [u8], allow: Option<HashSet<String>>) {
    let Generated { docs, map, subset } = generate(codepoints, mod_name, allow).unwrap();

    let mut font_bytes = Cow::Borrowed(font);

    #[cfg(feature = "embedded_subset")]
    if let Some(set) = subset {
        font_bytes = Cow::Owned(self::subset(font, set));
    }
    #[cfg(not(feature = "embedded_subset"))]
    let _ = (subset, &mut font_bytes);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let generated = out_dir.join(format!("generated.{mod_name}.map.rs"));
    fs::write(generated, map.as_bytes()).unwrap();

    let generated = out_dir.join(format!("generated.{mod_name}.docs.txt"));
    fs::write(generated, docs.as_bytes()).unwrap();

    let generated = out_dir.join(format!("generated.{mod_name}.ttf"));
    fs::write(generated, font_bytes).unwrap();
}
struct Generated {
    docs: String,
    map: String,
    subset: Option<BTreeSet<char>>,
}
fn generate(codepoints: &str, mod_name: &str, allow: Option<HashSet<String>>) -> Result<Generated, Box<dyn Error>> {
    use fmt::Write;

    let mut docs = String::new();
    let mut map = phf_codegen::Map::new();
    let mut subset = BTreeSet::new();

    let mut buffer = String::with_capacity(3);
    for line in codepoints.lines() {
        if let Some((name, code)) = line.split_once(' ') {
            if name.is_empty() || code.is_empty() {
                return Err("invalid codepoints file".into());
            }
            if let Some(allow) = &allow
                && !allow.contains(name)
            {
                continue;
            }

            let code = u32::from_str_radix(code, 16)?;
            let code = char::from_u32(code).ok_or("invalid codepoint")?;
            buffer.push('\'');
            buffer.push(code);
            buffer.push('\'');

            writeln!(&mut docs, r#"| {name} | <span class="material-icons {mod_name}">{code}</span> |"#)?;

            map.entry(name, buffer.clone());
            buffer.clear();

            if allow.is_some() {
                subset.insert(code);
            }
        } else {
            return Err("invalid codepoints file".into());
        }
    }
    let map = format!(
        "/// Map of name to icon codepoint.\npub static MAP: phf::Map<&'static str, char> = {};",
        map.build()
    );

    let subset = if allow.is_some() { Some(subset) } else { None };

    Ok(Generated { docs, map, subset })
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

    for (mod_name, font, _) in FONTS {
        let mut file = doc_dir.join(mod_name);
        file.set_extension("ttf");
        fs::write(file, font).unwrap();

        writeln!(&mut css, "@font-face {{").unwrap();
        writeln!(&mut css, "   font-family: \"zng-material-icons-extensions-{mod_name}\";").unwrap();
        writeln!(
            &mut css,
            "   src: url('/doc/zng-material-icons-extensions/{mod_name}.ttf') format(\"truetype\");"
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
    let dir = target_dir().join("doc");
    fs::create_dir_all(&dir).unwrap();

    dir
}
fn target_dir() -> PathBuf {
    let out_dir = dunce::canonicalize(PathBuf::from(std::env::var("OUT_DIR").unwrap())).unwrap();
    let mut dir = out_dir.parent().unwrap();
    while dir.file_name().unwrap() != "target" {
        dir = dir.parent().expect("failed to get 'target' dir from `OUT_DIR`");
    }
    dir.to_path_buf()
}

#[cfg(not(feature = "embedded_subset"))]
fn read_subset_allow() -> HashMap<String, HashSet<String>> {
    Default::default()
}

#[cfg(feature = "embedded_subset")]
fn read_subset_allow() -> HashMap<String, HashSet<String>> {
    // find profile file
    let path = match std::env::var("ZNG_MATERIAL_ICONS_PROFILE_FILE") {
        Ok(p) => p,
        Err(_) => "res/optimization-profiles/zng-wgt-material-icons.rec.subset".to_owned(),
    };
    println!("cargo::rerun-if-env-changed=ZNG_MATERIAL_ICONS_PROFILE_FILE");
    let mut path = PathBuf::from(path);
    if path.is_relative() {
        // relative to workspace root
        let workspace_root = std::process::Command::new("cargo")
            .args(["locate-project", "--workspace", "--message-format", "plain"])
            .current_dir(target_dir())
            .output()
            .unwrap()
            .stdout;
        let workspace_root = String::from_utf8(workspace_root).unwrap();
        let workspace_root = PathBuf::from(workspace_root);
        path = workspace_root.parent().unwrap().join(path);
    }

    let name = path.file_name().unwrap().to_str().unwrap();
    // maybe has a pair, {name}.rec.subset or {name}.subset
    let pair_name = if let Some(n) = name.strip_suffix(".rec.subset") {
        format!("{n}.subset")
    } else if let Some(n) = name.strip_suffix(".subset") {
        format!("{n}.rec.subset")
    } else {
        panic!("expected .subset or .rec.subset file")
    };
    let pair_path = path.parent().unwrap().join(pair_name);

    assert!(
        path.exists() || pair_path.exists() || cfg!(feature = "_all_features"),
        "subset profile file not found"
    );

    // read
    let mut allow = HashMap::<String, HashSet<String>>::new();
    for profile in [path, pair_path] {
        use std::io::BufRead as _;

        println!("cargo::rerun-if-changed={}", profile.display());

        let profile = match fs::File::open(profile) {
            Ok(f) => f,
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => continue,
                e => panic!("{e}"),
            },
        };
        for line in std::io::BufReader::new(profile).lines() {
            let line = line.unwrap();
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let (set, name) = line.split_once('/').expect("invalid .subset line");

            match allow.get_mut(set) {
                Some(a) => {
                    if !a.contains(name) {
                        a.insert(name.to_owned());
                    }
                }
                None => {
                    allow.entry(set.to_owned()).or_default().insert(name.to_owned());
                }
            }
        }
    }

    if allow.is_empty() && !cfg!(feature = "_all_features") {
        println!("cargo::warning=no icon included in subset");
    }

    allow
}

#[cfg(feature = "embedded_subset")]
fn subset(font: &'static [u8], subset: BTreeSet<char>) -> Vec<u8> {
    use font_subset::*;

    let reader = FontReader::new(font).unwrap();
    let font = reader.read().unwrap();

    let permissions = font.permissions();
    assert!(permissions.embedding.is_lenient());
    assert!(permissions.allow_subsetting);

    if subset.is_empty() {
        return vec![];
    }

    let subset = font.subset(&subset).unwrap();

    subset.to_opentype()
}
