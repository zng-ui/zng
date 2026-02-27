use std::{
    collections::{HashMap, HashSet},
    io::{Read as _, Write},
    path::PathBuf,
    time::SystemTime,
};

use clap::*;
use serde::Deserialize as _;

#[derive(Args, Debug, Default)]
pub struct TraceArgs {
    /// Path or command to run the Zng executable
    ///
    /// Example: `cargo zng trace "./some/exe"` or `cargo zng trace -- cargo run exe`
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,

    /// env_logger style filter
    #[arg(long, short, default_value = "trace")]
    filter: String,

    /// Output JSON file
    ///
    /// {timestamp} and {ts} is replaced with a timestamp in microseconds from Unix epoch
    #[arg(long, short, default_value = "./trace-{timestamp}.json")]
    output: String,
}

pub fn run(args: TraceArgs) {
    let mut cmd = {
        let mut cmd = args.command.into_iter().peekable();
        if let Some(c) = cmd.peek()
            && c == "--"
        {
            cmd.next();
        }
        if let Some(c) = cmd.next() {
            let mut o = std::process::Command::new(c);
            o.args(cmd);
            o
        } else {
            fatal!("COMMAND is required")
        }
    };

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
        .to_string();

    let tmp = std::env::temp_dir().join("cargo-zng-trace");
    if let Err(e) = std::fs::create_dir_all(&tmp) {
        fatal!("cannot create temp dir, {e}");
    }
    let out_dir = tmp.join(&ts);
    let _ = std::fs::remove_dir_all(&out_dir);

    let out_file = PathBuf::from(args.output.replace("{timestamp}", &ts).replace("{ts}", &ts));
    if let Some(p) = out_file.parent()
        && let Err(e) = std::fs::create_dir_all(p)
    {
        fatal!("cannot output to {}, {e}", out_file.display());
    }
    let mut out = match std::fs::File::create(&out_file) {
        Ok(f) => f,
        Err(e) => fatal!("cannot output to {}, {e}", out_file.display()),
    };

    cmd.env("ZNG_RECORD_TRACE", "")
        .env("ZNG_RECORD_TRACE_DIR", &tmp)
        .env("ZNG_RECORD_TRACE_FILTER", args.filter)
        .env("ZNG_RECORD_TRACE_TIMESTAMP", &ts);

    let mut cmd = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => fatal!("cannot run, {e}"),
    };

    let code = match cmd.wait() {
        Ok(s) => s.code().unwrap_or(0),
        Err(e) => {
            error!("cannot wait command exit, {e}");
            101
        }
    };

    if !out_dir.exists() {
        fatal!("run did not save any trace\nnote: the feature \"trace_recorder\" must be enabled during build")
    }

    println!("merging trace files...");

    out.write_all(b"[\n")
        .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));
    let mut separator = "";

    for trace in glob::glob(out_dir.join("*.json").display().to_string().as_str())
        .ok()
        .into_iter()
        .flatten()
    {
        let trace = match trace {
            Ok(t) => t,
            Err(e) => {
                error!("error globing trace files, {e}");
                continue;
            }
        };
        let json = match std::fs::read_to_string(&trace) {
            Ok(s) => s,
            Err(e) => {
                error!("cannot read {}, {e}", trace.display());
                continue;
            }
        };

        let name_sys_pid = trace
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .strip_suffix(".json")
            .unwrap_or_default()
            .to_owned();
        let name_sys_pid = match name_sys_pid.parse::<u64>() {
            Ok(i) => i,
            Err(_) => {
                error!("expected only {{pid}}.json files");
                continue;
            }
        };

        // skip the array opening
        let json = json.trim_start();
        if !json.starts_with('[') {
            error!("unknown format in {}", trace.display());
            continue;
        }
        let json = &json[1..];

        let mut thread_ids = HashSet::new();
        // peek thread ids
        {
            let mut search = json;
            let prefix = r#""tid":"#;
            while let Some(i) = search.find(prefix) {
                search = &search[i + prefix.len()..];
                if let Some(i) = search.find(',') {
                    let tid = &search[..i];
                    if let Ok(tid) = tid.parse::<u64>() {
                        thread_ids.insert(tid);
                    }
                    search = &search[i + 1..];
                }
            }
        }
        let mut custom_thread_id = 9000u64;
        let mut custom_threads = HashMap::new();

        let mut reader = std::io::Cursor::new(json.as_bytes());
        loop {
            // skip white space and commas to the next object
            let mut pos = reader.position();
            let mut buf = [0u8];
            while reader.read(&mut buf).is_ok() {
                if !b" \r\n\t,".contains(&buf[0]) {
                    break;
                }
                pos = reader.position();
            }
            reader.set_position(pos);
            let mut de = serde_json::Deserializer::from_reader(&mut reader);
            match serde_json::Value::deserialize(&mut de) {
                Ok(mut entry) => {
                    // patch "pid" to be unique
                    if let Some(serde_json::Value::Number(pid)) = entry.get_mut("pid") {
                        if pid.as_u64() != Some(1) {
                            error!("expected only pid:1 in trace file");
                            continue;
                        }
                        *pid = serde_json::Number::from(name_sys_pid);
                    }

                    // convert custom entries to actual trace format
                    match &mut entry {
                        serde_json::Value::Object(entry) => {
                            enum Phase {
                                Event,
                                Begin,
                                End,
                                Other,
                            }
                            let mut ph = Phase::Other;
                            if let Some(serde_json::Value::String(p)) = entry.get("ph") {
                                match p.as_str() {
                                    "i" => ph = Phase::Event,
                                    "B" => ph = Phase::Begin,
                                    "E" => ph = Phase::End,
                                    _ => {}
                                }
                            }
                            if let Some(serde_json::Value::Object(args)) = entry.get_mut("args") {
                                match ph {
                                    Phase::Event => {
                                        // convert the INFO message process name to actual "process_name" metadata
                                        if let Some(serde_json::Value::String(msg)) = args.get("message")
                                            && let Some(rest) = msg.strip_prefix("pid: ")
                                            && let Some((sys_pid, p_name)) = rest.split_once(", name: ")
                                            && let Ok(sys_pid) = sys_pid.parse::<u64>()
                                            && name_sys_pid == sys_pid
                                        {
                                            let args = format_args!(
                                                r#"{separator}{{"ph":"M","pid":{sys_pid},"name":"process_name","args":{{"name":"{p_name}"}}}}"#,
                                            );
                                            out.write_fmt(args)
                                                .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));
                                        } else if let Some(serde_json::Value::String(custom_thread)) = args.remove("thread")
                                            && let Some(serde_json::Value::Number(n)) = entry.get_mut("tid")
                                        {
                                            let tid = match custom_threads.entry(custom_thread) {
                                                std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                                                std::collections::hash_map::Entry::Vacant(e) => {
                                                    let name = serde_json::Value::String(e.key().clone());

                                                    while !thread_ids.insert(custom_thread_id) {
                                                        custom_thread_id = custom_thread_id.wrapping_add(1);
                                                    }
                                                    e.insert(custom_thread_id);

                                                    let args = format_args!(
                                                        r#"{separator}{{"ph":"M","pid":{name_sys_pid},"tid":{custom_thread_id},"name":"thread_name","args":{{"name":{name}}}}}"#,
                                                    );
                                                    out.write_fmt(args)
                                                        .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));

                                                    custom_thread_id
                                                }
                                            };
                                            *n = tid.into();
                                        }
                                    }
                                    Phase::Begin | Phase::End => {
                                        // convert "thread" arg to actual tid
                                        if let Some(serde_json::Value::String(custom_thread)) = args.remove("thread")
                                            && let Some(serde_json::Value::Number(n)) = entry.get_mut("tid")
                                        {
                                            let tid = match custom_threads.entry(custom_thread.clone()) {
                                                std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                                                std::collections::hash_map::Entry::Vacant(e) => {
                                                    let name = serde_json::Value::String(e.key().clone());

                                                    while !thread_ids.insert(custom_thread_id) {
                                                        custom_thread_id = custom_thread_id.wrapping_add(1);
                                                    }
                                                    e.insert(custom_thread_id);

                                                    let args = format_args!(
                                                        r#"{separator}{{"ph":"M","pid":{name_sys_pid},"tid":{custom_thread_id},"name":"thread_name","args":{{"name":{name}}}}}"#,
                                                    );
                                                    out.write_fmt(args)
                                                        .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));

                                                    custom_thread_id
                                                }
                                            };
                                            *n = tid.into();
                                        }
                                    }
                                    Phase::Other => {}
                                }
                            }
                        }
                        _ => {
                            error!("unknown format in {}", trace.display());
                        }
                    }

                    out.write_all(separator.as_bytes())
                        .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));
                    serde_json::to_writer(&mut out, &entry).unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));
                    separator = ",\n";
                }
                Err(_) => break,
            }
        }
    }

    out.write_all(b"\n]")
        .unwrap_or_else(|e| fatal!("cannot write {}, {e}", out_file.display()));
    println!("saved to {}", out_file.display());

    if code == 0 {
        crate::util::exit();
    } else {
        // forward the exit code from the exe or cmd
        std::process::exit(code);
    }
}
