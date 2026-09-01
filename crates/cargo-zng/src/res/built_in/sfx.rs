use std::{
    fmt::Write as _,
    io::{Read, Seek},
};

use indexmap::IndexMap;
use lzma_rust2::filter::bcj::BcjWriter;
use serde::Deserialize;

use crate::util::unix_path;

use super::*;

const SFX_HELP: &str = r#"
Compile a self-extracting executable

The request file:
  source/sfx-package.zr-sfx
   | [sfx]
   | # executable to run, required
   | run = "target/release/run"
   |
   | # Embedded icon for the sfx executable (Windows only)
   | icon = "res/sfx.ico"
   |
   | # optional args for 'run'
   | args = ["--foo"]
   | # optional extra env for 'run'
   | env = {
   |     FOO = "bar",
   | }
   |
   | # rustc target triple, default is the host triple
   | # rustc-target = "x86_64-pc-windows-msvc"
   | # build a console exe on Windows, default is true (build a GUI exe)
   | windows-subsystem = false
   |
   | # compression to use for 'run', default is "zstd-bcj"
   | # compress = "none"
   |
   | # data the sfx can serve the 'run'
   | [[data]]
   | # name must be unique and not include ':' or '\n', default is "", for single data
   | name = "payload"
   | # compress data on build, default is "zstd"
   | compress = "zstd"
   | # file to include
   | file = "./data.tar"
   | 
   | [sign]
   | # optional, code sign the sfx exe
   | tool = "signtool sign /v /f $PFX /tr http://timestamp.sectigo.com /td SHA256 /fd SHA256 $SIGN_TARGET"
   | # only sign the sfx exe, default 'false' signs the 'run' exe too
   | # sfx-only = true

Compiles and signs a 'sfx-package.exe' with custom icon on Windows, or a 'sfx-package' on Unix.

Run:

When sfx runs it extracts the 'run' executable to a temp dir and runs it.

The optional 'env' variables override the system env. The SFX_ARGS var is always set. 

The SFX_ARGS is set to the sfx command line args, '\n' separated. The first arg is the path to the sfx exe.

On build, also searches for "$run.exe" if "$run" is not found and has no extension.

Data:

To read data the 'run' exe must spawn another instance of the sfx with the "SFX_GET_DATA" set
to the entry name. It will serve the data to stdout. The data may be decompressed on demand.

To get a list of data names and decompressed lengths run with "SFX_GET_MANIFEST", each stdout
line is <name>:<len>, <len> is an u64 or "unknown".

The `zng::setup::SfxClient` can also be used to connect and get data.

File Paths:

Paths are relative to the Cargo workspace root, you can also use .zr-rp to select files in the
resource target dir.

This request file:
  source/sfx-package.zr-sfxf.zr-rp
   | [[data]]
   | file = "${ZR_TARGET_DD}/res.txt"

Compiles a 'sfx-package' that includes the 'res.txt' copied to the target dir by `cargo zng res`.

Compress:

The sfx exe includes a zstd decompressor that is used to extract the 'run' exe.

The decompressor code can be used to read data too. The 'compress' field values are:

"none" — No compression on build. Data is served as is.
"zstd" — Compress on build unless file is already zstd (magic number check). Decompress on demand while reading.
"zstd-[filter]" — Transform data to improve compression, unless file is already zstd. Reverses
  transform on demand while reading.

Sfx is optimized for small number of large data entries. Use a container format to
package many small entries.

Filter:

Currently only BCJ (Branch/Call/Jump) filters are supported, identified by CPU instruction set:

"zstd-bcj-[set]" where [set] is: "x86", "arm", "arm64", "arm-thumb", "ppc", "sparc", "ia64", "riscv".
"zstd-bcj" — Select filter from 'rustc-target' arch, or zstd unfiltered for no matches.

The target file must be a binary (exe or lib) or a container (like tar) with only binary entries. The filters
are non-destructive but if the wrong filter is selected it will have negative impact on the compression level.

Signing:

Code signing must be applied to both the run exe and sfx exe, to facilitate this you can set the 'sign.tool'.

The sign-tool command will run twice, with $SIGN_TARGET set to "./run.exe" and "package.exe".

In the example above The $PFX var is an example of how to set the the private key. 
Keep the private key file outside the repository and set an env var to it. In CI use
secure variables.

Icon:

On Windows the sfx executable icon can be set with 'icon' field. Note that this requires the build
to run on a Windows machine with MSVC Toolkit installed. Cross-compilation from other systems will not work.

"#;
pub(super) fn sfx() {
    help(SFX_HELP);

    let request = std::fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    let request: Request = toml::from_str(&request).unwrap_or_else(|e| fatal!("{e}"));

    // make target-temp/src
    let target = path(ZR_TARGET);
    let tmp = target.with_file_name(format!("{}-temp", target.file_name().unwrap().display()));
    fs::create_dir(&tmp).unwrap_or_else(|e| fatal!("cannot create {}, {e}", tmp.display()));
    let src = tmp.join("src");
    fs::create_dir(&src).unwrap_or_else(|e| fatal!("cannot create src, {e}"));

    let mut icon = request.sfx.icon;
    if icon.is_some() && (!cfg!(windows) || !request.sfx.rustc_target.contains("windows")) {
        warn!("ignoring icon, can only build for Windows on Windows");
        icon = None;
    }

    // write target-temp/Cargo.toml
    let cargo = sfx_cargo(icon.is_some());
    let manifest = tmp.join("Cargo.toml");
    fs::write(&manifest, cargo.as_bytes()).unwrap_or_else(|e| fatal!("cannot create Cargo.toml, {e}"));

    // write target-temp/build.rs if needed
    if let Some(build_rs) = sfx_build(icon) {
        let build = tmp.join("build.rs");
        fs::write(&build, build_rs.as_bytes()).unwrap_or_else(|e| fatal!("cannot create build.rs, {e}"));
    }

    let mut run = request.sfx.run;
    if !run.exists() {
        if run.extension().is_none() && run.set_extension("exe") {
            if !run.exists() {
                run.set_extension("");
                fatal!(
                    "cannot find 'run' executable\n    {}\n    also tried {}.exe",
                    unix_path(&run),
                    run.file_name().unwrap().display()
                );
            }
        } else {
            fatal!("cannot find 'run' executable\n    {}", unix_path(&run));
        }
    }

    if let Some(tool) = &request.sign.tool
        && !request.sign.only_sfx
    {
        let sr = tmp.join("signed-run");
        fs::copy(&run, &sr).unwrap_or_else(|e| fatal!("cannot copy {}, {e}", run.display()));
        run = sr.clone();
        // SAFETY: tools run single threaded
        unsafe { std::env::set_var("SIGN_TARGET", &*unix_path(&sr)) };
        super::sh_run(tool.clone(), false, None).unwrap_or_else(|e| fatal!("cannot sign run, {e}"));
    }

    let mut data = vec![];
    let mut run_compression = parse_compress(&request.sfx.rustc_target, &request.sfx.compress);
    let run_parts = prepare_data(&tmp, 0, &mut run_compression, &run).unwrap_or_else(|e| fatal!("cannot compress run, {e}"));
    data.push((":run", run_compression, run_parts));
    for (id, d) in request.data.iter().enumerate() {
        if d.name.contains(':') || d.name.contains('\n') {
            fatal!("data name cannot contain ':' or '\n'");
        }
        let mut compression = parse_compress(&request.sfx.rustc_target, &d.compress);
        let file = d.file.as_path();

        let parts = prepare_data(&tmp, id + 1, &mut compression, file)
            .unwrap_or_else(|e| fatal!("cannot process file, {e}\n    file: {}", unix_path(file)));
        data.push((d.name.as_str(), compression, parts));
    }
    let sfx_main = sfx_main(request.sfx.windows_subsystem, &request.sfx.args, &request.sfx.env, &data);
    fs::write(src.join("main.rs"), sfx_main.as_bytes()).unwrap_or_else(|e| fatal!("cannot create main.rs, {e}"));

    let r = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg(&request.sfx.rustc_target)
        .arg("--manifest-path")
        .arg(manifest)
        .status()
        .unwrap_or_else(|e| fatal!("{e}"));
    assert!(r.success());

    let output = tmp.join(format!(
        "target/{}/release/zng-res-sfx{}",
        request.sfx.rustc_target,
        std::env::consts::EXE_SUFFIX
    ));
    if let Some(tool) = request.sign.tool {
        // SAFETY: tools run single threaded
        unsafe { std::env::set_var("SIGN_TARGET", &*unix_path(&output)) };
        super::sh_run(tool, false, None).unwrap_or_else(|e| fatal!("cannot sign sfx, {e}"));
    }

    let mut target = target;
    target.set_extension(output.extension().unwrap_or_default());
    fs::rename(output, target).unwrap_or_else(|e| fatal!("cannot finalize build, {e}"));

    fs::remove_dir_all(tmp).unwrap_or_else(|e| fatal!("cannot cleanup, {e}"));
}

fn parse_compress(rustc_target: &str, compress: &str) -> Compression {
    if let Some(filter) = compress.strip_prefix("zstd-") {
        match filter {
            "bcj-x86" => Compression::ZstdBcj(BcjFilter::X86),
            "bcj-arm" => Compression::ZstdBcj(BcjFilter::Arm),
            "bcj-arm64" => Compression::ZstdBcj(BcjFilter::Arm64),
            "bcj-arm-thumb" => Compression::ZstdBcj(BcjFilter::ArmThumb),
            "bcj-arm-ppc" => Compression::ZstdBcj(BcjFilter::Ppc),
            "bcj-arm-sparc" => Compression::ZstdBcj(BcjFilter::Sparc),
            "bcj-arm-ia64" => Compression::ZstdBcj(BcjFilter::Ia64),
            "bcj-arm-riscv" => Compression::ZstdBcj(BcjFilter::Riscv),
            "bcj" => bcj_from_triple(rustc_target).map(Compression::ZstdBcj).unwrap_or(Compression::Zstd),
            unk => fatal!("unknown filter {unk:?}"),
        }
    } else {
        match compress {
            "none" => Compression::None,
            "zstd" => Compression::Zstd,
            unk => fatal!("unknown compression {unk:?}"),
        }
    }
}

#[derive(Deserialize)]
struct Request {
    sfx: Sfx,
    #[serde(default)]
    data: Vec<Data>,
    #[serde(default = "default_sign")]
    sign: Sign,
}
fn default_sign() -> Sign {
    Sign {
        tool: None,
        only_sfx: false,
    }
}

#[derive(Deserialize)]
struct Sfx {
    run: PathBuf,
    icon: Option<PathBuf>,
    args: Vec<String>,
    env: indexmap::IndexMap<String, String>,
    #[serde(rename = "rustc-target")]
    #[serde(default = "rustc_host_triple")]
    rustc_target: String,
    #[serde(rename = "windows-subsystem")]
    #[serde(default = "default_windows_subsystem")]
    windows_subsystem: bool,
    #[serde(default = "default_run_compress")]
    compress: String,
}
fn default_windows_subsystem() -> bool {
    true
}
fn default_run_compress() -> String {
    "zstd-bcj".to_owned()
}

#[derive(Deserialize)]
struct Data {
    name: String,
    #[serde(default = "default_compress")]
    compress: String,
    file: PathBuf,
}
fn default_compress() -> String {
    "zstd".to_owned()
}

#[derive(Deserialize)]
struct Sign {
    tool: Option<String>,
    #[serde(default)]
    only_sfx: bool,
}

fn prepare_data(tmp: &Path, data_id: usize, decompress: &mut Compression, file: &Path) -> io::Result<Vec<PathBuf>> {
    let file_path = file;
    let mut file = fs::File::open(file)?;

    // 200MB
    //
    // Difficult to find exact limits for `include_bytes!`, Windows maybe has a 2GB limit per object.
    //
    // Static data is memory mapped to RAM on demand, so we depend on the system pagination to keep RAM
    // usage down. The split into 200MB parts here is to hopefully provide extra clear hinting that an
    // object is no longer needed as the sfx iterates over parts.
    const PART_MAX: u64 = 200u64 * 2u64.pow(20);

    let len = file.metadata()?.len();
    if len < 32 {
        // not worth compressing, zstd header alone is ~18 bytes
        *decompress = Compression::None;
    }

    let mut compress = *decompress;

    if matches!(compress, Compression::Zstd) {
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        file.seek(io::SeekFrom::Start(0))?;
        if magic == [0x28, 0xB5, 0x2F, 0xFD] {
            // already zstd
            compress = Compression::None;
        }
    }

    match compress {
        Compression::None => {
            println!("preparing {}", unix_path(file_path));

            if len <= PART_MAX {
                return Ok(vec![file_path.to_owned()]);
            }

            let mut len = len;

            let mut file = io::BufReader::new(file);
            let mut parts = vec![];
            loop {
                let part_path = tmp.join(format!("d{data_id}-p{}", parts.len()));
                let mut part = fs::File::create_new(&part_path)?;
                parts.push(part_path);
                if PART_MAX > len {
                    part.set_len(PART_MAX)?;
                    io::copy(&mut (&mut file).take(PART_MAX), &mut part)?;
                    len -= PART_MAX;
                } else {
                    part.set_len(len)?;
                    io::copy(&mut file, &mut part)?;
                    // len = 0;
                    break;
                }
            }
            Ok(parts)
        }
        Compression::Zstd => {
            println!("compressing {}", unix_path(file_path));

            // 19 is the maximum non-ultra compression
            // zstd creates an optimal BufReader
            let mut file = zstd::stream::read::Encoder::new(file, 19)?;

            file.set_pledged_src_size(Some(len))?;
            file.include_contentsize(true)?;

            let mut parts = vec![];
            loop {
                let part_path = tmp.join(format!("d{data_id}-p{}", parts.len()));
                let mut part = fs::File::create_new(&part_path)?;
                parts.push(part_path);

                let part_len = io::copy(&mut (&mut file).take(PART_MAX), &mut part)?;
                if part_len < PART_MAX {
                    break;
                }
            }

            Ok(parts)
        }
        Compression::ZstdBcj(bcj) => {
            println!("compressing {}", unix_path(file_path));

            // There is no pull-based bcj encoder so the parts swap needs to happen inside
            struct PartsWriter<'a> {
                tmp: &'a Path,
                data_id: usize,
                parts: &'a mut Vec<PathBuf>,
                part: Option<fs::File>,
                left: usize,
            }
            impl<'a> PartsWriter<'a> {
                fn next_part(&mut self) -> io::Result<()> {
                    let part_path = self.tmp.join(format!("d{}-p{}", self.data_id, self.parts.len()));
                    self.part = Some(fs::File::create_new(&part_path)?);
                    self.parts.push(part_path);
                    self.left = PART_MAX as usize;
                    Ok(())
                }
            }
            impl<'a> io::Write for PartsWriter<'a> {
                fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                    if buf.is_empty() {
                        return Ok(0);
                    }
                    if self.left == 0 {
                        self.next_part()?;
                    }
                    if let Some(f) = &mut self.part {
                        let max_len = self.left.min(buf.len());
                        let buf = &buf[..max_len];
                        let written = f.write(buf)?;
                        self.left -= written;
                        Ok(written)
                    } else {
                        Ok(0)
                    }
                }

                fn flush(&mut self) -> io::Result<()> {
                    if let Some(f) = &mut self.part { f.flush() } else { Ok(()) }
                }
            }

            let mut parts = vec![];
            let out = PartsWriter {
                tmp,
                data_id,
                parts: &mut parts,
                part: None,
                left: 0,
            };
            // 22 is the maximum ultra compression (high CPU and RAM usage)
            let mut encoder = zstd::stream::write::Encoder::new(out, 22)?;
            encoder.set_pledged_src_size(Some(len))?;
            encoder.include_contentsize(true)?;

            let out = &mut encoder;
            let mut bcj_filter = match bcj {
                BcjFilter::X86 => BcjWriter::new_x86(out, 0),
                BcjFilter::Arm => BcjWriter::new_arm(out, 0),
                BcjFilter::Arm64 => BcjWriter::new_arm64(out, 0),
                BcjFilter::ArmThumb => BcjWriter::new_arm_thumb(out, 0),
                BcjFilter::Ppc => BcjWriter::new_ppc(out, 0),
                BcjFilter::Sparc => BcjWriter::new_sparc(out, 0),
                BcjFilter::Ia64 => BcjWriter::new_ia64(out, 0),
                BcjFilter::Riscv => BcjWriter::new_riscv(out, 0),
            };

            let mut file = file;
            io::copy(&mut file, &mut bcj_filter)?;
            bcj_filter.finish()?;
            encoder.finish()?;

            Ok(parts)
        }
    }
}

fn bcj_from_triple(target: &str) -> Option<BcjFilter> {
    let arch = target.split('-').next()?;

    match arch {
        "i386" | "i486" | "i586" | "i686" | "x86_64" => Some(BcjFilter::X86),
        "arm" | "armv4t" | "armv5te" | "armv7" | "armv7a" | "armv7r" | "armv7s" | "armebv7r" => Some(BcjFilter::Arm),
        "thumbv6m" | "thumbv7m" | "thumbv7em" | "thumbv7neon" | "thumbv8m.base" | "thumbv8m.main" => Some(BcjFilter::ArmThumb),
        "aarch64" | "aarch64_be" => Some(BcjFilter::Arm64),
        "powerpc" | "powerpc64" | "powerpc64le" => Some(BcjFilter::Ppc),
        "sparc" | "sparcv9" | "sparc64" => Some(BcjFilter::Sparc),
        "ia64" => Some(BcjFilter::Ia64),
        "riscv32" | "riscv64" => Some(BcjFilter::Riscv),

        _ => None,
    }
}

// enable rust-analyzer
#[path = "sfx_res/sfx_main.rs"]
#[allow(unused)]
mod sfx_main;
use sfx_main::{BcjFilter, Compression};

fn sfx_main(
    windows_subsystem: bool,
    args: &[String],
    env: &IndexMap<String, String>,
    data: &[(&str, Compression, Vec<PathBuf>)],
) -> String {
    let main = include_str!("sfx_res/sfx_main.rs");

    const DATA: &str = "static DATA: &[(&str, Compression, &[&[u8]])] = &[";
    let mut out_data = String::new();
    if windows_subsystem {
        out_data.push_str("#![windows_subsystem = \"windows\"]\n");
    }
    out_data.push_str(DATA);
    out_data.push('\n');
    for (name, compression, parts) in data {
        let (compression, filter) = match compression {
            Compression::None => ("None", ""),
            Compression::Zstd => ("Zstd", ""),
            Compression::ZstdBcj(f) => {
                let f = match f {
                    BcjFilter::X86 => "(BcjFilter::X86)",
                    BcjFilter::Arm => "(BcjFilter::Arm)",
                    BcjFilter::Arm64 => "(BcjFilter::Arm64)",
                    BcjFilter::ArmThumb => "(BcjFilter::ArmThumb)",
                    BcjFilter::Ppc => "(BcjFilter::Ppc)",
                    BcjFilter::Sparc => "(BcjFilter::Sparc)",
                    BcjFilter::Ia64 => "(BcjFilter::Ia64)",
                    BcjFilter::Riscv => "(BcjFilter::Riscv)",
                };
                ("ZstdBcj", f)
            }
        };
        write!(&mut out_data, "  ({name:?}, Compression::{compression}{filter}, &[").unwrap();
        for part in parts {
            write!(&mut out_data, "include_bytes!(\"{}\"), ", unix_path(part)).unwrap();
        }
        writeln!(&mut out_data, "]),").unwrap();
    }
    out_data.push_str("\n];");

    const ARGS: &str = "static ARGS: &[&str] = &[";
    let mut out_args = ARGS.to_owned();
    for arg in args {
        write!(&mut out_args, "{arg:?}, ").unwrap();
    }
    out_args.push_str("];");

    const ENV: &str = "static ENV: &[(&str, &str)] = &[";
    let mut out_env = ENV.to_owned();
    out_env.push('\n');
    for (key, value) in env {
        writeln!(&mut out_env, "  ({key:?}, {value:?}),").unwrap();
    }
    out_env.push_str("\n];");

    let mut replaces = vec![(DATA, out_data), (ARGS, out_args), (ENV, out_env)];

    let mut out_main = String::new();
    let mut region = "";
    for line in main.lines() {
        // conditional regions
        if let Some(r) = line.trim_start().strip_prefix("// </")
            && let Some(r) = r.strip_suffix('>')
        {
            assert_eq!(region, r);
            region = "";
            continue;
        }
        if let Some(r) = line.trim_start().strip_prefix("// <")
            && let Some(r) = r.strip_suffix('>')
        {
            region = r;
            continue;
        }
        let keep_line = match region {
            "windows-subsystem" => windows_subsystem,
            "" => true,
            unk => panic!("unknown region {unk:?}"),
        };
        if !keep_line {
            continue;
        }

        if let Some(i) = replaces.iter().position(|(k, _)| line.starts_with(k)) {
            let (_, value) = replaces.swap_remove(i);
            out_main.push_str(&value);
        } else {
            out_main.push_str(line);
        }

        if replaces.is_empty() {
            let i = line.as_ptr() as usize - main.as_ptr() as usize + line.len();
            out_main.push_str(&main[i..]);
            break;
        } else {
            out_main.push('\n');
        }
    }

    out_main
}

fn sfx_build(icon: Option<PathBuf>) -> Option<String> {
    let ico = icon?;
    let build = include_str!("sfx_res/build_icon.rs");
    let build = build.replace("{icon-path}", &ico.display().to_string());
    Some(build)
}

fn sfx_cargo(has_icon: bool) -> String {
    let cargo = include_str!("sfx_res/sfx_cargo.toml");
    if has_icon {
        cargo.to_owned()
    } else {
        let (before, after) = cargo.split_once("# <has-icon>\n").unwrap();
        let (_, after) = after.split_once("\n# </has-icon>\n").unwrap();
        format!("{before}{after}")
    }
}

fn rustc_host_triple() -> String {
    let o = std::process::Command::new("rustc")
        .arg("--version")
        .arg("--verbose")
        .output()
        .unwrap_or_else(|e| fatal!("cannot find host triple, {e}"));

    if !o.status.success() {
        fatal!("cannot find host triple, exit code: {:?}", o.status.code().unwrap_or(0))
    }

    let stdout = str::from_utf8(&o.stdout).unwrap_or_else(|e| fatal!("cannot find host triple, {e}"));

    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return triple.to_owned();
        }
    }

    fatal!("cannot find host triple")
}
