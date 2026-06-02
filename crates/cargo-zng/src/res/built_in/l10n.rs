use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use once_cell::unsync::Lazy;

use super::*;

const L10N_HELP: &str = "
Copy localization files (.ftl) and optimize for release

The request file:
  source/l10n.zr-l10n
   | # comment
   | path/dev-l10n

Copies the `path/dev-l10n` dir to:
  target/l10n

Paths are relative to the Cargo workspace root

Filter:

Only localization files are included
    **/*.ftl

Development langs are excluded
    !./pseudo*
    !./template

Only lang folders that have local translations are included
    If ./{lang}/deps/** exists but no ./{lang}/*.ftl exists it is excluded

Comments are stripped

Subsetting:

If a l10n subset profile is found is is applied to the dependency localization

The subset profile is an allow list, see the docs `zng::l10n` for how to create one

The subset profile is resolved in this order:

ZNG_L10N_PROFILE_FILE env if is set
    Must be set to a .subset file path, relative to the Cargo workspace root
    If the file is a {name}.rec.subset auto includes a {name}.subset and vice versa 

res/optimization-profiles/zng-ext-l10n.rec.subset
    Default location, also includes zng-ext-l10n.subset if present

{l10n-path}/*.subset
    If multiple files match all are used

";
pub(super) fn l10n() {
    help(L10N_HELP);

    // read source
    let source = read_path(&path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    // target derived from the request file name
    let mut target = path(ZR_TARGET);
    // request without name "./.zr-copy", take name from source (this is deliberate not documented)
    if target.ends_with(".zr-l10n") {
        target = target.with_file_name(source.file_name().unwrap());
    }

    if source.is_dir() {
        println!("{}", display_path(&target));
        fs::create_dir(&target).unwrap_or_else(|e| {
            if e.kind() != io::ErrorKind::AlreadyExists {
                fatal!("{e}")
            }
        });

        l10n_filter_copy(source, target);
    } else if source.is_file() {
        fatal!("expected l10n dir, '{}' is a file", source.display());
    } else if source.is_symlink() {
        symlink_warn(&source);
    } else {
        warn!("cannot copy l10n dir '{}', not found", source.display());
    }
}

fn l10n_filter_copy(from: PathBuf, to: PathBuf) {
    let subset = allow_subset(&from);

    for from_lang in fs::read_dir(&from).unwrap_or_else(|e| fatal!("cannot read {}, {}", from.display(), e)) {
        let from_lang = from_lang
            .unwrap_or_else(|e| fatal!("cannot read {} entry, {}", from.display(), e))
            .path();
        if !from_lang.is_dir() {
            continue;
        }

        // skip pseudo* and template
        let name = from_lang.file_name().unwrap().to_string_lossy();
        if name.starts_with("pseudo") || name == "template" {
            continue;
        }

        let to_lang = to.join(from_lang.file_name().unwrap());

        // copy *.ftl and collect ./deps
        let mut any_ftl = false;
        let mut from_deps = None;
        for from_entry in fs::read_dir(&from).unwrap_or_else(|e| fatal!("cannot read {}, {}", from_lang.display(), e)) {
            let from_entry = from_entry
                .unwrap_or_else(|e| fatal!("cannot read {} entry, {}", from_lang.display(), e))
                .path();

            if from_entry.is_file() {
                if let Some(ext) = from_entry.extension()
                    && ext.eq_ignore_ascii_case("ftl")
                {
                    if !any_ftl {
                        fs::create_dir(&to_lang).unwrap_or_else(|e| fatal!("cannot create {}, {}", to_lang.display(), e));
                    }
                    any_ftl = true;

                    let to_entry = to_lang.join(from_entry.file_name().unwrap());
                    fs::copy(from_entry, &to_entry).unwrap_or_else(|e| fatal!("cannot copy to {}, {}", to_entry.display(), e));
                }
            } else if from_entry.is_dir()
                && let Some(name) = from_entry.file_name()
                && name == "deps"
            {
                from_deps = Some(from_entry);
            }
        }

        // skip lang, no local translations
        if !any_ftl {
            continue;
        }

        let from_deps = match from_deps {
            Some(p) => p,
            None => continue,
        };

        macro_rules! lazy_path {
            ($init:expr) => {
                Lazy::<PathBuf, _>::new(|| {
                    let p = $init;
                    fs::create_dir(&p).unwrap_or_else(|e| fatal!("cannot create {}, {}", p.display(), e));
                    p
                })
            };
        }
        let to_deps = lazy_path!(to_lang.join("deps"));

        // copy ./deps/*/*/*.ftl
        for from_pkg in fs::read_dir(&from_deps).unwrap_or_else(|e| fatal!("cannot read {}, {}", from_deps.display(), e)) {
            let from_pkg = from_pkg
                .unwrap_or_else(|e| fatal!("cannot read {} entry, {}", from_deps.display(), e))
                .path();
            if !from_pkg.is_dir() {
                continue;
            }

            let pkg = match from_pkg.file_name().and_then(|p| p.to_str()) {
                Some(p) => p,
                None => continue,
            };
            let subset_pkg = match subset.get(pkg) {
                Some(m) => Cow::Borrowed(m),
                None => {
                    // subset filter
                    if !subset.is_empty() {
                        continue;
                    }
                    // no filter
                    Cow::Owned(HashMap::new())
                }
            };

            let to_pkg = lazy_path!(to_deps.join(pkg));

            for from_ver in fs::read_dir(&from_pkg).unwrap_or_else(|e| fatal!("cannot read {}, {}", from_pkg.display(), e)) {
                let from_ver = from_ver
                    .unwrap_or_else(|e| fatal!("cannot read {} entry, {}", from_pkg.display(), e))
                    .path();
                if !from_ver.is_dir() {
                    continue;
                }

                let to_ver = lazy_path!(to_pkg.join(from_ver.file_name().unwrap()));

                for from_entry in fs::read_dir(&from_ver).unwrap_or_else(|e| fatal!("cannot read {}, {}", from_ver.display(), e)) {
                    let from_entry = from_entry
                        .unwrap_or_else(|e| fatal!("cannot read {} entry, {}", from_ver.display(), e))
                        .path();
                    if !from_entry.is_file() || !matches!(from_entry.extension(), Some(ext) if ext.eq_ignore_ascii_case("ftl")) {
                        continue;
                    }

                    let file = match from_entry.file_name().unwrap().to_str() {
                        Some(f) => f,
                        None => continue,
                    };
                    let subset_file = match subset_pkg.get(file) {
                        Some(m) => Cow::Borrowed(m),
                        None => {
                            if !subset_pkg.is_empty() {
                                continue;
                            }
                            Cow::Owned(HashMap::new())
                        }
                    };

                    let to_entry = lazy_path!(to_ver.join(file));

                    let ok = crate::l10n::generate_util::transform_file(
                        &from_entry,
                        &to_entry,
                        "",
                        &|id, attr| match subset_file.get(id) {
                            Some(attrs) => attr.is_empty() || attrs.contains(attr),
                            None => subset_file.is_empty(), // allow if no filter
                        },
                        &|s| Cow::Borrowed(s),
                        false,
                        false,
                    );
                    if !ok {
                        fatal!("cannot optimize {}", from_entry.display());
                    }
                }
            }
        }
    }
}

// [package => [file => [id => [attribute]]]]
type SubsetMap = HashMap<String, HashMap<String, HashMap<String, HashSet<String>>>>;

fn allow_subset(from: &Path) -> SubsetMap {
    let mut out = SubsetMap::new();

    if let Ok(path) = env::var("ZNG_L10N_PROFILE_FILE")
        && !path.is_empty()
    {
        if !path.ends_with(".subset") {
            fatal!("ZNG_L10N_PROFILE_FILE must be a .subset file");
        }
        read_subset(Path::new(&path), true, &mut out);
    } else {
        let default_profile = Path::new("res/optimization-profiles/zng-ext-l10n.rec.subset");
        if default_profile.exists() {
            read_subset(default_profile, true, &mut out);
        } else {
            let default_profile_pair = Path::new("res/optimization-profiles/zng-ext-l10n.subset");
            if default_profile_pair.exists() {
                read_subset(default_profile, false, &mut out);
            } else {
                for file in ::glob::glob(&format!("{}/*.subset", from.display())).unwrap() {
                    let file = file.unwrap_or_else(|e| fatal!("cannot read {}, {}", from.display(), e));
                    read_subset(&file, false, &mut out);
                }
            }
        }
    }

    out
}
fn read_subset(path: &Path, try_pair: bool, out: &mut SubsetMap) {
    let file = fs::File::open(path).unwrap_or_else(|e| fatal!("cannot  read {}, {}", path.display(), e));
    let file = io::BufReader::new(file);
    for (i, line) in file.lines().enumerate() {
        let line = line.unwrap_or_else(|e| fatal!("cannot read {}, {}", path.display(), e));
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let (dependency, mut key) = match line.split_once("//") {
            Some((d, k)) if !d.is_empty() && k.is_empty() => (d, k),
            _ => fatal!("unexpected line {}:{}", path.display(), i + 1),
        };
        let file = match key.split_once('/') {
            Some((f, k)) => {
                key = k;
                // add .ftl so we can match the file_name directly
                if f == "_" || f.is_empty() {
                    Cow::Borrowed("_.ftl")
                } else {
                    Cow::Owned(format!("{f}.ftl"))
                }
            }
            None => Cow::Borrowed("_.ftl"),
        };
        let (id, attribute) = match key.split_once('.') {
            Some((id, a)) => {
                if id.is_empty() || a.is_empty() {
                    fatal!("unexpected line {}:{}", path.display(), i + 1);
                }
                (id, a)
            }
            None => (key, ""),
        };

        let dep_map = match out.get_mut(dependency) {
            Some(m) => m,
            None => out.entry(dependency.to_owned()).or_default(),
        };
        let file_map = match dep_map.get_mut(&*file) {
            Some(m) => m,
            None => dep_map.entry(file.into_owned()).or_default(),
        };
        let id_map = match file_map.get_mut(id) {
            Some(m) => m,
            None => file_map.entry(id.to_owned()).or_default(),
        };
        if !attribute.is_empty() && !id_map.contains(attribute) {
            id_map.insert(attribute.to_owned());
        }
    }

    if try_pair {
        let name = path.file_name().unwrap().to_str().unwrap();
        let pair_name = if let Some(n) = name.strip_suffix(".rec.subset") {
            format!("{n}.subset")
        } else if let Some(n) = name.strip_suffix(".subset") {
            format!("{n}.rec.subset")
        } else {
            return;
        };
        let pair_path = path.parent().unwrap().join(pair_name);
        if pair_path.exists() {
            read_subset(&pair_path, false, out);
        }
    }
}
