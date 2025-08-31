use crate::{println, util};
use std::{borrow::Cow, collections::HashSet, path::PathBuf};

pub fn generate(args: Vec<&str>) {
    for member in &util::publish_members() {
        if !args.is_empty() && !args.contains(&member.name.as_str()) {
            continue;
        }

        let readme = PathBuf::from(format!("crates/{}/README.md", member.name));

        println(&format!("crates/{}/Cargo.toml", member.name));

        let previous = if readme.exists() {
            Cow::from(std::fs::read_to_string(&readme).unwrap())
        } else {
            Cow::from(README_TEMPLATE.to_owned())
        };

        let mut s = String::new();
        let mut lines = previous.lines().peekable();
        while let Some(line) = lines.next() {
            use std::fmt::*;

            writeln!(&mut s, "{line}").unwrap();
            match line {
                "<!--do doc --readme header-->" => {
                    writeln!(&mut s, "{HEADER}").unwrap();
                    while let Some(l) = lines.next() {
                        if l.trim().is_empty() {
                            break;
                        }
                    }
                }
                "<!--do doc --readme features-->" => {
                    if let Some(l) = lines.peek() {
                        if l.trim() == FEATURES_HEADER {
                            while let Some(l) = lines.next() {
                                if l == SECTION_END {
                                    break;
                                }
                            }
                        }
                    }

                    let (features, defaults) = read_features(&format!("crates/{}/Cargo.toml", member.name));
                    if !features.is_empty() {
                        writeln!(&mut s, "{FEATURES_HEADER}").unwrap();

                        if features.len() == 1 {
                            if defaults.contains(&features[0].name) {
                                writeln!(&mut s, "\n This crate provides 1 feature flag, enabled by default.",).unwrap();
                            } else {
                                writeln!(&mut s, "\n This crate provides 1 feature flag, not enabled by default.",).unwrap();
                            }
                        } else {
                            writeln!(
                                &mut s,
                                "\nThis crate provides {} feature flags, {} enabled by default.\n",
                                features.len(),
                                defaults.len(),
                            )
                            .unwrap();
                        }

                        for f in features {
                            if f.docs.is_empty() {
                                crate::error(format_args!("missing docs for `{}` feature", f.name));
                            }
                            writeln!(&mut s, "#### `\"{}\"`\n{}", f.name, f.docs).unwrap();
                            if defaults.contains(&f.name) {
                                writeln!(&mut s, "*Enabled by default.*\n").unwrap();
                            }
                        }

                        writeln!(&mut s, "{SECTION_END}").unwrap();
                    }
                }
                l => {
                    if let Some(run) = l.strip_prefix("<!--do doc --readme do zng") {
                        if let Some(args) = run.strip_suffix("-->") {
                            let args = args.trim();
                            let output = std::process::Command::new("cargo")
                                .args(&["do", "zng"])
                                .args(args.split(' '))
                                .env("NO_COLOR", "")
                                .output()
                                .unwrap_or_else(|e| crate::util::fatal(format_args!("failed to run `{run}`, {e}")));
                            let output = String::from_utf8(output.stdout).unwrap();
                            writeln!(&mut s, "```console\n$ cargo zng {args}\n").unwrap();

                            let mut out_lines = output.trim_end().lines();
                            out_lines.next().unwrap(); // skip do header

                            while let Some(line) = out_lines.next() {
                                let line = line.trim_end().replace("@ target/debug/cargo-zng", "@ cargo-zng");
                                if line == ".zr-tool-crate @ cargo-zng-res-tool-crate" {
                                    out_lines.next().unwrap(); // skip help
                                    out_lines.next().unwrap(); // skip empty line
                                } else {
                                    writeln!(&mut s, "{line}").unwrap();
                                }
                            }

                            writeln!(&mut s, "```").unwrap();
                            while let Some(l) = lines.next() {
                                if l == "```" {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if s != previous {
            std::fs::write(&readme, s.as_bytes()).unwrap();

            if previous == README_TEMPLATE {
                println("    generated");
            } else {
                println("    updated");
            }
        }
    }
}

struct Feature {
    name: String,
    docs: String,
}

fn read_features(cargo: &str) -> (Vec<Feature>, HashSet<String>) {
    let cargo = std::fs::read_to_string(cargo).unwrap();
    let mut r = vec![];
    let mut rd = HashSet::new();
    let mut in_features = false;

    let mut next_docs = String::new();

    let rgx = regex::Regex::new(r#"(\w+)\s*=\s*\[.*"#).unwrap();

    let mut lines = cargo.lines();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line == "[features]" {
            in_features = true;
        } else if in_features {
            use std::fmt::*;

            if line.starts_with('[') && line.ends_with(']') {
                break;
            }

            if line.starts_with('#') {
                let mut docs = &line[1..];
                if docs.starts_with(' ') {
                    docs = &docs[1..];
                }
                writeln!(&mut next_docs, "{docs}").unwrap();
            } else {
                if let Some(caps) = rgx.captures(&line) {
                    let name = caps.get(1).unwrap().as_str();
                    if name == "default" {
                        let s = line.find('[').unwrap();
                        let mut defaults = String::new();
                        if let Some(e) = line.find(']') {
                            defaults.push_str(&line[s + 1..e]);
                        } else {
                            defaults.push_str(&line[s + 1..]);
                            while let Some(line) = lines.next() {
                                if let Some(e) = line.find(']') {
                                    defaults.push_str(&line[..e]);
                                    break;
                                }
                                defaults.push_str(line);
                            }
                        }
                        for dft in defaults.split(',') {
                            rd.insert(dft.trim_matches(&['"', ' ']).to_owned());
                        }
                    } else {
                        r.push(Feature {
                            name: name.to_owned(),
                            docs: std::mem::take(&mut next_docs),
                        })
                    };
                } else {
                    next_docs.clear();
                }
            }
        }
    }
    (r, rd)
}

const HEADER: &str = "This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.\n";

const README_TEMPLATE: &str = "\
<!--do doc --readme header-->
.


<!--do doc --readme features-->


";

const FEATURES_HEADER: &str = "## Cargo Features";

const SECTION_END: &str = "<!--do doc --readme #SECTION-END-->";

/*
*  EXAMPLES README
*/

pub fn generate_examples(_args: Vec<&str>) {
    use std::fmt::*;

    const TAG: &str = "<!--do doc --readme-examples-->";
    let mut section = format!("{TAG}\n");

    for example in crate::util::examples() {
        if example.starts_with("test") {
            continue;
        }

        let file = format!("examples/{example}/src/main.rs");
        println(&file);

        let file = std::fs::read_to_string(file).unwrap();

        let mut docs = String::new();
        for line in file.lines() {
            let line = line.trim();

            if let Some(doc) = line.strip_prefix("//!") {
                writeln!(&mut docs, "{}", doc.trim_start()).unwrap();
            }
        }

        writeln!(&mut section, "### `{example}`\n").unwrap();

        let screenshot = format!("./{example}/res/screenshot.png");
        if PathBuf::from("examples").join(&screenshot).exists() {
            writeln!(&mut section, "<img alt='{example} screenshot' src='{screenshot}' width='300'>\n",).unwrap();
        }

        writeln!(&mut section, "Source: [{example}/src](./{example}/src)\n").unwrap();
        writeln!(&mut section, "```console\ncargo do run {example}\n```\n").unwrap();

        if docs.is_empty() {
            crate::error(format_args!("missing docs"));
        } else {
            writeln!(&mut section, "{docs}").unwrap();
        }
    }
    writeln!(&mut section, "{SECTION_END}").unwrap();

    let mut readme = std::fs::read_to_string("examples/README.md").unwrap();

    let s = readme.find(TAG).unwrap();
    let e = readme[s..].find(SECTION_END).unwrap();
    let e = s + e + SECTION_END.len() + "\n".len();
    readme.replace_range(s..e, &section);

    std::fs::write("examples/README.md", readme).unwrap();
}
