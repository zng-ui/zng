// <windows-subsystem>
#![windows_subsystem = "windows"]
// </windows-subsystem>

// source code for the generated self-extracting executable compiled by ./sfx.rs

// [(name, compression, parts)]
// data split in parts for performance and to avoid hard limits
static DATA: &[(&str, Compression, &[&[u8]])] = &[];

static ARGS: &[&str] = &[];
static ENV: &[(&str, &str)] = &[];

#[derive(Clone, Copy)]
pub enum Compression {
    None,
    Zstd,
    ZstdBcj(BcjFilter),
}

#[derive(Clone, Copy)]
pub enum BcjFilter {
    X86,
    Arm,
    Arm64,
    ArmThumb,
    Ppc,
    Sparc,
    Ia64,
    Riscv,
}

use std::{
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _},
    path::PathBuf,
};

use lzma_rust2::filter::bcj::BcjReader;

pub fn main() {
    // <windows-subsystem>
    #[cfg(windows)]
    attach_console();
    // </windows-subsystem>

    if let Ok(name) = std::env::var("SFX_GET_DATA") {
        return serve_data(&name);
    }
    run();
}

macro_rules! err_exit {
    ($($msg:tt)*) => {
        {
            eprintln!($($msg)*);
            std::process::exit(-1);
        }
    };
}
trait UnwrapOrExit<T> {
    fn unwrap_or_exit(self, ctx: &str) -> T;
}
impl<T, E: std::error::Error> UnwrapOrExit<T> for Result<T, E> {
    fn unwrap_or_exit(self, ctx: &str) -> T {
        match self {
            Ok(r) => r,
            Err(e) => err_exit!("{ctx} error, {e}"),
        }
    }
}

fn read_data(name: &str) -> Box<dyn io::Read> {
    for (key, compression, parts) in DATA {
        if *key == name {
            if parts.is_empty() {
                return Box::new(&[0u8; 0][..]);
            }
            let mut data: Box<dyn io::Read> = Box::new(parts[0]);
            for &part in parts[1..].iter() {
                data = Box::new(data.chain(part));
            }
            match *compression {
                Compression::None => {}
                Compression::Zstd => {
                    let d = zstd::stream::read::Decoder::new(data).unwrap_or_exit("Decoder::new");
                    data = Box::new(d);
                }
                Compression::ZstdBcj(bcj) => {
                    let d = zstd::stream::read::Decoder::new(data).unwrap_or_exit("Decoder::new");
                    let d = match bcj {
                        BcjFilter::X86 => BcjReader::new_x86(d, 0),
                        BcjFilter::Arm => BcjReader::new_arm(d, 0),
                        BcjFilter::Arm64 => BcjReader::new_arm64(d, 0),
                        BcjFilter::ArmThumb => BcjReader::new_arm_thumb(d, 0),
                        BcjFilter::Ppc => BcjReader::new_ppc(d, 0),
                        BcjFilter::Sparc => BcjReader::new_sparc(d, 0),
                        BcjFilter::Ia64 => BcjReader::new_ia64(d, 0),
                        BcjFilter::Riscv => BcjReader::new_riscv(d, 0),
                    };
                    data = Box::new(d);
                }
            }
            return data;
        }
    }
    err_exit!("{name:?} not found")
}

fn serve_data(name: &str) {
    let mut data = read_data(name);
    io::copy(&mut data, &mut std::io::stdout()).unwrap_or_exit("serve_data/copy");
}

fn run() {
    let run_file = tmp_run_file().unwrap_or_exit("tmp_run_file");
    {
        let mut data = read_data(":run");
        let mut run_file = fs::File::create_new(&run_file).unwrap_or_exit("File::create_new");
        io::copy(&mut data, &mut run_file).unwrap_or_exit("run/copy");
    }

    let mut sfx_args = String::new();
    let mut sep = "";
    for arg in env::args() {
        write!(&mut sfx_args, "{sep}{arg}").unwrap_or_exit("");
        sep = "\n";
    }

    let mut run = std::process::Command::new(&run_file);
    for arg in ARGS {
        run.arg(arg);
    }
    for (key, value) in ENV {
        run.env(key, value);
    }
    let s = run.env("SFX_ARGS", sfx_args).status().unwrap_or_exit("run");

    if let Err(e) = fs::remove_file(run_file)
        && !matches!(e.kind(), io::ErrorKind::NotFound)
    {
        err_exit!("run/remove error, {e:?}");
    }
    if !s.success() {
        std::process::exit(s.code().unwrap_or(-1))
    }
}
fn tmp_run_file() -> io::Result<PathBuf> {
    let tmp = env::temp_dir().join("zng-sfx");
    if let Err(e) = fs::create_dir(&tmp)
        && !matches!(e.kind(), io::ErrorKind::AlreadyExists)
    {
        return Err(e);
    }
    for i in 0..1000 {
        let tmp = tmp.join(format!("run-{i}.exe"));
        if let Err(e) = fs::remove_file(&tmp)
            && matches!(e.kind(), io::ErrorKind::NotFound)
        {
            return Ok(tmp);
        }
    }
    Err(io::Error::new(io::ErrorKind::QuotaExceeded, "too many tmp exe"))
}

// <windows-subsystem>
#[cfg(windows)]
pub fn attach_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleWindow() -> isize;
        fn AttachConsole(process_id: u32) -> i32;
    }
    unsafe {
        // If no console is attached, attempt to attach to parent
        if GetConsoleWindow() == 0 {
            let _ = AttachConsole(0xFFFFFFFF);
        }
    }
}
// </windows-subsystem>
