use std::fs;

use crate::util::glob_walker;

/// Rename **/*.zr-* to **/* '*
pub(crate) fn zr(p: std::path::PathBuf) {
    let pat = format!("{}/**/*.zr-*", p.to_str().unwrap_or_else(|| fatal!("path not utf-8")));
    let iter = glob_walker(&pat, false).unwrap_or_else(|e| fatal!("{e}"));

    let mut targets = vec![];
    for p in iter {
        let p = p.unwrap_or_else(|e| fatal!("{e}")).into_path();
        targets.push(p);
    }

    for p in targets {
        println!("{}", p.display());

        let name = p.file_name().unwrap();
        let name = match name.to_str() {
            Some(n) => n,
            None => {
                error!("   not utf-8");
                continue;
            }
        };
        let (name, tools) = name.split_once(".zr-").unwrap();
        let mut name = name.to_owned();
        name.push(' ');
        for tool in tools.rsplit(".zr-") {
            name.push('\'');
            name.push_str(tool);
        }
        let target = p.with_file_name(&name);
        match fs::rename(p, target) {
            Ok(_) => println!("   {name}"),
            Err(e) => error!("   {e}"),
        }
    }
}
