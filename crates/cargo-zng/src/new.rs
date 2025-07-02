//! Initialize a new project from a Zng template repository

use std::{
    fs, io, mem,
    path::{Path, PathBuf},
};

use clap::*;
use color_print::cstr;
use convert_case::{Case, Casing};

use crate::util;

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Set template values by position
    ///
    /// The first value for all templates is the app name.
    ///
    /// EXAMPLE
    ///
    /// cargo zng new "My App!" | creates a "my-app" project.
    ///
    /// cargo zng new "my_app"  | creates a "my_app" project.
    #[arg(num_args(0..))]
    value: Vec<String>,

    /// Zng template
    ///
    /// Can be a .git URL or an `owner/repo` for a GitHub repository.
    /// Can also be an absolute path or `./path` to a local template directory.
    ///
    /// Use `#branch` to select a branch, that is `owner/repo#branch`.
    #[arg(short, long, default_value = "zng-ui/zng-template")]
    template: String,

    /// Set a template value
    ///
    /// Templates have a `.zng-template/keys` file that defines the possible options.
    ///
    /// EXAMPLE
    ///
    /// -s"key=value" -s"k2=v2"
    #[arg(short, long, num_args(0..))]
    set: Vec<String>,

    /// Show all possible values that can be set on the template
    #[arg(short, long, action)]
    keys: bool,
}

pub fn run(args: NewArgs) {
    let template = parse_template(args.template);

    if args.keys {
        return print_keys(template);
    }

    let arg_keys = match parse_key_values(args.value, args.set) {
        Ok(arg_keys) => {
            if arg_keys.is_empty() || (!arg_keys[0].0.is_empty() && arg_keys.iter().all(|(k, _)| k != "app")) {
                fatal!("missing required key `app`")
            }
            arg_keys
        }
        Err(e) => fatal!("{e}"),
    };

    // validate name and init
    let app = &arg_keys[0].1;
    let project_name = util::clean_value(app, true)
        .unwrap_or_else(|e| fatal!("{e}"))
        .replace(' ', "-")
        .to_lowercase();

    if Path::new(project_name.as_str()).exists() {
        let full_path = Path::new(project_name.as_str())
            .canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or(project_name);
        fatal!("destination `{full_path}` already exists");
    }

    if let Err(e) = util::cmd("cargo new --quiet --bin", &[project_name.as_str()], &[]) {
        let _ = std::fs::remove_dir_all(&project_name);
        fatal!("cannot init project folder, {e}");
    }

    if let Err(e) = cleanup_cargo_new(&project_name) {
        fatal!("failed to cleanup `cargo new` template, {e}");
    }

    // clone template
    let template_temp = PathBuf::from(format!("{project_name}.zng_template.tmp"));

    let fatal_cleanup = || {
        let _ = fs::remove_dir_all(&template_temp);
        let _ = fs::remove_dir_all(&project_name);
    };

    let (template_keys, ignore) = template.git_clone(&template_temp, false).unwrap_or_else(|e| {
        fatal_cleanup();
        fatal!("failed to clone template, {e}")
    });

    let cx = Context::new(&template_temp, template_keys, arg_keys, ignore).unwrap_or_else(|e| {
        fatal_cleanup();
        fatal!("cannot parse template, {e}")
    });
    // generate template
    if let Err(e) = apply_template(&cx, &project_name) {
        error!("cannot generate, {e}");
        fatal_cleanup();
        util::exit();
    }

    // cargo zng fmt
    if Path::new(&project_name).join("Cargo.toml").exists() {
        if let Err(e) = std::env::set_current_dir(project_name) {
            fatal!("cannot format generated project, {e}")
        }
        crate::fmt::run(crate::fmt::FmtArgs::default());
    }
}

fn parse_key_values(value: Vec<String>, define: Vec<String>) -> io::Result<ArgsKeyMap> {
    let mut r = Vec::with_capacity(value.len() + define.len());

    for value in value {
        r.push((String::new(), value));
    }

    for key_value in define {
        if let Some((key, value)) = key_value.trim_matches('"').split_once('=') {
            if !is_key(key) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid key `{key}`")));
            }
            r.push((key.to_owned(), value.to_owned()));
        }
    }

    Ok(r)
}

fn print_keys(template: Template) {
    for i in 0..100 {
        let template_temp = std::env::temp_dir().join(format!("cargo-zng-template-keys-help-{i}"));
        if template_temp.exists() {
            continue;
        }

        match template.git_clone(&template_temp, true) {
            Ok((keys, _)) => {
                println!("TEMPLATE KEYS\n");
                for kv in keys {
                    let value = match &kv.value {
                        Some(dft) => dft.as_str(),
                        None => cstr!("<bold><y>required</y></bold>"),
                    };
                    println!(cstr!("<bold>{}=</bold>{}"), kv.key, value);
                    if !kv.docs.is_empty() {
                        for line in kv.docs.lines() {
                            println!("   {line}");
                        }
                        println!();
                    }
                }
            }
            Err(e) => {
                error!("failed to clone template, {e}");
            }
        }
        let _ = fs::remove_dir_all(&template_temp);
        return;
    }
    fatal!("failed to clone template, no temp dir available");
}

fn parse_template(arg: String) -> Template {
    let (arg, branch) = arg.rsplit_once('#').unwrap_or((&arg, ""));

    if arg.ends_with(".git") {
        return Template::Git(arg.to_owned(), branch.to_owned());
    }

    if arg.starts_with("./") {
        return Template::Local(PathBuf::from(arg), branch.to_owned());
    }

    if let Some((owner, repo)) = arg.split_once('/') {
        if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') {
            return Template::Git(format!("https://github.com/{owner}/{repo}.git"), branch.to_owned());
        }
    }

    let path = PathBuf::from(arg);
    if path.is_absolute() {
        return Template::Local(path.to_owned(), branch.to_owned());
    }

    fatal!("--template must be a `.git` URL, `owner/repo`, `./local` or `/absolute/local`");
}

enum Template {
    Git(String, String),
    Local(PathBuf, String),
}
impl Template {
    /// Clone repository, if it is a template return the `.zng-template/keys,ignore` files contents.
    fn git_clone(self, to: &Path, include_docs: bool) -> io::Result<(KeyMap, Vec<glob::Pattern>)> {
        let (from, branch) = match self {
            Template::Git(url, b) => (url, b),
            Template::Local(path, b) => {
                let path = dunce::canonicalize(path)?;
                (path.display().to_string(), b)
            }
        };
        let to_str = to.display().to_string();
        let mut args = vec![from.as_str(), &to_str];
        if !branch.is_empty() {
            args.push("--branch");
            args.push(&branch);
        }
        util::cmd_silent("git clone --depth 1", &args, &[])?;

        let keys = match fs::read_to_string(to.join(".zng-template/keys")) {
            Ok(s) => parse_keys(s, include_docs)?,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "git repo is not a zng template, missing `.zng-template/keys`",
                    ));
                }
                return Err(e);
            }
        };

        let mut ignore = vec![];
        match fs::read_to_string(to.join(".zng-template/ignore")) {
            Ok(i) => {
                for glob in i.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    let glob = glob::Pattern::new(glob).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    ignore.push(glob);
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }

        Ok((keys, ignore))
    }
}

fn cleanup_cargo_new(path: &str) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn apply_template(cx: &Context, package_name: &str) -> io::Result<()> {
    let template_temp = &cx.template_root;

    // remove template .git
    fs::remove_dir_all(template_temp.join(".git"))?;

    // replace keys in post scripts
    let post = template_temp.join(".zng-template/post");
    if post.is_dir() {
        let post_replaced = template_temp.join(".zng-template/post-temp");
        fs::create_dir_all(&post_replaced)?;
        apply(cx, true, &post, &post_replaced)?;
        fs::remove_dir_all(&post)?;
        fs::rename(&post_replaced, &post)?;

        unsafe {
            // SAFETY: cargo-zng new is single-threaded
            std::env::set_var("ZNG_TEMPLATE_POST_DIR", &post);
        }
    }

    // rename/rewrite template and move it to new package dir
    let to = PathBuf::from(package_name);
    apply(cx, false, template_temp, &to)?;

    let bash = post.join("post.sh");
    if bash.is_file() {
        let script = fs::read_to_string(bash)?;
        crate::res::built_in::sh_run(script, false, Some(&to))?;
    } else {
        let manifest = post.join("Cargo.toml");
        if manifest.exists() {
            let s = std::process::Command::new("cargo")
                .arg("run")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(manifest)
                .current_dir(to)
                .status()?;
            if !s.success() {}
        } else if post.exists() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".zng-template/post does not contain 'post.sh' nor 'Cargo.toml'",
            ));
        }
    }

    // remove template temp
    fs::remove_dir_all(template_temp)
}

fn apply(cx: &Context, is_post: bool, from: &Path, to: &Path) -> io::Result<()> {
    for entry in walkdir::WalkDir::new(from).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry?;
        let from = entry.path();
        if cx.ignore(from, is_post) {
            continue;
        }
        if from.is_dir() {
            let from = cx.rename(from)?;
            let to = to.join(from.file_name().unwrap());
            println!("  {}", to.display());
            fs::create_dir(&to)?;
            apply(cx, is_post, &from, &to)?;
        } else if from.is_file() {
            let from = cx.rename(from)?;
            let to = to.join(from.file_name().unwrap());
            cx.rewrite(&from)?;
            println!("  {}", to.display());
            fs::rename(from, to).unwrap();
        }
    }
    Ok(())
}

struct Context {
    template_root: PathBuf,
    replace: ReplaceMap,
    ignore_workspace: glob::Pattern,
    ignore: Vec<glob::Pattern>,
}
impl Context {
    fn new(template_root: &Path, mut template_keys: KeyMap, arg_keys: ArgsKeyMap, ignore: Vec<glob::Pattern>) -> io::Result<Self> {
        for (i, (key, value)) in arg_keys.into_iter().enumerate() {
            if key.is_empty() {
                if i >= template_keys.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "more positional values them template keys",
                    ));
                }
                template_keys[i].value = Some(value);
            } else if let Some(kv) = template_keys.iter_mut().find(|kv| kv.key == key) {
                kv.value = Some(value);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown key `{key}`, not declared by template"),
                ));
            }
        }
        Ok(Self {
            template_root: dunce::canonicalize(template_root)?,
            replace: make_replacements(&template_keys)?,
            ignore_workspace: glob::Pattern::new(".zng-template").unwrap(),
            ignore,
        })
    }

    fn ignore(&self, template_path: &Path, is_post: bool) -> bool {
        let template_path = template_path.strip_prefix(&self.template_root).unwrap();

        if !is_post && self.ignore_workspace.matches_path(template_path) {
            return true;
        }

        for glob in &self.ignore {
            if glob.matches_path(template_path) {
                return true;
            }
        }
        false
    }

    fn rename(&self, template_path: &Path) -> io::Result<PathBuf> {
        let mut path = template_path.to_string_lossy().into_owned();
        for (key, value) in &self.replace {
            let s_value;
            let value = if is_sanitized_key(key) {
                value
            } else {
                s_value = sanitise_file_name::sanitize(value);
                &s_value
            };
            path = path.replace(key, value);
        }
        let path = PathBuf::from(path);
        if template_path != path {
            fs::rename(template_path, &path)?;
        }
        Ok(path)
    }

    fn rewrite(&self, template_path: &Path) -> io::Result<()> {
        match fs::read_to_string(template_path) {
            Ok(txt) => {
                let mut new_txt = txt.clone();
                for (key, value) in &self.replace {
                    new_txt = new_txt.replace(key, value);
                }
                if new_txt != txt {
                    fs::write(template_path, new_txt.as_bytes())?;
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::InvalidData {
                    // not utf-8 text file
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

static PATTERNS: &[(&str, &str, Option<Case>)] = &[
    ("t-key-t", "kebab-case", Some(Case::Kebab)),
    ("T-KEY-T", "UPPER-KEBAB-CASE", Some(Case::UpperKebab)),
    ("t_key_t", "snake_case", Some(Case::Snake)),
    ("T_KEY_T", "UPPER_SNAKE_CASE", Some(Case::UpperSnake)),
    ("T-Key-T", "Train-Case", Some(Case::Train)),
    ("t.key.t", "lower case", Some(Case::Lower)),
    ("T.KEY.T", "UPPER CASE", Some(Case::Upper)),
    ("T.Key.T", "Title Case", Some(Case::Title)),
    ("ttKeyTt", "camelCase", Some(Case::Camel)),
    ("TtKeyTt", "PascalCase", Some(Case::Pascal)),
    ("{{key}}", "Unchanged", None),
    ("f-key-f", "Sanitized", None),
    ("F-Key-F", "Title Sanitized", None),
];

type KeyMap = Vec<TemplateKey>;
type ArgsKeyMap = Vec<(String, String)>;
type ReplaceMap = Vec<(String, String)>;

struct TemplateKey {
    docs: String,
    key: String,
    value: Option<String>,
    required: bool,
}

fn is_key(s: &str) -> bool {
    s.len() >= 3 && s.is_ascii() && s.chars().all(|c| c.is_ascii_alphabetic() && c.is_lowercase())
}

fn parse_keys(zng_template_v1: String, include_docs: bool) -> io::Result<KeyMap> {
    let mut r = vec![];

    let mut docs = String::new();

    for (i, line) in zng_template_v1.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            docs.clear();
            continue;
        }

        if line.starts_with('#') {
            if include_docs {
                let mut line = line.trim_start_matches('#');
                if line.starts_with(' ') {
                    line = &line[1..];
                }
                docs.push_str(line);
                docs.push('\n');
            }
            continue;
        }

        if r.is_empty() && line != "app=" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "broken template, first key must be `app=`",
            ));
        }

        let docs = mem::take(&mut docs);
        if let Some((key, val)) = line.split_once('=') {
            if is_key(key) {
                if val.is_empty() {
                    r.push(TemplateKey {
                        docs,
                        key: key.to_owned(),
                        value: None,
                        required: true,
                    });
                    continue;
                } else if val.starts_with('"') && val.ends_with('"') {
                    r.push(TemplateKey {
                        docs,
                        key: key.to_owned(),
                        value: Some(val[1..val.len() - 1].to_owned()),
                        required: false,
                    });
                    continue;
                }
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("broken template, invalid syntax in `.zng-template:{}`", i + 1),
        ));
    }

    Ok(r)
}
fn make_replacements(keys: &KeyMap) -> io::Result<ReplaceMap> {
    let mut r = Vec::with_capacity(keys.len() * PATTERNS.len());
    for kv in keys {
        let value = match &kv.value {
            Some(v) => v,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("missing required key `{}`", kv.key),
                ));
            }
        };
        let clean_value = util::clean_value(value, kv.required)?;

        for (pattern, _, case) in PATTERNS {
            let prefix = &pattern[..2];
            let suffix = &pattern[pattern.len() - 2..];
            let (key, value) = if let Some(case) = case {
                let key_case = match case {
                    Case::Camel => Case::Pascal,
                    c => *c,
                };
                let value = match !pattern.contains('.') && !pattern.contains('{') {
                    true => &clean_value,
                    false => value,
                };
                (kv.key.to_case(key_case), value.to_case(*case))
            } else {
                (kv.key.to_owned(), value.to_owned())
            };
            let value = if is_sanitized_key(&key) {
                sanitise_file_name::sanitize(&value)
            } else {
                value
            };
            let key = format!("{prefix}{key}{suffix}");
            r.push((key, value));
        }
    }
    Ok(r)
}

fn is_sanitized_key(key: &str) -> bool {
    key.starts_with('f') || key.starts_with('F')
}
