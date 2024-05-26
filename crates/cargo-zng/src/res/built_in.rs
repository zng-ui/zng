//! Built-in tools

use clap::Parser;
use std::{
    env, fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

pub const ENV_TOOL: &str = "ZNG_RES_TOOL";

pub const CACHE_DIR: &str = "ZNG_RES_CACHE";

macro_rules! built_in {
    ($($tool:tt,)+) => {
        pub static BUILT_INS: &[&str] = &[
            $(stringify!($tool),)+
        ];
        static BUILT_IN_FNS: &[fn(ToolCli)] = &[
            $($tool,)+
        ];
    };
}
built_in! {
    copy,
}

pub fn run() {
    if let Ok(tool) = env::var(ENV_TOOL) {
        if let Some(i) = BUILT_INS.iter().position(|n| *n == tool.as_str()) {
            let cli = ToolCli::parse();
            (BUILT_IN_FNS[i])(cli);
        } else {
            fatal!("`tool` is not a built-in tool");
        }
    }
}

#[derive(Parser, Debug)]
#[command(long_about = None)]
struct ToolCli {
    /// Resources source dir
    #[arg(default_value = "")]
    source: PathBuf,
    /// Resources target dir
    #[arg(default_value = "")]
    target: PathBuf,
    /// The .zr-{tool} file
    #[arg(default_value = "")]
    request: PathBuf,
    /// If the tool requested to be called again on the final pass
    #[arg(long, default_value = "")]
    on_final: String,
    #[arg(long, action)]
    help: bool,
}
impl ToolCli {
    /// Get `request` without `.zr-*` and in the equivalent `target` dir.
    fn target_rq(&self) -> PathBuf {
        // if the request is already in `target`
        let mut target = self.request.with_extension("");

        // if the request is in `source`
        if let Ok(p) = target.strip_prefix(&self.source) {
            target = self.target.join(p);
        }

        target
    }
}

const COPY_HELP: &str = r#"
Copy the file or dir

The request file:
  source/foo.txt.zr-copy
  | # comment
  | path/bar.txt

Copies `path/bar.txt` to:
  target/foo.txt

Path is relative to the workspace root (where cargo res is called), 
unless it starts with `./`, them it is relative to the `.zr-copy` file.
"#;
fn copy(cli: ToolCli) {
    if cli.help {
        println!("{COPY_HELP}");
        return;
    }

    // read source
    let source = read_path(&cli.request).unwrap_or_else(|e| fatal!("{e}"));
    // target derived from the request file name
    let mut target = cli.target_rq();
    // request without name "./.zr-copy", take name from source (this is deliberate not documented)
    if target.ends_with(".zr-copy") {
        target = target.with_file_name(source.file_name().unwrap());
    }

    if source.is_dir() {
        copy_dir_all(&source, &target, true).unwrap_or_else(|e| fatal!("{e}"));
    } else {
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
        println!("{}", target.display());
    }
}

/// Read first non-comment non-empty line
fn read_line(path: &Path, expected: &str) -> io::Result<String> {
    let file = fs::File::open(path)?;
    for line in io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            return Ok(line.to_owned());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("expected {expected} in tool file content"),
    ))
}

fn read_path(path: &Path) -> io::Result<PathBuf> {
    let path = PathBuf::from(read_line(path, "path")?);
    if let Ok(p) = path.strip_prefix(".") {
        // './' is relative to the request
        Ok(path.parent().unwrap().join(p))
    } else {
        Ok(path)
    }
}

fn copy_dir_all(from: &Path, to: &Path, trace: bool) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let from = entry?.path();
        if from.is_dir() {
            let to = to.join(from.file_name().unwrap());
            fs::create_dir(&to)?;
            if trace {
                println!("{}", to.display());
            }
            copy_dir_all(&from, &to, trace)?;
        } else if from.is_file() {
            let to = to.join(from.file_name().unwrap());
            fs::copy(&from, &to)?;
            if trace {
                println!("{}", to.display());
            }
        } else {
            continue;
        }
    }
    Ok(())
}
