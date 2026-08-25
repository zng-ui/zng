use std::{collections::HashSet, io::Write, rc::Rc};

use lzma_rust2::filter::bcj::BcjWriter;
use serde::Deserialize;

use super::*;

const TAR_HELP: &str = r#"
Pack files and dirs into a TAR container with optional compression

The request file:
  source/data.tar.zst.zr-tar
   | [[entry]]
   | path = "res/bin/*"
   | name = "bin/*"
   |
   | [[entry]]
   | path = "README.md"
   | name = "docs/README.md"
   |
   | # Optional compression
   | [zstd]
   | level = 19

Packs entries into a TAR, compresses it with ZSTD.

The syntax is a TOML file, the tables are:

[[entry]] — Array of entries to pack.
path — Path or glob pattern, relative to the workspace root. Required.
name — Optional name of the file on the TAR container. Optional.

Each entry can be a file, directory or glob selection. Only file and directory 
entries are supported by this tool, other tar archive entries are not supported.

If 'path' matches a directory all contained files and sub directories are packed. 

If 'name' is not set it is path relative to workspace root. 

If 'name' is set it must not contain "/..".

If 'name' ends with "/*" the selected files and dirs are named in this TAR dir.

All 'name' paths are relative to the TAR root, "/foo" is packed as "foo".

[filter] — Optional lossless transform to apply to the TAR
bcj — Branch/Call/Jump filter that optimizes compression of binary code files.
    Values: "x86", "arm", "arm64", "arm-thumb", "ppc", "sparc", "ia64", "riscv"
    Note that decompressor must revert the filter.

The decompressor must undo these changes before reading the TAR. The .zr-sfx supports decoding BCJ. 

[zstd] — Optional ZStandard compression
level — Compression level, -131072..=22, 0 means no compression, default is 19.

Compress the TAR, after [filter] is applied, using ZStandard.

[gzip] — Optional GZip compression
level — Compression level, 0..=9, 0 means no compression, default is 8.

"#;
pub(super) fn tar() {
    help(TAR_HELP);

    let request = std::fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    let request: Request = toml::from_str(&request).unwrap_or_else(|e| fatal!("{e}"));

    let target = path(ZR_TARGET);

    let target = fs::File::create(&target).unwrap_or_else(|e| fatal!("{e}"));
    let mut target: Box<dyn WriteFinish> = Box::new(target);

    if let Some(zstd) = request.zstd
        && zstd.level != 0
    {
        let range = zstd::compression_level_range();
        let level = zstd.level.clamp(*range.start(), *range.end());
        let zstd = zstd::Encoder::new(target, level).unwrap_or_else(|e| fatal!("{e}"));
        target = Box::new(zstd);
    } else if let Some(gzip) = request.gzip
        && gzip.level != 0
    {
        let level = flate2::Compression::new(gzip.level.clamp(0, 9));
        let gzip = flate2::write::GzEncoder::new(target, level);
        target = Box::new(gzip);
    }

    if !request.filter.bcj.is_empty() {
        let bcj = match request.filter.bcj.as_str() {
            "x86" => BcjWriter::new_x86(target, 0),
            "arm" => BcjWriter::new_arm(target, 0),
            "arm64" => BcjWriter::new_arm64(target, 0),
            "arm-thumb" => BcjWriter::new_arm_thumb(target, 0),
            "ppc" => BcjWriter::new_ppc(target, 0),
            "sparc" => BcjWriter::new_sparc(target, 0),
            "ia64" => BcjWriter::new_ia64(target, 0),
            "riscv" => BcjWriter::new_riscv(target, 0),
            _ => fatal!("unknown bcj filter"),
        };
        target = Box::new(bcj)
    }

    let mut target = ::tar::Builder::new(target);
    // all symlinks are skipped anyway, this is just an optimization
    // avoids some IO in the builder
    target.follow_symlinks(false);

    let mut names = HashSet::new();
    for entry in request.entries {
        println!("{}", entry.path);

        let mut name = entry.name.as_str();
        if name.contains("/..") {
            error!("name cannot contain /..");
            continue;
        }
        if name.contains('\\') {
            error!("name must use / slashes");
            continue;
        }
        let name_is_prefix = name.ends_with("/*");
        if name_is_prefix {
            name = &name[..name.len() - "/*".len()];
        }
        if name.starts_with('/') {
            name = &name["/".len()..];
        }
        if name.contains('*') {
            error!("name can only contain * in suffix /*");
            continue;
        }

        let mut any = false;
        for glob_entry in ::glob::glob(&entry.path).unwrap_or_else(|e| fatal!("{e}")) {
            let glob_entry = glob_entry.unwrap_or_else(|e| fatal!("{e}"));
            let entry_name = if let Some(n) = glob_entry.file_name()
                && let Some(n) = n.to_str()
            {
                n
            } else {
                continue;
            };
            if glob_entry.is_file() {
                let name = if name_is_prefix {
                    format!("{name}/{entry_name}")
                } else {
                    if name.ends_with('/') {
                        error!("matched file, but name ends with /");
                        continue;
                    }
                    name.to_owned()
                };
                let name = Rc::new(name);
                println!("   {name}");

                if !names.insert(name.clone()) {
                    warn!("name already defined, entry overwritten");
                }

                target
                    .append_path_with_name(glob_entry, name.as_str())
                    .unwrap_or_else(|e| fatal!("{e}"));
            } else if glob_entry.is_dir() {
                let dir_parent = glob_entry.parent().unwrap_or_else(|| Path::new(""));
                for dir_entry in walkdir::WalkDir::new(&glob_entry).follow_links(false) {
                    let dir_entry = dir_entry.unwrap_or_else(|e| fatal!("{e}"));
                    let dir_entry = dir_entry.path();
                    if dir_entry.is_file() || dir_entry.is_dir() {
                        let name = Path::new(&name)
                            .join(dir_entry.strip_prefix(dir_parent).unwrap())
                            .display()
                            .to_string()
                            .replace('\\', "/")
                            .to_owned();
                        let name = Rc::new(name);
                        println!("   {name}");
                        // dirs only create a dir entry, see docs
                        target
                            .append_path_with_name(dir_entry, name.as_str())
                            .unwrap_or_else(|e| fatal!("{e}"));

                        if !names.insert(name.clone()) {
                            warn!("name already defined, entry overwritten");
                        }
                    }
                }
            } else {
                continue;
            }
            any = true;
        }
        if !any {
            if entry.path.contains('*') {
                warn!("no matches")
            } else {
                error!("no matches")
            }
        }
    }

    target.finish().unwrap_or_else(|e| fatal!("{e}"));
    target.into_inner().unwrap().finish().unwrap_or_else(|e| fatal!("{e}"));
}

#[derive(Deserialize)]
struct Request {
    #[serde(default, rename = "entry")]
    entries: Vec<Entry>,
    #[serde(default)]
    filter: Filter,
    #[serde(default)]
    zstd: Option<ZStd>,
    #[serde(default)]
    gzip: Option<GZip>,
}
#[derive(Deserialize)]
struct Entry {
    path: String,
    #[serde(default)]
    name: String,
}
#[derive(Deserialize, Default)]
struct Filter {
    #[serde(default)]
    bcj: String,
}
#[derive(Deserialize, Default)]
struct ZStd {
    #[serde(default = "default_zstd_level")]
    level: i32,
}
fn default_zstd_level() -> i32 {
    19
}
#[derive(Deserialize, Default)]
struct GZip {
    #[serde(default = "default_gzip_level")]
    level: u32,
}
fn default_gzip_level() -> u32 {
    7
}

/// BcjWriter and zstd::Encoder need to write a "footer" after data write
trait WriteFinish: io::Write {
    fn finish(self: Box<Self>) -> io::Result<()>;
}
impl WriteFinish for fs::File {
    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.flush()
    }
}
impl WriteFinish for BcjWriter<Box<dyn WriteFinish>> {
    fn finish(self: Box<Self>) -> io::Result<()> {
        (*self).finish()?.finish()
    }
}
impl WriteFinish for zstd::Encoder<'static, Box<dyn WriteFinish>> {
    fn finish(self: Box<Self>) -> io::Result<()> {
        (*self).finish()?.finish()
    }
}
impl WriteFinish for flate2::write::GzEncoder<Box<dyn WriteFinish>> {
    fn finish(self: Box<Self>) -> io::Result<()> {
        (*self).finish()?.finish()
    }
}
