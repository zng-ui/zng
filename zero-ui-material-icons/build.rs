use std::{env, error::Error, fmt, fs, path::PathBuf};

const OUTLINED: &str = include_str!("fonts/MaterialIconsOutlined-Regular.codepoints");
const FILLED: &str = include_str!("fonts/MaterialIcons-Regular.codepoints");
const ROUNDED: &str = include_str!("fonts/MaterialIconsRound-Regular.codepoints");
const SHARP: &str = include_str!("fonts/MaterialIconsSharp-Regular.codepoints");
const TWO_TONE: &str = include_str!("fonts/MaterialIconsTwoTone-Regular.codepoints");

fn main() {
    write(OUTLINED, "outlined");
    write(FILLED, "filled");
    write(ROUNDED, "rounded");
    write(SHARP, "sharp");
    write(TWO_TONE, "two_tone");
}

fn write(codepoints: &str, mod_name: &str) {
    let code = generate(codepoints).unwrap();

    let generated = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("generated.{mod_name}.rs"));
    fs::write(generated, code.as_bytes()).unwrap();
}

fn generate(codepoints: &str) -> Result<String, Box<dyn Error>> {
    use fmt::Write;

    let mut s = String::new();

    let mut all = vec![];
    for line in codepoints.lines() {
        if let Some((name, code)) = line.split_once(' ') {
            if name.is_empty() || code.is_empty() {
                return Err("invalid codepoints file".into());
            }

            let name = if name.chars().next().unwrap().is_digit(10) {
                format!("N{name}").to_uppercase()
            } else {
                name.to_uppercase()
            };

            let code = u32::from_str_radix(code, 16)?;
            let code = char::from_u32(code).ok_or("invalid codepoint")?;

            writeln!(&mut s)?;
            writeln!(&mut s, r#"/// <span class="material-icons sharp">&#{code};</span>"#)?;
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
