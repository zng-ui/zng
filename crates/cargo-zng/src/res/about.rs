use std::{cmp::Ordering, fmt::Write as _, fs, path::Path, process::Stdio};

use crate::util::workspace_dir;

pub fn find_about(metadata: Option<&Path>, verbose: bool) -> zng_env::About {
    if let Some(m) = metadata {
        if verbose {
            println!("parsing `{}`", m.display());
        }

        let cargo_toml = fs::read_to_string(m).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", m.display()));
        return zng_env::About::parse_manifest(&cargo_toml).unwrap_or_else(|e| fatal!("cannot parse `{}`, {e}", m.display()));
    }

    let mut options = Vec::with_capacity(1);

    let workspace_manifest =
        workspace_dir().unwrap_or_else(|| fatal!("cannot locate workspace, use --metadata if source is not in a cargo project"));
    if verbose {
        println!("workspace `{}`", workspace_manifest.display())
    }

    for manifest in glob::glob("**/Cargo.toml").unwrap_or_else(|e| fatal!("cannot search metadata, {e}")) {
        let manifest = manifest.unwrap_or_else(|e| fatal!("error searching metadata, {e}"));
        let manifest_dir = match manifest.parent() {
            Some(p) => p,
            None => continue,
        };

        let output = std::process::Command::new("cargo")
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format=plain")
            .current_dir(manifest_dir)
            .stderr(Stdio::inherit())
            .output()
            .unwrap_or_else(|e| fatal!("cannot locate workspace, {e}"));
        if !output.status.success() {
            continue;
        }
        let w2 = Path::new(std::str::from_utf8(&output.stdout).unwrap().trim()).parent().unwrap();
        if w2 != workspace_manifest {
            if verbose {
                println!("skip `{}` cause it is not a workspace member", manifest.display())
            }
            continue;
        }

        let cargo_toml = fs::read_to_string(&manifest).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", manifest.display()));
        let about = zng_env::About::parse_manifest(&cargo_toml).unwrap_or_else(|e| fatal!("cannot parse `{}`, {e}", manifest.display()));

        if about.has_about || manifest_dir.join("src/main.rs").exists() {
            options.push(about);
        } else if verbose {
            println!(
                "skip `{}` cause it has no zng metadata and/or it is not a bin crate",
                manifest.display()
            );
        }
    }

    match options.len().cmp(&1) {
        Ordering::Less => fatal!("cannot find main crate metadata, workspace has no bin crate, use --metadata to select a source"),
        Ordering::Equal => options.remove(0),
        Ordering::Greater => {
            let mut main_options = Vec::with_capacity(1);
            for (i, o) in options.iter().enumerate() {
                if o.has_about {
                    main_options.push(i);
                }
            }
            match main_options.len().cmp(&1) {
                Ordering::Equal => options.remove(main_options[0]),
                Ordering::Less => {
                    let mut msg = "cannot find main crate metadata, workspace has multiple bin crates\n".to_owned();
                    for o in &options {
                        writeln!(&mut msg, "   {}", o.pkg_name).unwrap();
                    }
                    writeln!(
                        &mut msg,
                        "set [package.metadata.zng.about]app=\"Display Name\" in one of the crates\nor use --metadata to select the source"
                    )
                    .unwrap();
                    fatal!("{msg}");
                }
                Ordering::Greater => {
                    let mut msg = "cannot find main crate metadata, workspace has multiple metadata sources\n".to_owned();
                    for i in main_options {
                        writeln!(&mut msg, "   {}", options[i].pkg_name).unwrap();
                    }
                    writeln!(
                        &mut msg,
                        "set [package.metadata.zng.about] in only one crate\nor use --metadata to select the source"
                    )
                    .unwrap();
                    fatal!("{msg}");
                }
            }
        }
    }
}
