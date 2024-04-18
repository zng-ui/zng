// Fix some doc issues.

pub fn apply() {
    println!("fixing docs");

    // fix source links to build.rs generated code.
    for dir in crate::util::glob("target/doc/src/*/code/**/out") {
        let name = &dir["target/doc/src/".len()..];
        let name = &name[..name.find('/').unwrap()];

        let name_part = name.replace('_', "-");
        let name_rgx = regex::Regex::new(&format!(r"[/\\]{name_part}-.+?[/\\]")).unwrap();

        let old_name = name_rgx.find(&dir).unwrap().as_str();
        let new_name = format!("/{name_part}-generated/");

        if old_name == new_name {
            continue;
        }

        crate::println(format_args!("fixing `{name}` generated source links"));

        let parent = &dir[..dir.find(&old_name).unwrap()];
        let old_dir = &dir[..parent.len() + old_name.len()];
        let new_dir = format!("{parent}{new_name}");

        crate::println(format_args!("    rename `{old_name}` to `{new_name}`"));
        let _ = std::fs::remove_dir_all(&new_dir);
        std::fs::rename(&old_dir, new_dir).unwrap();

        let mut count = 0;
        for file in crate::util::glob("target/doc/**/*.html") {
            let html = std::fs::read_to_string(&file).unwrap();
            let out = name_rgx.replace(&html, &new_name);

            if html != out {
                std::fs::write(file, out.as_bytes()).unwrap();
                count += 1;
            }
        }
        crate::println(format_args!("    fixed {count} files."));
    }
}
