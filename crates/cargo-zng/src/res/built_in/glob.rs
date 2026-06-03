use super::*;

const GLOB_HELP: &str = "
Copy all matches in place

The request file:
  source/l10n/fluent-files.zr-glob
   | # localization dir
   | l10n
   | # only Fluent files
   | **/*.ftl
   | # except test locales
   | !:**/pseudo*

Copies all '.ftl' not in a *pseudo* path to:
  target/l10n/

The first path pattern is required and defines the entries that
will be copied, an initial pattern with '**' flattens the matches.
The path is relative to the Cargo workspace root.

The subsequent patterns are optional and filter each file or dir selected by
the first pattern. The paths are relative to each match, if it is a file 
the filters apply to the file name only, if it is a dir the filters apply to
the dir and descendants.

The glob pattern syntax is:

    ? — matches any single character.
    * — matches any (possibly empty) sequence of characters.
   ** — matches the current directory and arbitrary subdirectories.
  [c] — matches any character inside the brackets.
[a-z] — matches any characters in the Unicode sequence.
 [!b] — negates the brackets match.

And in filter patterns only:

!:pattern — negates the entire pattern.

";
pub(super) fn glob() {
    help(GLOB_HELP);

    // target derived from the request place
    let target = path(ZR_TARGET);
    let target = target.parent().unwrap();

    let request_path = path(ZR_REQUEST);
    let mut lines = read_lines(&request_path);
    let (ln, selection) = lines
        .next()
        .unwrap_or_else(|| fatal!("expected at least one path pattern"))
        .unwrap_or_else(|e| fatal!("{e}"));

    // parse first pattern
    let selection = ::glob::glob(&selection).unwrap_or_else(|e| fatal!("at line {ln}, {e}"));
    // parse filter patterns
    let mut filters = vec![];
    for r in lines {
        let (ln, filter) = r.unwrap_or_else(|e| fatal!("{e}"));
        let (filter, matches_if) = if let Some(f) = filter.strip_prefix("!:") {
            (f, false)
        } else {
            (filter.as_str(), true)
        };
        let pat = ::glob::Pattern::new(filter).unwrap_or_else(|e| fatal!("at line {ln}, {e}"));
        filters.push((pat, matches_if));
    }
    // collect first matches
    let selection = {
        let mut s = vec![];
        for entry in selection {
            s.push(entry.unwrap_or_else(|e| fatal!("{e}")));
        }
        // sorted for deterministic results in case flattened files override previous
        s.sort();
        s
    };

    let mut any = false;

    'apply: for source in selection {
        if source.is_dir() {
            let filters_root = source.parent().map(Path::to_owned).unwrap_or_default();
            'copy_dir: for entry in walkdir::WalkDir::new(&source).sort_by_file_name() {
                let source = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", source.display()));
                let source = source.path();
                // filters match 'entry/**'
                let match_source = source.strip_prefix(&filters_root).unwrap();
                for (filter, matches_if) in &filters {
                    if filter.matches_path(match_source) != *matches_if {
                        continue 'copy_dir;
                    }
                }
                let target = target.join(match_source);

                any = true;
                if source.is_dir() {
                    fs::create_dir_all(&target).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", source.display()));
                } else {
                    if let Some(p) = &target.parent() {
                        fs::create_dir_all(p).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", p.display()));
                    }
                    fs::copy(source, &target)
                        .unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
                }
                println!("{}", display_path(&target));
            }
        } else if source.is_file() {
            // filters match 'entry'
            let source_name = source.file_name().unwrap().to_string_lossy();
            for (filter, matches_if) in &filters {
                if filter.matches(&source_name) != *matches_if {
                    continue 'apply;
                }
            }
            let target = target.join(source_name.as_ref());

            any = true;
            fs::copy(&source, &target).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
            println!("{}", display_path(&target));
        } else if source.is_symlink() {
            symlink_warn(&source);
        }
    }

    if !any {
        warn!("no match")
    }
}
