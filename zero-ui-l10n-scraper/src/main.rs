pub mod scrap;

use std::io::Write;

use clap::Parser;

/// Localization text scraper
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Rust files glob
    #[arg(short, long)]
    input: String,

    /// Default template file
    #[arg(short, long)]
    output: String,

    /// Custom macro names, comma separated
    #[arg(short, long, default_value = "")]
    macros: String,

    /// Sort entries by the resource ID
    #[arg(short, long, default_value_t = true)]
    sort: bool,
}

fn main() {
    let args = Args::parse();

    println!(r#"searching {:?}.."#, args.input);

    let custom_macro_names: Vec<&str> = args.macros.split(',').map(|n| n.trim()).collect();

    match scrap::scrap_fluent_text(&args.input, &custom_macro_names) {
        Ok(t) => {
            match t.entries.len() {
                0 => {
                    println!("did not find any entry");
                    return;
                }
                1 => println!("found 1 entry"),
                n => println!("found {n} entries"),
            }
            let r = t.write(|file| {
                fn box_dyn(file: std::fs::File) -> Box<dyn Write + Send> {
                    Box::new(file)
                }
                if file.is_empty() {
                    std::fs::File::create(&args.output).map(box_dyn)
                } else {
                    std::fs::File::create(file).map(box_dyn)
                }
            });

            match r {
                Ok(()) => println!("done."),
                Err(e) => eprintln!("error: {e}"),
            }
        }
        Err(e) => eprintln!("error: {e}"),
    }
}
