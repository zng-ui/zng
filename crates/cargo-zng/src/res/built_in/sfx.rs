use std::collections::HashMap;

use serde::Deserialize;

use super::*;

const SFX_HELP: &str = r#"
Compile a self-extracting Windows executable

The request file:
  source/sfx.exe.zr-sfx
   | [sfx]
   | # executable to run, required
   | run = "target/run.exe"
   | # data file, required
   | data = "target/data.zip"
   |
   | # custom icon, default is the 'run' icon
   | icon = "res/sfx.ico"
   |
   | # optional args for 'run'
   | args = ["--foo"]
   | # optional extra env for 'run'
   | env = {
   |     FOO = "bar",
   | }
   |
   | [sign]
   | # optional, code sign the sfx exe
   | tool = "signtool sign /v /f $PFX /tr http://timestamp.sectigo.com /td SHA256 /fd SHA256 $SIGN_TARGET"
   | # only sign the sfx exe, default 'false' signs the 'run' exe too
   | # sfx-only = true

Compiles and signs a 'sfx.exe' with custom icon.

Run:

When sfx runs it extracts the 'run' executable to a temp dir and runs it.

The optional 'env' variables override the system env. The SFX_ARGS and SFX_DATA var is always set. 

The SFX_ARGS is set to the sfx command line args, '\n' separated. The first arg is the path to the sfx exe.

The SFX_DATA is set to "offset:len" of the data in the sfx exe file.

Data:

The data can be any format, use a container like TAR to package multiple files. Unlike the 'run' exe it is not
compressed by sfx.

Signing:

Code signing must be applied to both the run exe and sfx exe, to facilitate this you can set the 'sign.tool'.

The sign-tool command will run twice, with $SIGN_TARGET set to "./run.exe" and "package.exe".

In the example above The $PFX var is an example of how to set the the private key. 
Keep the private key file outside the repository and set an env var to it. In CI use
secure variables.

"#;
pub(super) fn sfx() {
    help(SFX_HELP);

    let request = std::fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    let request: Request = toml::from_str(&request).unwrap_or_else(|e| fatal!("{e}"));

    if let Some(sign) = &request.sign.tool {
        todo!()
    }

    let sfx_main = sfx_main();

    todo!()
}

#[derive(Deserialize)]
struct Request {
    package: Package,
    sign: Sign,
}

#[derive(Deserialize)]
struct Package {
    run: PathBuf,
    data: PathBuf,
    icon: Option<PathBuf>,
    args: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Sign {
    tool: Option<String>,
    only_sfx: bool,
}

// enable rust-analyzer
#[cfg(debug_assertions)]
#[path = "sfx_main.rs"]
mod sfx_main;

fn sfx_main() -> &'static str {
    #[cfg(debug_assertions)]
    #[allow(unused)]
    fn allow_unused() {
        sfx_main::main();
    }

    include_str!("sfx_main.rs")
}
