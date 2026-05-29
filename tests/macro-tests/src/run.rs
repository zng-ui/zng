use std::{fs, io};

pub(crate) fn do_request() {
    if let Some(test) = std::env::var_os("DO_TASKS_TEST_MACRO") {
        let mut test = test.to_string_lossy();

        if ["*", "**"].contains(&test.as_ref()) {
            test = "*/*".into();
        }

        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
        unsafe {
            // SAFETY: do_request is called by main, no other thread spawned so far
            std::env::set_var("TRYBUILD", "overwrite");
        }

        {
            trybuild::TestCases::new().compile_fail(format!("cases/{test}.rs"));
            // trybuild runs on drop
        }

        cleanup(&test);
    } else {
        eprintln!("run with `cargo do test --macro --all`");
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
                if !skip_trait_impl {
                    skip_trait_impl = line.contains("help: the following other types implement trait")
                        || (line.contains("= help: `") && line.contains("` implements trait `"));

                    clean.push_str(line);
                    clean.push('\n');
                } else if line.contains("note: required") || line.is_empty() {
                    skip_trait_impl = false;
                    clean.push_str("            <implementers-list>\n");
                    clean.push_str(line);
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
