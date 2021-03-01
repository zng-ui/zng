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
            let (names, options) = parse_task_line(task_line);
            let mut names = names.into_iter();
            write!(
                out,
                "\n    %c_wb%{}%c_w%",
                names.next().expect("`// do` task comment missing task name")
            )
            .unwrap();
            for name in names {
                write!(out, ", %c_wb%{}%c_w%", name).unwrap();
            }
            writeln!(out, "{}", options).unwrap();
        } else if expect_details {
            expect_details = line.starts_with("//");
            if expect_details {
                writeln!(out, "    {}", &line["//".len()..]).expect("failed to write help comments");
            }
        }
    }
}

// parse {name_0} [, {name_1}] [, {name_n}] [{options}]
fn parse_task_line(mut task_line: &str) -> (Vec<&str>, &str) {
    let mut names = Vec::with_capacity(1);

    let mut rest_is_name = true;
    while let Some(i) = task_line.find(|c| c == ' ' || c == ',') {
        names.push(&task_line[..i]);
        task_line = &task_line[i..];
        if task_line.starts_with(',') {
            task_line = task_line[1..].trim_start();
        } else {
            rest_is_name = false;
            break;
        }
    }

    if rest_is_name && !task_line.is_empty() {
        names.push(task_line);
        task_line = "";
    }

    (names, task_line)
}
