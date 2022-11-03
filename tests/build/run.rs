use std::{fs, io};

pub fn do_request() {
    if let Some(test) = std::env::var_os("DO_TASKS_TEST_BUILD") {
        let mut test = test.to_string_lossy();

        if ["*", "**"].contains(&test.as_ref()) {
            test = "*/*".into();
        }

        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
        std::env::set_var("TRYBUILD", "overwrite");

        {
            trybuild::TestCases::new().compile_fail(format!("cases/{test}.rs"));
            // trybuild runs on drop
        }

        cleanup(&test);
    } else {
        eprintln!("run with `cargo do test --build *`");
    }
}

fn cleanup(test: &str) {
    let cases = glob::glob(&format!("cases/{test}.stderr")).expect("cleanup glob error");
    for case in cases {
        let cleanup_case = move || {
            let case = case.map_err(glob::GlobError::into_error)?;
            let raw = fs::read_to_string(&case)?;

            let mut clean = String::with_capacity(raw.len());
            let mut skip_trait_impl = false;

            let mut changed = false;

            for line in raw.lines() {
                let test_line = line.trim();
                if !skip_trait_impl {
                    skip_trait_impl = test_line.starts_with("= help: the following other types implement trait");

                    clean.push_str(line);
                    clean.push('\n');
                } else if (test_line.starts_with("and ") && test_line.ends_with(" others")) || test_line.starts_with("= note") {
                    changed = true;
                    skip_trait_impl = false;
                    clean.push_str("            <implementers-list>\n")
                } else {
                    changed = true;
                }
            }

            if changed {
                fs::write(&case, clean)?;
            }

            io::Result::Ok(if changed { Some(case) } else { None })
        };

        match cleanup_case() {
            Ok(changed) => {
                if let Some(case) = changed {
                    println!("cleanup {}", case.display());
                }
            }
            Err(e) => {
                eprintln!("ERROR: cleanup error, {e}")
            }
        }
    }
}
