use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // collects lines from main.rs that start with "// do " and comment lines directly after then.

    let out_file = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tasks-help.stdout");
    let input_file = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("src/main.rs");

    let rs = fs::read_to_string(input_file).expect("failed to read help comments");

    let mut out = File::create(out_file).expect("failed to write help comments");

    let mut expect_details = false;
    for line in rs.lines() {
        if line.starts_with("// do ") {
            expect_details = true;
            let task_line = &line["// do ".len()..];
            let task_name_end = task_line.find(' ').unwrap();
            writeln!(
                out,
                "\n    <task>{}</task>{}", // me mark the task name for coloring.
                &task_line[..task_name_end],
                &task_line[task_name_end..]
            )
            .expect("failed to write help comments");
        } else if expect_details {
            expect_details = line.starts_with("//");
            if expect_details {
                writeln!(out, "    {}", &line["//".len()..]).expect("failed to write help comments");
            }
        }
    }
}
