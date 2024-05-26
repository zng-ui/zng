use std::{
    fs, io,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use color_print::cstr;
use parking_lot::Mutex;

/// Visit in the `ToolKind` order.
pub fn visit_tools(local: &Path, mut tool: impl FnMut(Tool) -> anyhow::Result<ControlFlow<()>>) -> anyhow::Result<()> {
    macro_rules! tool {
        ($($args:tt)+) => {
            let flow = tool($($args)+)?;
            if flow.is_break() {
                return Ok(())
            }
        };
    }

    let mut local_bin_crate = None;
    if local.exists() {
        for entry in fs::read_dir(local).with_context(|| format!("cannot read_dir {}", local.display()))? {
            let path = entry.with_context(|| format!("cannot read_dir entry {}", local.display()))?.path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy();
                if let Some(name) = name.strip_prefix("cargo-zng-res-") {
                    if path.join("Cargo.toml").exists() {
                        tool!(Tool {
                            name: name.to_owned(),
                            kind: ToolKind::LocalCrate,
                            path,
                        });
                    }
                } else if name == "cargo-zng-res" && path.join("Cargo.toml").exists() {
                    local_bin_crate = Some(path);
                }
            }
        }
    }

    if let Some(path) = local_bin_crate {
        let bin_dir = path.join("src/bin");
        for entry in fs::read_dir(&bin_dir).with_context(|| format!("cannot read_dir {}", bin_dir.display()))? {
            let path = entry
                .with_context(|| format!("cannot read_dir entry {}", bin_dir.display()))?
                .path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if let Some(name) = name.strip_suffix(".rs") {
                    tool!(Tool {
                        name: name.to_owned(),
                        kind: ToolKind::LocalBin,
                        path,
                    });
                }
            }
        }
    }

    let current_exe = std::env::current_exe()?;

    for &name in crate::res::built_in::BUILT_INS {
        tool!(Tool {
            name: name.to_owned(),
            kind: ToolKind::BuiltIn,
            path: current_exe.clone(),
        });
    }

    let install_dir = current_exe
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no cargo install dir"))?;

    for entry in fs::read_dir(install_dir).with_context(|| format!("cannot read_dir {}", install_dir.display()))? {
        let path = entry
            .with_context(|| format!("cannot read_dir entry {}", install_dir.display()))?
            .path();
        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy();
            if let Some(name) = name.strip_prefix("cargo-zng-res-") {
                tool!(Tool {
                    name: name.split('.').next().unwrap().to_owned(),
                    kind: ToolKind::Installed,
                    path,
                });
            }
        }
    }

    Ok(())
}

pub struct Tool {
    pub name: String,
    pub kind: ToolKind,

    pub path: PathBuf,
}
impl Tool {
    pub fn help(&self) -> anyhow::Result<String> {
        self.run_cmd(self.cmd().arg("--help")).map(|o| o.output)
    }

    fn run(&self, cache: &Path, source: &Path, target: &Path, request: &Path) -> anyhow::Result<ToolOutput> {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();

        hasher.update(source.as_os_str().as_encoded_bytes());
        hasher.update(target.as_os_str().as_encoded_bytes());
        hasher.update(request.as_os_str().as_encoded_bytes());

        let mut hash_request = || -> anyhow::Result<()> {
            let mut file = fs::File::open(request)?;
            io::copy(&mut file, &mut hasher)?;
            Ok(())
        };
        if let Err(e) = hash_request() {
            fatal!("cannot read request `{}`, {e}", request.display());
        }

        let cache_dir = format!("{:x}", hasher.finalize());

        self.run_cmd(self.cmd().env(crate::res::built_in::CACHE_DIR, cache.join(cache_dir)))
    }

    fn run_final(&self, args: String) -> anyhow::Result<String> {
        let mut cmd = self.cmd();
        for arg in args.split(' ') {
            let arg = arg.trim();
            if !arg.is_empty() {
                cmd.arg(arg);
            }
        }
        self.run_cmd(&mut cmd).map(|o| o.output)
    }

    fn cmd(&self) -> std::process::Command {
        use std::process::Command;

        macro_rules! cargo_cmd {
            () => {{
                let mut cmd = Command::new("cargo");
                cmd.arg("run")
                    .arg("--quiet")
                    .arg("--manifest-path")
                    .arg(self.path.join("Cargo.toml"));
                cmd
            }};
        }

        match self.kind {
            ToolKind::LocalCrate => {
                let mut cmd = cargo_cmd!();
                cmd.arg("--");
                cmd
            }
            ToolKind::LocalBin => {
                let mut cmd = cargo_cmd!();
                cmd.arg("--bin").arg(&self.name).arg("--");
                cmd
            }
            ToolKind::BuiltIn => {
                let mut cmd = Command::new(&self.path);
                cmd.env(crate::res::built_in::ENV_TOOL, &self.name);
                cmd
            }
            ToolKind::Installed => Command::new(&self.path),
        }
    }

    fn run_cmd(&self, cmd: &mut std::process::Command) -> anyhow::Result<ToolOutput> {
        let output = cmd.output()?;
        if output.status.success() {
            Ok(ToolOutput::from(String::from_utf8_lossy(&output.stdout).into_owned()))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("{err}")
        }
    }
}

pub struct Tools {
    tools: Vec<Tool>,
    cache: PathBuf,
    on_final: Mutex<Vec<(usize, String)>>,
}
impl Tools {
    pub fn capture(local: &Path, cache: PathBuf) -> anyhow::Result<Self> {
        let mut tools = vec![];
        visit_tools(local, |t| {
            tools.push(t);
            Ok(ControlFlow::Continue(()))
        })?;
        Ok(Self {
            tools,
            cache,
            on_final: Mutex::new(vec![]),
        })
    }

    pub fn run(&self, tool_name: &str, source: &Path, target: &Path, request: &Path) -> anyhow::Result<String> {
        for (i, tool) in self.tools.iter().enumerate() {
            if tool.name == tool_name {
                let output = tool.run(&self.cache, source, target, request)?;
                for warn in output.warnings {
                    println!(cstr!("<bold><yellow>warning</yellow>:</bold> {}"), warn);
                }
                for args in output.on_final {
                    self.on_final.lock().push((i, args));
                }
                if !output.delegate {
                    return Ok(output.output);
                }
            }
        }
        bail!("no tool `{tool_name}` to handle request")
    }

    pub fn run_final(self) -> anyhow::Result<()> {
        for (i, args) in self.on_final.into_inner() {
            println!(cstr!("<bold>{}</bold> {}"), self.tools[i].name, args);
            self.tools[i].run_final(args)?;
        }
        Ok(())
    }
}

struct ToolOutput {
    // output without requests
    pub output: String,

    // zng-res::delegate
    pub delegate: bool,
    // zng-res::warning=
    pub warnings: Vec<String>,
    // zng-res::on-final=
    pub on_final: Vec<String>,
}
impl From<String> for ToolOutput {
    fn from(value: String) -> Self {
        let mut out = Self {
            output: String::new(),
            delegate: false,
            warnings: vec![],
            on_final: vec![],
        };
        for line in value.lines() {
            if line == "zng-res::delegate" {
                out.delegate = true;
            } else if let Some(w) = line.strip_prefix("zng-res::warning=") {
                out.warnings.push(w.to_owned());
            } else if let Some(a) = line.strip_prefix("zng-res::on-final=") {
                out.on_final.push(a.to_owned());
            } else {
                out.output.push_str(line);
                out.output.push('\n');
            }
        }
        out
    }
}

#[derive(Clone, Copy)]
pub enum ToolKind {
    LocalCrate,
    LocalBin,
    BuiltIn,
    Installed,
}
