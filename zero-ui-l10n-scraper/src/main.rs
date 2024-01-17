#![doc = include_str!("../../zero-ui-app/README.md")]

pub mod pseudo;
pub mod scraper;

use std::{borrow::Cow, io::Write, path::PathBuf};

use clap::Parser;

/// Localization text scraper
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Rust files glob
    #[arg(short, long)]
    input: String,

    /// Lang dir
    #[arg(short, long)]
    output: PathBuf,

    /// Custom macro names, comma separated
    #[arg(short, long, default_value = "")]
    macros: String,

    /// Pseudo Base name, empty to disable
    #[arg(long, default_value = "pseudo")]
    pseudo: String,
    /// Pseudo Mirrored name, empty to disable
    #[arg(long, default_value = "pseudo-mirr")]
    pseudo_m: String,
    /// Pseudo Wide name, empty to disable
    #[arg(long, default_value = "pseudo-wide")]
    pseudo_w: String,
}

fn main() {
    let args = Args::parse();

    println!(r#"searching {:?}.."#, args.input);

    let custom_macro_names: Vec<&str> = args.macros.split(',').map(|n| n.trim()).collect();

    if let Err(e) = std::fs::create_dir_all(&args.output) {
        println!("error: {e}");
        return;
    }

    match scraper::scrape_fluent_text(&args.input, &custom_macro_names) {
        Ok(mut template) => {
            match template.entries.len() {
                0 => {
                    println!("did not find any entry");
                    return;
                }
                1 => println!("found 1 entry"),
                n => println!("found {n} entries"),
            }

            struct Task {
                name: String,
                transform: fn(&str) -> Cow<str>,
            }
            let mut tasks = vec![Task {
                name: "template".to_owned(),
                transform: pseudo::none,
            }];
            if !args.pseudo.is_empty() {
                tasks.push(Task {
                    name: args.pseudo,
                    transform: pseudo::pseudo,
                })
            }
            if !args.pseudo_m.is_empty() {
                tasks.push(Task {
                    name: args.pseudo_m,
                    transform: pseudo::pseudo_mirr,
                })
            }
            if !args.pseudo_w.is_empty() {
                tasks.push(Task {
                    name: args.pseudo_w,
                    transform: pseudo::pseudo_wide,
                })
            }

            template.sort();

            for task in tasks {
                let r = template.write(task.transform, |file| {
                    fn box_dyn(file: std::fs::File) -> Box<dyn Write + Send> {
                        Box::new(file)
                    }

                    let mut output = args.output.clone();
                    if file.is_empty() {
                        output.push(format!("{}.ftl", task.name));
                    } else {
                        output.push(&task.name);
                        std::fs::create_dir_all(&output)?;
                        output.push(format!("{file}.ftl"));
                    }
                    std::fs::File::create(output).map(box_dyn)
                });

                match r {
                    Ok(()) => println!("finished {:?}.", task.name),
                    Err(e) => eprintln!("{:?} error: {}", task.name, e),
                }
            }
        }
        Err(e) => eprintln!("error: {e}"),
    }
}
