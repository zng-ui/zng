//! Helpers for apps built with `#![windows_subsystem = "windows"]`.
//!
//! The Windows operating system does not support hybrid CLI and GUI apps in the same executable,
//! this module contains helpers that help provide a *best effort* compatibility, based on the tricks
//! Microsoft Visual Studio uses.
//!
//! The [`attach_console`] function must be called at the start of the hybrid executable when it determinate
//! it is running in CLI mode, the [`build_cli_com_proxy`] must be called in the build script for the hybrid executable.
//!
//! The functions in this module are noop in other systems.
//!
//! See the `zng::env::windows_subsystem` docs for a full example.

/// Connect to parent stdio if disconnected.
///
/// In a CLI app this does nothing, in a GUI app (windows_subsystem = "windows") attaches to console.
///
/// Note that the Windows console returns immediately when it spawns `"windows"` executables, so any output
/// will not appear to be from your app and nothing stops the user from spawning another app causing text
/// from your app to mix with other streams. This is bas but it is what VSCode does, see [`build_cli_com_proxy`] for
/// how to implement a more polished solution.
///
/// [`build_cli_com_proxy`]: https://zng-ui.github.io/doc/zng_env/windows_subsystem/fn.build_cli_com_proxy.html
pub fn attach_console() {
    imp::attach_console();
}

/// Compile a small console executable that proxies CLI requests to a full hybrid CLI and GUI executable.
///
/// The `exe_name` must be the name of the full executable, with lower case `.exe` extension.
///
/// The `exe_dir` is only required if the target dir is not `$OUT_DIR/../../../`.
///
/// The full executable must call [`attach_console`] at the beginning.
///
/// # How it Works
///
/// This will compile a `foo.com` executable beside the `foo.exe`, both must be deployed in the same dir.
/// When users call the `foo` command from command line the `foo.com` is selected, it simply proxies all requests to the
/// `foo.exe` and holds the console open. This is the same trick used by Visual Studio with `devenv.com` and `devenv.exe`.
///
/// # Code Signing
///
/// If you code sign the full executable or configure any other policy metadata on it you mut repeat the signing for the
/// proxy executable too. Note that the generated .com executable is a normal PE file (.exe), it is just renamed to have higher priority.
///
/// # Panics
///
/// Panics if not called in a build script (build.rs). Returns an error in case of sporadic IO errors.
#[cfg(feature = "build_cli_com_proxy")]
pub fn build_cli_com_proxy(exe_name: &str, exe_dir: Option<std::path::PathBuf>) -> std::io::Result<()> {
    imp::build_cli_com_proxy(exe_name, exe_dir)
}

#[cfg(not(windows))]
mod imp {
    pub fn attach_console() {}
    #[cfg(feature = "build_cli_com_proxy")]
    pub fn build_cli_com_proxy(_: &str, _: Option<std::path::PathBuf>) -> std::io::Result<()> {
        unreachable!()
    }
}

#[cfg(windows)]
mod imp {

    pub fn attach_console() {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetConsoleWindow() -> isize;
            fn AttachConsole(process_id: u32) -> i32;
        }
        unsafe {
            // If no console is attached, attempt to attach to parent
            if GetConsoleWindow() == 0 {
                let _ = AttachConsole(0xFFFFFFFF);
            }
        }
    }

    #[cfg(feature = "build_cli_com_proxy")]
    pub fn build_cli_com_proxy(exe_name: &str, exe_dir: Option<std::path::PathBuf>) -> std::io::Result<()> {
        use std::{
            env, fs,
            path::{Path, PathBuf},
            process::Command,
        };

        macro_rules! proxy {
        ($($code:tt)+) => {
            #[allow(unused)]
            mod validate {
                $($code)*
            }
            const CODE: &str = stringify!($($code)*);
        };
    }
        proxy! {
            #![crate_name = "zng_env_build_cli_com_proxy"]
            use std::{
                env,
                process::{Command, Stdio},
            };
            fn main() {
                let mut exe = Command::new(env::current_exe().unwrap().with_file_name("{EXE_NAME}"))
                    .args(env::args_os().skip(1))
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap();
                let status = exe.wait().unwrap();
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        let code = CODE.replace("{EXE_NAME}", exe_name);

        let name = exe_name.strip_suffix(".exe").expect("expected name with .exe extension");
        let com_name = format!("{name}.com");

        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR, must be called in build.rs"));
        let proxy_src = out_dir.join(format!("zng-env-com-proxy.{name}.rs"));
        let proxy_com = out_dir.join(&com_name);
        std::fs::write(&proxy_src, code)?;
        let status = Command::new("rustc")
            .arg(&proxy_src)
            .arg("-o")
            .arg(&proxy_com)
            .arg("-C")
            .arg("opt-level=z")
            .arg("-C")
            .arg("panic=abort")
            .arg("-C")
            .arg("strip=symbols")
            .status()?;
        if !status.success() {
            panic!("failed to compile generated cli com proxy");
        }

        let target_dir = match &exe_dir {
            Some(d) => d.as_path(),
            None => {
                let d = || -> Option<&Path> { out_dir.parent()?.parent()?.parent() };
                d().expect("cannot find exe_dir")
            }
        };
        let final_proxy_com = target_dir.join(&com_name);
        fs::copy(&proxy_com, &final_proxy_com)?;

        Ok(())
    }
}
