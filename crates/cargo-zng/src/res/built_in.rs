//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

/// Environment variable set by cargo-zng to dir named
/// from a hash of source, target, request and request content
pub const CACHE_DIR: &str = "ZNG_RES_CACHE";

/// See [`ToolCli::Request`].
pub struct ToolRequest {
    /// Resources source dir
    source: PathBuf,
    /// Resources target dir
    target: PathBuf,
    /// The .zr-{tool} file
    request: PathBuf,
}
impl ToolRequest {
    /// Derive target file path from request path.
    ///
    /// Gets `request` without `.zr-*` and in the equivalent `target` dir.
    pub fn target_file(&self) -> PathBuf {
        // if the request is already in `target` (recursion)
        let mut target = self.request.with_extension("");
        // if the request is in `source`
        if let Ok(p) = target.strip_prefix(&self.source) {
            target = self.target.join(p);
        }
        target
    }

    /// Cargo workspace is the `std::env::current_dir`. Unless `source` is not inside
    /// a Cargo project, them it is the workspace.
    pub fn workspace(&self) -> PathBuf {
        std::env::current_dir().unwrap()
    }

    /// Cache dir named from a hash of source, target, request and request content
    pub fn cache(&self) -> PathBuf {
        std::env::var(CACHE_DIR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join(self.request.file_name().unwrap()))
    }
}

const COPY_HELP: &str = "\
Copy the file or dir

The request file:
  source/foo.txt.zr-copy
   | # comment
   | path/bar.txt

Copies `path/bar.txt` to:
  target/foo.txt

Path is relative to the Cargo workspace root, unless it starts with `./`,
them it is relative to the `.zr-copy` file.
";
fn copy(cli: ToolCli) {
    let args = match cli {
        ToolCli::Request(r) => r,
        ToolCli::Help => return println!("{COPY_HELP}"),
        ToolCli::OnFinal(_) => fatal!("did not request"),
    };

    // read source
    let source = read_path(&args.request).unwrap_or_else(|e| fatal!("{e}"));
    // target derived from the request file name
    let mut target = args.target_file();
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

/// CLI arguments for a `cargo-zng-res-{tool}`.
///
/// Copy this type to your own custom tool to use.
pub enum ToolCli {
    /// Print help (for cargo zng res --list)
    Help,

    /// Run tool
    Request(ToolRequest),

    /// If tool requested 'zng-res::on-final={args}'
    OnFinal(String),
}
impl ToolCli {
    pub fn parse() -> Self {
        Self::try_parse().unwrap_or_else(|| fatal!("use cargo-zng to call this tool"))
    }
    fn try_parse() -> Option<Self> {
        let mut args: Vec<_> = std::env::args().skip(1).take(4).collect();
        match args.len() {
            1 if args[0] == "--help" => Some(Self::Help),
            2 if args[0] == "--on-final" => Some(Self::OnFinal(args.remove(1))),
            3 if args.iter().all(|a| !a.starts_with('-')) => Some(Self::Request(ToolRequest {
                request: args.remove(2).into(),
                target: args.remove(1).into(),
                source: args.remove(0).into(),
            })),
            _ => None,
        }
    }
}

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

pub const ENV_TOOL: &str = "ZNG_RES_TOOL";

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
            std::process::exit(0);
        } else {
            fatal!("`tool` is not a built-in tool");
        }
    }
}
