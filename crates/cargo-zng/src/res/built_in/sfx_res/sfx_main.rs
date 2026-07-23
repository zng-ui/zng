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
    if let Ok(name) = std::env::var("SFX_GET_DATA") {
        return serve_data(&name);
    }
    run();
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
                    let d = zstd::stream::read::Decoder::new(data).unwrap();
                    data = Box::new(d);
                }
                Compression::ZstdBcj(bcj) => {
                    let d = zstd::stream::read::Decoder::new(data).unwrap();
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
    panic!("{name:?} not found")
}

fn serve_data(name: &str) {
    let mut data = read_data(name);
    io::copy(&mut data, &mut std::io::stdout()).unwrap();
}

fn run() {
    let run_file = tmp_run_file().unwrap();
    {
        let mut data = read_data(":run");
        let mut run_file = fs::File::create_new(&run_file).unwrap();
        io::copy(&mut data, &mut run_file).unwrap();
    }

    let mut sfx_args = String::new();
    let mut sep = "";
    for arg in env::args() {
        write!(&mut sfx_args, "{sep}{arg}").unwrap();
        sep = "\n";
    }

    let mut run = std::process::Command::new(&run_file);
    for arg in ARGS {
        run.arg(arg);
    }
    for (key, value) in ENV {
        run.env(key, value);
    }
    let s = run.env("SFX_ARGS", sfx_args).status().unwrap();

    if let Err(e) = fs::remove_file(run_file)
        && !matches!(e.kind(), io::ErrorKind::NotFound)
    {
        panic!("{e}");
    }
    if !s.success() {
        std::process::exit(s.code().unwrap_or(-1))
    }
}
fn tmp_run_file() -> io::Result<PathBuf> {
    let tmp = env::temp_dir().join("zng-sfx");
    fs::create_dir(&tmp)?;
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
