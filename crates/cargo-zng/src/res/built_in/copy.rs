use super::*;

const COPY_HELP: &str = "
Copy the file or dir

The request file:
  source/foo.txt.zr-copy
   | # comment
   | path/bar.txt

Copies `path/bar.txt` to:
  target/foo.txt

Paths are relative to the Cargo workspace root
";
pub(super) fn copy() {
    help(COPY_HELP);

    // read source
    let source = read_path(&path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    // target derived from the request file name
    let mut target = path(ZR_TARGET);
    // request without name "./.zr-copy", take name from source (this is deliberate not documented)
    if target.ends_with(".zr-copy") {
        target = target.with_file_name(source.file_name().unwrap());
    }

    if source.is_dir() {
        println!("{}", display_path(&target));
        fs::create_dir(&target).unwrap_or_else(|e| {
            if e.kind() != io::ErrorKind::AlreadyExists {
                fatal!("{e}")
            }
        });
        copy_dir_all(&source, &target, true);
    } else if source.is_file() {
        println!("{}", display_path(&target));
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
    } else if source.is_symlink() {
        symlink_warn(&source);
    } else {
        warn!("cannot copy '{}', not found", source.display());
    }
}

fn copy_dir_all(from: &Path, to: &Path, trace: bool) {
    for entry in walkdir::WalkDir::new(from).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", from.display()));
        let from = entry.path();
        let to = to.join(entry.file_name());
        if entry.file_type().is_dir() {
            fs::create_dir(&to).unwrap_or_else(|e| {
                if e.kind() != io::ErrorKind::AlreadyExists {
                    fatal!("cannot create_dir `{}`, {e}", to.display())
                }
            });
            if trace {
                println!("{}", display_path(&to));
            }
            copy_dir_all(from, &to, trace);
        } else if entry.file_type().is_file() {
            fs::copy(from, &to).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", from.display(), to.display()));
            if trace {
                println!("{}", display_path(&to));
            }
        } else if entry.file_type().is_symlink() {
            symlink_warn(entry.path())
        }
    }
}
