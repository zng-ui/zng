//! Generate the zero_ui::core::color::colors module.

use serde::Deserialize;

fn main() {
    let colors: Vec<WebColor> = serde_json::from_str(JSON).unwrap();
    for color in colors {
        println! {}
        println! {"/// {} (`#{}`)", color.doc_name(), color.hex}
        println! {"///"}
        println! {"/// `rgb({}, {}, {})`", color.rgb.r, color.rgb.g, color.rgb.b}
        println! {"pub const {}: Color = rgb!({}, {}, {});", color.const_name(), color.rgb.r, color.rgb.g, color.rgb.b}
    }
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
