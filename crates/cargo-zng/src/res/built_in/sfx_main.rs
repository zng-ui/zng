// source code for the generated self-extracting executable compiled by ./sfx.rs

use std::{
    env, fs, io::{self, Read, Seek}, path::PathBuf,
};

pub fn main() {
    let sfx_args: Vec<_> = env::args().collect();

    let payload = Payload::find().unwrap();

    let sfx_data = format!("{}:{}", payload.data_start, payload.data_len);

    let run_tmp = payload.extract_run().unwrap();
}

struct Payload {
    data_start: u64,
    data_len: u64,

    run: io::Take<io::BufReader<fs::File>>,
}
impl Payload {
    fn find() -> io::Result<Payload> {
        // payload is appended to the end of the compiled sfx executable
        // 
        // [run..][data..][run_len:u64][data_len:u64][b"zng-sfx"]
        //
        // lengths are encoded in little-endian

        let sfx = env::current_exe()?;
        let sfx = fs::File::open(sfx)?;
        let mut sfx = io::BufReader::new(sfx);
        
        // Windows can append data after the marker (code signature)
        let i = rfind(&mut sfx, b"zng-sfx")?.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing sfx header"))?;

        // read [run_len][data_len]
        sfx.seek_relative(-(i as i64 + 16))?;
        let mut u64_buf = [0u8; 8];
        sfx.read_exact(&mut u64_buf)?;
        let run_len = u64::from_le_bytes(u64_buf);
        sfx.read_exact(&mut u64_buf)?;
        let data_len = u64::from_le_bytes(u64_buf);

        let data_end = sfx.stream_position()? - 16;
        let data_start = data_end - data_len;
        let run_start = data_start - run_len;

        sfx.seek(io::SeekFrom::Start(run_start))?;
        
        Ok(Self {
            data_start,
            data_len,
            run: sfx.take(run_len),
        })
    }

    fn extract_run(mut self) -> io::Result<PathBuf> {
        let tmp = tmp_file()?;
        let mut tmp_file = fs::File::create_new(&tmp)?;
        // !!: TODO extract
        io::copy(&mut self.run, &mut tmp_file)?;

        Ok(tmp)
    }
}
fn rfind<R: Read + Seek>(reader: &mut R, marker: &[u8]) -> io::Result<Option<u64>> {
    const CHUNK_SIZE: usize = 64 * 1024;

    let file_len = reader.seek(io::SeekFrom::End(0))?;

    let mut overlap = Vec::new();
    let mut pos = file_len;

    while pos > 0 {
        let chunk_size = (pos as usize).min(CHUNK_SIZE);
        pos -= chunk_size as u64;

        reader.seek(io::SeekFrom::Start(pos))?;

        let mut chunk = vec![0u8; chunk_size];
        reader.read_exact(&mut chunk)?;

        chunk.extend_from_slice(&overlap);

        if let Some(i) = chunk.windows(marker.len()).rposition(|w| w == marker) {
            return Ok(Some(pos + i as u64));
        }

        overlap.clear();
        overlap.extend_from_slice(&chunk[..marker.len().saturating_sub(1).min(chunk.len())]);
    }

    Ok(None)
}

fn tmp_file() -> io::Result<PathBuf> {
    let tmp = env::temp_dir().join("zng-sfx");
    fs::create_dir(&tmp)?;
    for i in 0..1000 {
        let tmp = tmp.join(format!("run-{i}.exe"));
        if let Err(e) = fs::remove_file(&tmp) && matches!(e.kind(), io::ErrorKind::NotFound) {
            return Ok(tmp)
        }
    }
    Err(io::Error::new(io::ErrorKind::QuotaExceeded, "too many tmp exe"))
}