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

use crate::util;

use self::tool::Tools;

mod about;
pub mod built_in;
mod tool;

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

    /// Use verbose output.
    #[arg(short, long, action)]
    verbose: bool,
}

fn canonicalize(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|e| fatal!("cannot resolve path, {e}"))
}

pub(crate) fn run(mut args: ResArgs) {
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
        let about = about::find_about(args.metadata.as_deref(), args.verbose);
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
    if let Err(e) = fs::remove_dir_all(&args.target) {
        if e.kind() != io::ErrorKind::NotFound {
            fatal!("cannot remove target dir, {e}");
        }
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

    let about = about::find_about(args.metadata.as_deref(), args.verbose);

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
        std::env::set_var(ZR_WORKSPACE_DIR, std::env::current_dir().unwrap());
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
    let tools = Tools::capture(&args.tool_dir, args.tool_cache.clone(), about, args.verbose)?;
    source_to_target_pass(args, &tools, &args.source, &args.target)?;

    let mut passes = 0;
    while target_to_target_pass(args, &tools, &args.target)? {
        passes += 1;
        if passes >= args.recursion_limit {
            bail!("reached --recursion-limit of {}", args.recursion_limit)
        }
    }

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
            let source = entry.path();

            // run tool
            if let Some(ext) = source.extension() {
                let ext = ext.to_string_lossy();
                if let Some(tool) = ext.strip_prefix("zr-") {
                    // run prints request
                    tools.run(tool, &args.source, &args.target, source)?;
                    continue;
                }
            }

            // or pack
            if args.pack {
                println!("{}", display_path(source));
                let target = target.join(source.file_name().unwrap());
                fs::copy(source, &target).with_context(|| format!("cannot copy {} to {}", source.display(), target.display()))?;
                println!(cstr!("  <dim>{}</>"), display_path(&target));
            }
        } else if entry.file_type().is_symlink() {
            built_in::symlink_warn(entry.path());
        }
    }
    Ok(())
}

fn target_to_target_pass(args: &ResArgs, tools: &Tools, dir: &Path) -> anyhow::Result<bool> {
    let mut any = false;
    for entry in walkdir::WalkDir::new(dir).min_depth(1).sort_by_file_name() {
        let entry = entry.with_context(|| format!("cannot read dir entry {}", dir.display()))?;
        if entry.file_type().is_file() {
            let path = entry.path();
            // run tool
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                if let Some(tool) = ext.strip_prefix("zr-") {
                    any = true;
                    // run prints request
                    let tool_r = tools.run(tool, &args.source, &args.target, path);
                    fs::remove_file(path)?;
                    tool_r?;
                }
            }
        }
    }
    Ok(any)
}

fn tools_help(tools: &Path) {
    let r = tool::visit_tools(tools, |tool| {
        if crate::util::ansi_enabled() {
            println!(cstr!("<bold>.zr-{}</bold> @ {}"), tool.name, display_tool_path(&tool.path));
        } else {
            println!(".zr-{} @ {}", tool.name, display_tool_path(&tool.path));
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
    let name = name.strip_prefix(".zr-").unwrap_or(name);
    let mut found = false;
    let r = tool::visit_tools(tools, |tool| {
        if tool.name == name {
            if crate::util::ansi_enabled() {
                println!(cstr!("<bold>.zr-{}</bold> @ {}"), tool.name, display_tool_path(&tool.path));
            } else {
                println!(".zr-{}</bold> @ {}", tool.name, display_tool_path(&tool.path));
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
    return r.replace('\\', "/");

    #[cfg(not(windows))]
    r
}
