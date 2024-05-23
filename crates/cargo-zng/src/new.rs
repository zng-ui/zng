//! Initialize a new repository from a Zng template repository

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use clap::*;
use color_print::cstr;

use crate::util;

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Project Name
    ///
    /// Can be a simple "name" or a "qualifier/org/project-name".
    ///
    /// EXAMPLES
    ///
    /// "br.com/My Org/My App" generates a `./my-app` project and sets metadata
    ///
    /// "my_app" generates a `./my_app` project
    ///
    /// "My App" generates a `./my-app` project
    name: String,

    /// Zng template
    ///
    /// Can be `.git` URL or an `owner/repo` for a GitHub repository.
    ///
    /// Can also be an absolute path or `./path` to a local template directory.
    #[arg(short, long, default_value = "zng-ui/zng-template")]
    template: String,
}

pub fn run(args: NewArgs) {
    let name = parse_name(args.name);
    let template = parse_template(args.template);
    let package_name = name.package_name();
    // let crate_name = name.crate_name();

    println!(cstr!("<bold>validate name and init repo<bold>"));
    if let Err(e) = util::cmd("cargo new --quiet --bin", &[package_name.as_str()], &[]) {
        let _ = cleanup_cargo_new(&package_name);
        fatal!("{e}");
    }

    if let Err(e) = cleanup_cargo_new(&package_name) {
        fatal!("failed to cleanup `cargo new` template, {e}");
    }

    println!(cstr!("<bold>clone template to temp dir<bold>"));
    let template_temp = format!("{package_name}.zng_template.tmp");
    if let Err(e) = template.git_clone(&template_temp) {
        fatal!("failed to clone template, {e}");
    }

    println!(cstr!("<bold>generate template<bold>"));
    let cx = Fmt::new(&name);
    if let Err(e) = apply_template(&cx, &template_temp, &package_name) {
        error!("{e}");
        let _ = fs::remove_dir_all(&template_temp);
        let _ = fs::remove_dir_all(&package_name);
        util::exit();
    }
}

struct Name {
    qualifier: String,
    org: String,
    app: String,
}
impl Name {
    fn package_name(&self) -> String {
        self.app.replace(' ', "-").to_lowercase()
    }

    fn crate_name(&self) -> String {
        self.package_name().replace('-', "_")
    }
}

fn parse_name(arg: String) -> Name {
    let arg: Box<[_]> = arg.splitn(3, '/').map(|s| s.trim()).collect();
    let qualifier;
    let org;
    let app;
    if arg.len() == 3 {
        qualifier = arg[0];
        org = arg[1];
        app = arg[2];
    } else {
        qualifier = "";
        org = "";
        app = arg[0];
    }

    if arg.len() == 2 || app.contains('/') {
        fatal!(r#"NAME must be a "name" or a "qualifier/organization/application""#);
    }

    Name {
        qualifier: qualifier.to_owned(),
        org: org.to_owned(),
        app: app.to_owned(),
    }
}

fn parse_template(arg: String) -> Template {
    if arg.ends_with(".git") {
        return Template::Git(arg);
    }

    if arg.starts_with("./") {
        return Template::Local(PathBuf::from(arg));
    }

    if let Some((owner, repo)) = arg.split_once('/') {
        if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') {
            return Template::Git(format!("https://github.com/{owner}/{repo}.git"));
        }
    }

    let path = PathBuf::from(arg);
    if path.is_absolute() {
        return Template::Local(path);
    }

    fatal!("--template must be a `.git` URL, `owner/repo`, `./local` or `/absolute/local`");
}

enum Template {
    Git(String),
    Local(PathBuf),
}
impl Template {
    fn git_clone(self, to: &str) -> io::Result<()> {
        let from = match self {
            Template::Git(url) => url,
            Template::Local(path) => {
                let path = path.canonicalize()?;
                let path = path.display().to_string();
                // Windows inserts this "\\?", git does not like it
                #[cfg(windows)]
                {
                    path.trim_start_matches(r#"\\?"#).replace('\\', "/")
                }

                #[cfg(not(windows))]
                {
                    path
                }
            }
        };
        util::cmd("git clone --depth 1", &[from.as_str(), to], &[])
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

fn apply_template(cx: &Fmt, template_temp: &str, package_name: &str) -> io::Result<()> {
    let template_temp = PathBuf::from(template_temp);
    // remove template .git
    fs::remove_dir_all(template_temp.join(".git"))?;
    // rename/rewrite template and move it to new package dir
    apply(cx, &template_temp, &PathBuf::from(package_name))?;
    // remove (empty) template temp
    fs::remove_dir_all(&template_temp)
}

fn apply(cx: &Fmt, from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let from = entry?.path();
        if from.is_dir() {
            let from = cx.rename(&from)?;
            let to = to.join(from.file_name().unwrap());
            println!("{}", to.display());
            fs::create_dir(&to)?;
            apply(cx, &from, &to)?;
        } else if from.is_file() {
            let from = cx.rename(&from)?;
            let to = to.join(from.file_name().unwrap());
            cx.rewrite(&from)?;
            println!("{}", to.display());
            fs::rename(from, to).unwrap();
        }
    }
    Ok(())
}

struct Fmt {
    rename: Vec<(&'static str, String)>,
    rewrite: Vec<(&'static str, String)>,
}
impl Fmt {
    fn new(name: &Name) -> Self {
        let rename = vec![
            ("t-app-t", name.package_name()),
            ("t_app_t", name.crate_name()),
            ("T_APP_T", name.crate_name().to_uppercase()),
            ("T-APP-T", name.package_name().to_uppercase()),
        ];
        let rewrite = vec![
            ("t.App.t", name.app.clone()),
            ("t-App-t", name.app.replace(' ', "-")),
            ("t.Org.t", name.org.clone()),
            ("t-Org-t", name.org.replace(' ', "-")),
            ("t.qualifier.t", name.qualifier.clone()),
        ];

        Self { rename, rewrite }
    }

    fn rename(&self, template_path: &Path) -> io::Result<PathBuf> {
        let mut path = template_path.to_string_lossy().into_owned();
        for (key, value) in &self.rename {
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
                for (key, value) in self.rename.iter().chain(&self.rewrite) {
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
