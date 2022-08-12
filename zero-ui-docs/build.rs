use minifier::{css, js};
use std::fs;
use std::{env, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    minify_js()?;
    minify_css()?;
    Ok(())
}

fn minify_js() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("js_min");
    fs::create_dir_all(&out_dir)?;

    let in_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("src/js");

    println!("cargo:rerun-if-changed={}", in_dir.display()); // in case a new JS gets added

    for entry in fs::read_dir(in_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file = entry.path();

        if file.extension().map(|e| e == "js").unwrap_or_default() {
            let js_str = fs::read_to_string(&file)?;

            let out_file = out_dir.join(file.file_name().unwrap());
            let out_file = fs::File::create(out_file)?;
            js::minify(&js_str).write(out_file)?;

            println!("cargo:rerun-if-changed={}", file.display());
        }
    }

    Ok(())
}

fn minify_css() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("css_min");
    fs::create_dir_all(&out_dir)?;

    let in_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("src/css");

    println!("cargo:rerun-if-changed={}", in_dir.display()); // in case a new CSS gets added

    for entry in fs::read_dir(in_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file = entry.path();

        if file.extension().map(|e| e == "css").unwrap_or_default() {
            let css_str = fs::read_to_string(&file)?;

            let out_file = out_dir.join(file.file_name().unwrap());
            let out_file = fs::File::create(out_file)?;
            css::minify(&css_str).unwrap().write(out_file)?;

            println!("cargo:rerun-if-changed={}", file.display());
        }
    }

    Ok(())
}
