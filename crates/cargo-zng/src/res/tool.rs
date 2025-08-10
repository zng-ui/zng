use std::{
    fs,
    io::{self, BufRead, Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use is_executable::IsExecutable as _;
use parking_lot::Mutex;
use zng_env::About;

use crate::res_tool_util::*;

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
            if let Some(name) = name.strip_prefix("cargo-zng-res-")
                && path.is_executable()
            {
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

pub fn visit_about_vars(about: &About, mut visit: impl FnMut(&str, &str)) {
    visit(ZR_APP, &about.app);
    visit(ZR_CRATE_NAME, &about.crate_name);
    visit(ZR_HOMEPAGE, &about.homepage);
    visit(ZR_LICENSE, &about.license);
    visit(ZR_ORG, &about.org);
    visit(ZR_PKG_AUTHORS, &about.pkg_authors.clone().join(","));
    visit(ZR_PKG_NAME, &about.pkg_name);
    visit(ZR_QUALIFIER, &about.qualifier);
    visit(ZR_VERSION, &about.version.to_string());
    visit(ZR_DESCRIPTION, &about.description);
}

pub struct Tool {
    pub name: String,
    pub kind: ToolKind,

    pub path: PathBuf,
}
impl Tool {
    pub fn help(&self) -> anyhow::Result<String> {
        let out = self.cmd().env(ZR_HELP, "").output()?;
        if !out.status.success() {
            let error = String::from_utf8_lossy(&out.stderr);
            bail!("{error}\nhelp run failed, exit code {}", out.status.code().unwrap_or(0));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn run(
        &self,
        cache: &Path,
        source_dir: &Path,
        target_dir: &Path,
        request: &Path,
        about: &About,
        final_args: Option<String>,
    ) -> anyhow::Result<ToolOutput> {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();

        hasher.update(source_dir.as_os_str().as_encoded_bytes());
        hasher.update(target_dir.as_os_str().as_encoded_bytes());
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

        let mut cmd = self.cmd();
        if let Some(args) = final_args {
            cmd.env(ZR_FINAL, args);
        }

        // if the request is already in `target` (recursion)
        let mut target = request.with_extension("");
        // if the request is in `source`
        if let Ok(p) = target.strip_prefix(source_dir) {
            target = target_dir.join(p);
        }

        cmd.env(ZR_WORKSPACE_DIR, std::env::current_dir().unwrap())
            .env(ZR_SOURCE_DIR, source_dir)
            .env(ZR_TARGET_DIR, target_dir)
            .env(ZR_REQUEST_DD, request.parent().unwrap())
            .env(ZR_REQUEST, request)
            .env(ZR_TARGET_DD, target.parent().unwrap())
            .env(ZR_TARGET, target)
            .env(ZR_CACHE_DIR, cache.join(cache_dir));
        visit_about_vars(about, |key, value| {
            cmd.env(key, value);
        });
        self.run_cmd(&mut cmd)
    }

    fn cmd(&self) -> std::process::Command {
        use std::process::Command;

        match self.kind {
            ToolKind::LocalCrate => {
                let mut cmd = Command::new("cargo");
                cmd.arg("run")
                    .arg("--quiet")
                    .arg("--manifest-path")
                    .arg(self.path.join("Cargo.toml"))
                    .arg("--");
                cmd
            }
            ToolKind::LocalBin => {
                let mut cmd = Command::new("cargo");
                cmd.arg("run")
                    .arg("--quiet")
                    .arg("--manifest-path")
                    .arg(self.path.parent().unwrap().parent().unwrap().parent().unwrap().join("Cargo.toml"))
                    .arg("--bin")
                    .arg(&self.name)
                    .arg("--");
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
        let mut cmd = cmd
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // indent stderr
        let cmd_err = cmd.stderr.take().unwrap();
        let error_pipe = std::thread::spawn(move || {
            for line in io::BufReader::new(cmd_err).lines() {
                match line {
                    Ok(l) => eprintln!("  {l}"),
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                }
            }
        });

        // indent stdout and capture "zng-res::" requests
        let mut requests = vec![];
        const REQUEST: &[u8] = b"zng-res::";
        let mut cmd_out = cmd.stdout.take().unwrap();
        let mut out = io::stdout();
        let mut buf = [0u8; 1024];

        let mut at_line_start = true;
        let mut maybe_request_start = None;

        print!("\x1B[2m"); // dim
        loop {
            let len = cmd_out.read(&mut buf)?;
            if len == 0 {
                break;
            }

            for s in buf[..len].split_inclusive(|&c| c == b'\n') {
                if at_line_start {
                    if s.starts_with(REQUEST) || REQUEST.starts_with(s) {
                        maybe_request_start = Some(requests.len());
                    }
                    if maybe_request_start.is_none() {
                        out.write_all(b"  ")?;
                    }
                }
                if maybe_request_start.is_none() {
                    out.write_all(s)?;
                    out.flush()?;
                } else {
                    requests.write_all(s).unwrap();
                }

                at_line_start = s.last() == Some(&b'\n');
                if at_line_start
                    && let Some(i) = maybe_request_start.take()
                    && !requests[i..].starts_with(REQUEST)
                {
                    out.write_all(&requests[i..])?;
                    out.flush()?;
                    requests.truncate(i);
                }
            }
        }
        print!("\x1B[0m"); // clear styles
        let _ = std::io::stdout().flush();

        let status = cmd.wait()?;
        let _ = error_pipe.join();
        if status.success() {
            Ok(ToolOutput::from(String::from_utf8_lossy(&requests).as_ref()))
        } else {
            bail!("command failed, exit code {}", status.code().unwrap_or(0))
        }
    }
}

pub struct Tools {
    tools: Vec<Tool>,
    cache: PathBuf,
    on_final: Mutex<Vec<(usize, PathBuf, String)>>,
    about: About,
}
impl Tools {
    pub fn capture(local: &Path, cache: PathBuf, about: About, verbose: bool) -> anyhow::Result<Self> {
        let mut tools = vec![];
        visit_tools(local, |t| {
            if verbose {
                println!("found tool `{}` in `{}`", t.name, t.path.display())
            }
            tools.push(t);
            Ok(ControlFlow::Continue(()))
        })?;
        Ok(Self {
            tools,
            cache,
            on_final: Mutex::new(vec![]),
            about,
        })
    }

    pub fn run(&self, tool_name: &str, source: &Path, target: &Path, request: &Path) -> anyhow::Result<()> {
        println!("{}", display_path(request));
        for (i, tool) in self.tools.iter().enumerate() {
            if tool.name == tool_name {
                let output = tool.run(&self.cache, source, target, request, &self.about, None)?;
                for warn in output.warnings {
                    warn!("{warn}")
                }
                for args in output.on_final {
                    self.on_final.lock().push((i, request.to_owned(), args));
                }
                if !output.delegate {
                    return Ok(());
                }
            }
        }
        bail!("no tool `{tool_name}` to handle request")
    }

    pub fn run_final(self, source: &Path, target: &Path) -> anyhow::Result<()> {
        let on_final = self.on_final.into_inner();
        if !on_final.is_empty() {
            println!("--final--");
            for (i, request, args) in on_final {
                println!("{}", display_path(&request));
                let output = self.tools[i].run(&self.cache, source, target, &request, &self.about, Some(args))?;
                for warn in output.warnings {
                    warn!("{warn}")
                }
            }
        }
        Ok(())
    }
}

struct ToolOutput {
    // zng-res::delegate
    pub delegate: bool,
    // zng-res::warning=
    pub warnings: Vec<String>,
    // zng-res::on-final=
    pub on_final: Vec<String>,
}
impl From<&str> for ToolOutput {
    fn from(value: &str) -> Self {
        let mut out = Self {
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
