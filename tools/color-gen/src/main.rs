//! Generate the colors code.

use serde::Deserialize;

fn main() {
    println!("{}", colors::generate().unwrap());
}

pub mod xterm_256 {
    use super::*;

    pub fn generate() -> Result<String, Box<dyn std::error::Error>> {
        use std::fmt::Write;

        let colors: Vec<XColor> = serde_json::from_str(JSON)?;

        let mut s = String::new();

        writeln!(&mut s, "static X_TERM_256: [(u8, u8, u8); 256] = [")?;

        for color in colors {
            let Rgb { r, g, b } = color.rgb;
            writeln!(&mut s, "   ({r}, {g}, {b}),")?;
        }

        writeln!(&mut s, "];")?;

        Ok(s)
    }

    // credits to https://www.ditig.com/256-colors-cheat-sheet
    const JSON: &str = include_str! {"xterm-256.json"};

    #[allow(unused)]
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct XColor {
        color_id: u32,
        hex_string: String,
        rgb: Rgb,
        hsl: Hsl,
        name: String,
    }

    #[allow(unused)]
    #[derive(Deserialize)]
    struct Rgb {
        r: u8,
        g: u8,
        b: u8,
    }

    #[allow(unused)]
    #[derive(Deserialize)]
    struct Hsl {
        h: f32,
        s: f32,
        l: f32,
    }
}

pub mod web_colors {
    use super::*;

    pub fn generate() -> Result<String, Box<dyn std::error::Error>> {
        let colors: Vec<WebColor> = serde_json::from_str(JSON)?;

        let mut s = String::new();

        for color in colors {
            use std::fmt::Write;

            writeln!(&mut s)?;
            writeln!(
                &mut s,
                r#"/// <span style="display: inline-block; background-color:#{hex}; width:20px; height:20px;"></span> {name}, `#{hex}`, `rgb({r}, {g}, {b})`."#,
                hex = color.hex,
                name = color.doc_name(),
                r = color.rgb.r,
                g = color.rgb.g,
                b = color.rgb.b,
            )?;
            writeln!(
                &mut s,
                "pub const {}: Rgba = rgb!({}, {}, {});",
                color.const_name(),
                color.rgb.r,
                color.rgb.g,
                color.rgb.b
            )?;
        }

        Ok(s)
    }

    // credits to https://gist.github.com/raineorshine/10394189
    const JSON: &str = include_str! {"webcolors.json"};

    #[derive(Deserialize)]
    struct WebColor {
        name: String,
        hex: String,
        rgb: Color,
    }
    impl WebColor {
        fn doc_name(&self) -> String {
            let mut result = String::with_capacity(self.name.len() + 1);
            for c in self.name.chars() {
                if c.is_uppercase() && !result.is_empty() {
                    result.push(' ');
                }
                result.push(c);
            }
            result
        }

        fn const_name(&self) -> String {
            let mut result = String::with_capacity(self.name.len() + 1);
            for c in self.name.chars() {
                if result.is_empty() {
                    result.push(c)
                } else if c.is_uppercase() {
                    result.push('_');
                    result.push(c);
                } else {
                    result.push(c.to_ascii_uppercase())
                }
            }
            result
        }
    }

    #[derive(Deserialize)]
    struct Color {
        r: u8,
        g: u8,
        b: u8,
    }
}

pub mod colors {
    use super::*;

    pub fn generate() -> Result<String, Box<dyn std::error::Error>> {
        let colors: Vec<WebColor> = serde_json::from_str(JSON)?;

        let mut s = String::new();

        for color in colors {
            use std::fmt::Write;

            writeln!(&mut s)?;
            writeln!(
                &mut s,
                r#"/// <span style="display: inline-block; background-color:#{hex}; width:20px; height:20px;"></span> {name}, `#{hex}`, `rgb({r}, {g}, {b})`."#,
                hex = color.hex(),
                name = color.doc_name(),
                r = color.rgb.r,
                g = color.rgb.g,
                b = color.rgb.b,
            )?;
            writeln!(
                &mut s,
                "pub const {}: Rgba = rgb!({}, {}, {});",
                color.const_name(),
                color.rgb.r,
                color.rgb.g,
                color.rgb.b
            )?;
        }

        Ok(s)
    }

    const JSON: &str = include_str! {"colors.json"};

    #[derive(Deserialize)]
    struct WebColor {
        name: String,
        rgb: Color,
    }
    impl WebColor {
        fn doc_name(&self) -> String {
            let mut result = String::with_capacity(self.name.len() + 1);
            for c in self.name.chars() {
                if c.is_uppercase() && !result.is_empty() {
                    result.push(' ');
                }
                result.push(c);
            }
            result
        }

        fn const_name(&self) -> String {
            let mut result = String::with_capacity(self.name.len() + 1);
            for c in self.name.chars() {
                if result.is_empty() {
                    result.push(c)
                } else if c.is_uppercase() {
                    result.push('_');
                    result.push(c);
                } else {
                    result.push(c.to_ascii_uppercase())
                }
            }
            result
        }

        fn hex(&self) -> String {
            format!("{:02X}{:02X}{:02X}", self.rgb.r, self.rgb.g, self.rgb.b)
        }
    }

    #[derive(Deserialize)]
    struct Color {
        r: u8,
        g: u8,
        b: u8,
    }
}
