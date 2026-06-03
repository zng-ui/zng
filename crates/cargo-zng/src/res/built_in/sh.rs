use std::process::Command;

use super::*;

const SH_HELP: &str = r#"
Run a bash script

Script is configured using environment variables (like other tools):

ZR_SOURCE_DIR — Resources directory that is being build.
ZR_TARGET_DIR — Target directory where resources are being built to.
ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
ZR_WORKSPACE_DIR — Cargo workspace that contains source dir. Also the working dir.
ZR_REQUEST — Request file that called the tool (.zr-sh).
ZR_REQUEST_DD — Parent dir of the request file.
ZR_TARGET — Target file implied by the request file name.
ZR_TARGET_DD — Parent dir of the target file.

ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

In a Cargo workspace the `zng::env::about` metadata is also set:

ZR_APP_ID — package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
ZR_APP — package.metadata.zng.about.app or package.name
ZR_ORG — package.metadata.zng.about.org or the first package.authors
ZR_VERSION — package.version
ZR_DESCRIPTION — package.description
ZR_HOMEPAGE — package.homepage
ZR_LICENSE — package.license
ZR_PKG_NAME — package.name
ZR_PKG_AUTHORS — package.authors
ZR_CRATE_NAME — package.name in snake_case
ZR_QUALIFIER — package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
ZR_META_* — any other custom string value in package.metadata.zng.about.*

Script can make requests to the resource builder by printing to stdout.
Current supported requests:

zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

If the script fails the entire stderr is printed and the resource build fails. Scripts run with
`set -e` by default.

Tries to run on $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh.
"#;
pub(super) fn sh() {
    help(SH_HELP);
    let script = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    sh_run(script, false, None).unwrap_or_else(|e| fatal!("{e}"));
}

fn sh_options() -> Vec<std::ffi::OsString> {
    let mut r = vec![];
    if let Ok(sh) = env::var("ZR_SH")
        && !sh.is_empty()
    {
        let sh = PathBuf::from(sh);
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }

    #[cfg(windows)]
    if let Ok(pf) = env::var("PROGRAMFILES") {
        let sh = PathBuf::from(pf).join("Git/bin/bash.exe");
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }
    #[cfg(windows)]
    if let Ok(c) = env::var("SYSTEMDRIVE") {
        let sh = PathBuf::from(c).join("Program Files (x86)/Git/bin/bash.exe");
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }

    r.push("bash".into());
    r.push("sh".into());

    r
}
pub(crate) fn sh_run(mut script: String, capture: bool, current_dir: Option<&Path>) -> io::Result<String> {
    script.insert_str(0, "set -e\n");

    for opt in sh_options() {
        let r = sh_run_try(&opt, &script, capture, current_dir)?;
        if let Some(r) = r {
            return Ok(r);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "cannot find bash, tried $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh",
    ))
}
fn sh_run_try(sh: &std::ffi::OsStr, script: &str, capture: bool, current_dir: Option<&Path>) -> io::Result<Option<String>> {
    let mut sh = Command::new(sh);
    if let Some(d) = current_dir {
        sh.current_dir(d);
    }
    sh.arg("-c").arg(script);
    sh.stdin(std::process::Stdio::null());
    sh.stderr(std::process::Stdio::inherit());
    let r = if capture {
        sh.output().map(|o| (o.status, String::from_utf8_lossy(&o.stdout).into_owned()))
    } else {
        sh.stdout(std::process::Stdio::inherit());
        sh.status().map(|s| (s, String::new()))
    };
    match r {
        Ok((s, o)) => {
            if !s.success() {
                return Err(match s.code() {
                    Some(c) => io::Error::other(format!("script failed, exit code {c}")),
                    None => io::Error::other("script failed"),
                });
            }
            Ok(Some(o))
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}
