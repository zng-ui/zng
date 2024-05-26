use std::{
    fs, io,
    ops::ControlFlow,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context as _};

use clap::*;
use color_print::cstr;

use self::tool::Tools;

pub mod built_in;
mod tool;

#[derive(Args, Debug)]
pub struct ResArgs {
    /// Resources source dir
    #[arg(default_value = "assets")]
    source: PathBuf,
    /// Resources target dir
    ///
    /// This directory is wiped before each build.
    #[arg(default_value = "target/assets")]
    target: PathBuf,

    /// Copy all static files to the target dir
    #[arg(long, action)]
    pack: bool,

    /// Search for `zng-res-{tool}` in this directory first
    #[arg(long, default_value = "tools")]
    tools: PathBuf,
    /// Prints help for all tools available
    #[arg(long, action)]
    list: bool,

    /// Tool cache dir
    #[arg(long, default_value = "target/assets.cache")]
    tool_cache: PathBuf,

    /// Number of build passes allowed before final
    #[arg(long, default_value = "32")]
    recursion_limit: u32,
}
impl ResArgs {
    fn canonicalize(&mut self) -> io::Result<()> {
        self.source = dunce::canonicalize(&self.source)?;
        self.target = dunce::canonicalize(&self.target)?;
        self.tools = dunce::canonicalize(&self.tools)?;
        self.tool_cache = dunce::canonicalize(&self.tool_cache)?;

        Ok(())
    }
}

pub(crate) fn run(mut args: ResArgs) {
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

    args.canonicalize().unwrap();

    if args.list {
        return list(&args.tools);
    }

    let start = Instant::now();
    if let Err(e) = build(&args) {
        fatal!("build failed, {e}")
    }

    println!(cstr!("<bold><green>Finished</green></bold> res build in {:?}"), start.elapsed());
    println!("         {}", args.target.display());
}

fn build(args: &ResArgs) -> anyhow::Result<()> {
    let tools = Tools::capture(&args.tools, args.tool_cache.clone())?;
    source_to_target_pass(args, &tools, &args.source, &args.target)?;

    let mut passes = 0;
    while target_to_target_pass(args, &tools, &args.target)? {
        passes += 1;
        if passes >= args.recursion_limit {
            bail!("reached --recursion-limit of {}", args.recursion_limit)
        }
    }

    tools.run_final()
}

fn source_to_target_pass(args: &ResArgs, tools: &Tools, source: &Path, target: &Path) -> anyhow::Result<()> {
    for entry in fs::read_dir(source).with_context(|| format!("cannot read_dir {}", source.display()))? {
        let source = entry.with_context(|| format!("cannot read_dir entry {}", source.display()))?.path();
        if source.is_dir() {
            // mirror dir in target
            println!("{}", source.display());
            let target = target.join(source.file_name().unwrap());
            fs::create_dir(&target).with_context(|| format!("cannot create_dir {}", target.display()))?;
            println!("   {}", target.display());
            // recursive walk
            source_to_target_pass(args, tools, &source, &target)?;
        } else if source.is_file() {
            // run tool
            if let Some(ext) = source.extension() {
                let ext = ext.to_string_lossy();
                if let Some(tool) = ext.strip_prefix("zr-") {
                    println!("{}", source.display());
                    let output = tools.run(tool, &args.source, &args.target, &source)?;
                    for line in output.lines() {
                        println!("   {line}");
                    }
                    continue;
                }
            }

            // or pack
            if args.pack {
                println!("{}", source.display());
                let target = target.join(source.file_name().unwrap());
                fs::copy(&source, &target).with_context(|| format!("cannot copy {} to {}", source.display(), target.display()))?;
                println!("   {}", target.display());
            }
        }
    }
    Ok(())
}

fn target_to_target_pass(args: &ResArgs, tools: &Tools, dir: &Path) -> anyhow::Result<bool> {
    let mut any = false;
    for entry in fs::read_dir(dir).with_context(|| format!("cannot read_dir {}", dir.display()))? {
        let path = entry.with_context(|| format!("cannot read_dir entry {}", dir.display()))?.path();
        if path.is_dir() {
            any |= target_to_target_pass(args, tools, &path)?;
        } else if path.is_file() {
            // run tool
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                if let Some(tool) = ext.strip_prefix("zr-") {
                    any = true;
                    println!("{}", path.display());
                    let output = tools.run(tool, &args.source, &args.target, &path)?;
                    for line in output.lines() {
                        println!("   {line}");
                    }
                }
            }
        }
    }
    Ok(any)
}

fn list(tools: &Path) {
    let r = tool::visit_tools(tools, |tool| {
        println!(cstr!("<bold>.zr-{}</bold> @ {}"), tool.name, tool.path.display());
        match tool.help() {
            Ok(h) => {
                for line in h.lines() {
                    println!("  {line}");
                }
                if !h.is_empty() {
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
}
