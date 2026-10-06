use std::{
    fs, io,
    ops::ControlFlow,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context as _, bail};

use built_in::{ZR_WORKSPACE_DIR, display_path};
use clap::*;
use color_print::cstr;
use zng_env::About;

use crate::util::{self, unix_path};

use self::tool::Tools;

mod about;
pub mod built_in;
mod tool;

mod upgrade;

#[derive(Args, Debug)]
pub struct ResArgs {
    /// Resources source dir
    #[arg(default_value = "res")]
    source: PathBuf,
    /// Resources target dir
    ///
    /// This directory is wiped before each build.
    #[arg(default_value = "target/res")]
    target: PathBuf,

    /// Copy all static files to the target dir
    #[arg(long, action)]
    pack: bool,

    /// Search for `zng-res-{tool}` in this directory first
    #[arg(long, default_value = "tools", value_name = "DIR")]
    tool_dir: PathBuf,
    /// Prints help for all tools available
    #[arg(long, action)]
    tools: bool,
    /// Prints the full help for a tool
    #[arg(long)]
    tool: Option<String>,

    /// Tools cache dir
    #[arg(long, default_value = "target/res.cache")]
    tool_cache: PathBuf,

    /// Number of build passes allowed before final
    #[arg(long, default_value = "32")]
    recursion_limit: u32,

    /// TOML file that that defines metadata uses by tools (ZR_APP, ZR_ORG, ..)
    ///
    /// This is only needed if the workspace has multiple bin crates
    /// and none or many set '[package.metadata.zng.about]'.
    ///
    /// See `zng::env::About` for more details.
    #[arg(long, value_name = "TOML_FILE")]
    metadata: Option<PathBuf>,

    /// Writes the metadata extracted the workspace or --metadata
    #[arg(long, action)]
    metadata_dump: bool,

    /// Rename all "name.zr-tool" files to new syntax "name 'tool"
    #[arg(long, value_name = "DIR")]
    upgrade_zr: Option<PathBuf>,
}

fn canonicalize(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|e| fatal!("cannot resolve path, {e}"))
}

pub(crate) fn run(mut args: ResArgs) {
    if let Some(p) = args.upgrade_zr {
        return upgrade::zr(p);
    }

    if args.tool_dir.exists() {
        args.tool_dir = canonicalize(&args.tool_dir);
    }
    if args.tools {
        return tools_help(&args.tool_dir);
    }
    if let Some(t) = args.tool {
        return tool_help(&args.tool_dir, &t);
    }

    if args.metadata_dump {
        let about = about::find_about(args.metadata.as_deref());
        crate::res::tool::visit_about_vars(&about, |key, value| {
            println!("{key}={value}");
        });
        return;
    }

    if !args.source.exists() {
        fatal!("source dir does not exist");
    }
    if let Err(e) = fs::create_dir_all(&args.tool_cache) {
        fatal!("cannot create cache dir, {e}");
    }
    if let Err(e) = fs::remove_dir_all(&args.target)
        && e.kind() != io::ErrorKind::NotFound
    {
        fatal!("cannot remove target dir, {e}");
    }
    if let Err(e) = fs::create_dir_all(&args.target) {
        fatal!("cannot create target dir, {e}");
    }

    args.source = canonicalize(&args.source);
    args.target = canonicalize(&args.target);
    args.tool_cache = canonicalize(&args.tool_cache);

    if args.source == args.target {
        fatal!("cannot build res to same dir");
    }

    let about = about::find_about(args.metadata.as_deref());

    // tool request paths are relative to the workspace root
    if let Some(p) = util::workspace_dir() {
        if let Err(e) = std::env::set_current_dir(p) {
            fatal!("cannot change dir, {e}");
        }
    } else {
        warn!("source is not in a cargo workspace, tools will run using source as root");
        if let Err(e) = std::env::set_current_dir(&args.source) {
            fatal!("cannot change dir, {e}");
        }
    }

    unsafe {
        // SAFETY: cargo-zng res is single-threaded
        //
        // to use `display_path` in the tool runner (current process)
        std::env::set_var(ZR_WORKSPACE_DIR, &*unix_path(&std::env::current_dir().unwrap()));
    }

    let start = Instant::now();
    if let Err(e) = build(&args, about) {
        let e = e.to_string();
        for line in e.lines() {
            eprintln!("   {line}");
        }
        fatal!("res build failed");
    }

    println!(cstr!("<bold><green>Finished</green></bold> res build in {:?}"), start.elapsed());
    println!("         {}", args.target.display());
}

fn build(args: &ResArgs, about: About) -> anyhow::Result<()> {
    let tools = Tools::capture(&args.tool_dir, args.tool_cache.clone(), about)?;
    source_to_target_pass(args, &tools, &args.source, &args.target)?;
    tools.run_final(&args.source, &args.target)
}

fn source_to_target_pass(args: &ResArgs, tools: &Tools, source: &Path, target: &Path) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(source).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry.with_context(|| format!("cannot read dir entry {}", source.display()))?;
        if entry.file_type().is_dir() {
            let source = entry.path();
            // mirror dir in target
            println!("{}", display_path(source));
            let target = target.join(source.file_name().unwrap());
            fs::create_dir(&target).with_context(|| format!("cannot create_dir {}", target.display()))?;
            println!(cstr!("  <dim>{}</>"), display_path(&target));

            source_to_target_pass(args, tools, source, &target)?;
        } else if entry.file_type().is_file() {
            let request = entry.path();

            // run tool
            if let Some((request_wt, tool)) = take_tool(request) {
                // run prints request path
                tools.run(tool, &args.source, &args.target, request, &request_wt)?;

                // recurse immediately
                let mut passes = 0;
                while target_to_target_pass(args, tools, &args.target)? {
                    passes += 1;
                    if passes >= args.recursion_limit {
                        bail!("reached --recursion-limit of {}", args.recursion_limit)
                    }
                }
                continue;
            }

            // or pack
            if args.pack {
                println!("{}", display_path(request));
                let target = target.join(request.file_name().unwrap());
                fs::copy(request, &target).with_context(|| format!("cannot copy {} to {}", request.display(), target.display()))?;
                println!(cstr!("  <dim>{}</>"), display_path(&target));
            }
        } else if entry.file_type().is_symlink() {
            built_in::symlink_warn(entry.path());
        }
    }
    Ok(())
}

fn target_to_target_pass(args: &ResArgs, tools: &Tools, dir: &Path) -> anyhow::Result<bool> {
    let mut advanced = false;
    for entry in walkdir::WalkDir::new(dir).min_depth(1).sort_by_file_name() {
        let entry = entry.with_context(|| format!("cannot read dir entry {}", dir.display()))?;
        if entry.file_type().is_file() {
            let request = entry.path();

            // run tool
            if let Some((request_wt, tool)) = take_tool(request) {
                // run prints request path and removes file if not needed
                let done = tools.run(tool, &args.source, &args.target, request, &request_wt)?;
                if done {
                    advanced = true;
                }
            }
        }
    }
    Ok(advanced)
}

fn tools_help(tools: &Path) {
    let r = tool::visit_tools(tools, |tool| {
        if crate::util::ansi_enabled() {
            println!(cstr!("<bold>'{}</bold> @ {}"), tool.name, display_tool_path(&tool.path));
        } else {
            println!("'{} @ {}", tool.name, display_tool_path(&tool.path));
        }
        match tool.help() {
            Ok(h) => {
                if let Some(line) = h.trim().lines().next() {
                    println!("  {line}");
                    println!();
                }
            }
            Err(e) => error!("{e}"),
        }
        Ok(ControlFlow::Continue(()))
    });
    if let Err(e) = r {
        fatal!("{e}")
    }
    println!("call 'cargo zng res --help tool' to read full help from a tool");
}

fn tool_help(tools: &Path, name: &str) {
    let name = name.strip_prefix("'").unwrap_or(name);
    let mut found = false;
    let r = tool::visit_tools(tools, |tool| {
        if tool.name == name {
            if crate::util::ansi_enabled() {
                println!(cstr!("<bold>'{}</bold> @ {}"), tool.name, display_tool_path(&tool.path));
            } else {
                println!("'{} @ {}", tool.name, display_tool_path(&tool.path));
            }
            match tool.help() {
                Ok(h) => {
                    for line in h.trim().lines() {
                        println!("  {line}");
                    }
                    if !h.is_empty() {
                        println!();
                    }
                }
                Err(e) => error!("{e}"),
            }
            found = true;
            Ok(ControlFlow::Break(()))
        } else {
            Ok(ControlFlow::Continue(()))
        }
    });
    if let Err(e) = r {
        fatal!("{e}")
    }
    if !found {
        fatal!("did not find tool `{name}`")
    }
}

fn display_tool_path(p: &Path) -> String {
    let base = util::workspace_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    let r = if let Ok(local) = p.strip_prefix(base) {
        local.display().to_string()
    } else {
        p.file_name().unwrap().to_string_lossy().into_owned()
    };

    #[cfg(windows)]
    return r.replace('\\', "/").trim_end_matches(".exe").to_owned();

    #[cfg(not(windows))]
    r
}

fn take_tool(p: &Path) -> Option<(PathBuf, &str)> {
    if let Some(ext) = p.extension()
        && let Some(ext) = ext.to_str()
        && let Some(tool) = ext.strip_prefix("zr-")
    {
        warn!("deprecated syntax, call \"cargo zng res --upgrade-zr .\" to auto upgrade");
        return Some((p.with_extension(""), tool));
    }

    if let Some(name) = p.file_name()
        && let Some(name) = name.to_str()
        && let Some((file, tools)) = name.split_once(" '")
    {
        let name = file.trim_end(); // "name.txt   'tool"
        let mut tools = tools.split('\'');
        let tool = tools.next().unwrap().trim();

        if let Some(next_tool) = tools.next() {
            let mut name = format!("{name} '{}", next_tool.trim());
            for next_tool in tools {
                name.push('\'');
                name.push_str(next_tool.trim());
            }
            // "name.txt 'next_tool1'tool2"
            return Some((p.with_file_name(name), tool));
        }

        return Some((p.with_file_name(name), tool));
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::take_tool;

    #[test]
    fn take_legacy_one() {
        let path = PathBuf::from("foo/bar/name.txt.zr-copy");
        let (file, tool) = take_tool(&path).unwrap();
        assert_eq!(tool, "copy");
        assert_eq!(file.to_str().unwrap(), "foo/bar/name.txt");
    }

    #[test]
    fn take_legacy_two() {
        let path = PathBuf::from("foo/bar/name.txt.zr-copy.zr-rp");
        let (file, tool) = take_tool(&path).unwrap();
        assert_eq!(tool, "rp");
        assert_eq!(file.to_str().unwrap(), "foo/bar/name.txt.zr-copy");

        let (file, tool) = take_tool(&file).unwrap();
        assert_eq!(tool, "copy");
        assert_eq!(file.to_str().unwrap(), "foo/bar/name.txt");
    }

    #[test]
    fn take_one() {
        let path = PathBuf::from("foo/bar/name.txt.zr-copy");
        let (file, tool) = take_tool(&path).unwrap();
        assert_eq!(tool, "copy");
        assert_eq!(file.to_str().unwrap(), "foo/bar/name.txt");
    }

    #[test]
    fn take_two() {
        let path = PathBuf::from("foo/bar/name.txt 'rp'copy");
        let (file, tool) = take_tool(&path).unwrap();
        assert_eq!(tool, "rp");
        assert_eq!(file.to_str().unwrap(), Path::new("foo/bar/name.txt 'copy"));

        let (file, tool) = take_tool(&file).unwrap();
        assert_eq!(tool, "copy");
        assert_eq!(file.to_str().unwrap(), Path::new("foo/bar/name.txt"));
    }

    #[test]
    fn take_two_space() {
        let path = PathBuf::from("foo/bar/name.txt 'rp 'copy");
        let (file, tool) = take_tool(&path).unwrap();
        assert_eq!(tool, "rp");
        assert_eq!(file.to_str().unwrap(), Path::new("foo/bar/name.txt 'copy"));

        let (file, tool) = take_tool(&file).unwrap();
        assert_eq!(tool, "copy");
        assert_eq!(file.to_str().unwrap(), Path::new("foo/bar/name.txt"));
    }
}
