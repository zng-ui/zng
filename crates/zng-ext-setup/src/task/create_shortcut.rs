#![cfg(any(windows, target_os = "linux"))]

use std::fmt::Write as _;
use std::{fs, io, path::PathBuf};

use super::SetupTaskError;

/// Setup task that creates a shortcut to an executable file.
pub enum CreateShortcut {}

// https://docs.rs/mslnk/0.1.8/mslnk/

impl super::SetupTask for CreateShortcut {
    type InstallConfig = CreateShortcutConfig;

    type PrepareInstall = PrepareInstallData;

    type Install = InstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/CreateShortcut".into()
    }

    #[cfg(windows)]
    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> super::Result<Self::PrepareInstall> {
        let c = args.config;

        let working_dir = path_utf8(c.working_dir)?;
        let icon = path_utf8(c.icon)?;

        let mut args = String::new();
        let mut sep = "";
        for arg in &c.args {
            write!(&mut args, "{sep}{arg:?}").unwrap();
            sep = " ";
        }

        Ok(PrepareInstallData {
            link_file: c.link_file.with_added_extension("lnk"),
            target_file: c.target_file,
            working_dir,
            arguments: args,
            name: c.name,
            icon,
        })
    }

    #[cfg(target_os = "linux")]
    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> super::Result<Self::PrepareInstall> {
        let c = args.config;

        let mut desktop = "[Desktop Entry]\nVersion=1.0\nType=Application\n".to_owned();

        write!(&mut desktop, "Exec={}", path_utf8(c.target_file)?).unwrap();
        for arg in c.args {
            write!(&mut desktop, " {arg:?}").unwrap();
        }
        writeln!(&mut desktop).unwrap();

        if !c.working_dir.as_os_str().is_empty() {
            writeln!(&mut desktop, "Path={}", path_utf8(c.working_dir)?).unwrap();
        }

        if !c.icon.as_os_str().is_empty() {
            writeln!(&mut desktop, "Path={}", path_utf8(c.icon)?).unwrap();
        }

        if c.name.is_empty() {
            match c.link_file.file_name() {
                Some(n) => writeln!(&mut desktop, "Name={}", n.to_string_lossy()).unwrap(),
                None => {
                    return Err(SetupTaskError::io(
                        c.link_file,
                        io::Error::new(io::ErrorKind::InvalidData, "missing name"),
                    ));
                }
            }
        } else {
            let name = c
                .name
                .replace("\\", r"\\")
                .replace("\n", r"\n")
                .replace("\t", r"\t")
                .replace("\r", r"\r");
            writeln!(&mut desktop, "Name={name}").unwrap();
        }

        Ok(PrepareInstallData {
            link_file: c.link_file,
            desktop,
        })
    }

    #[cfg(windows)]
    async fn install(args: super::InstallArgs<Self>) -> super::Result<Self::Install> {
        fn install(d: PrepareInstallData) -> Result<(), mslnk::MSLinkError> {
            let mut l = mslnk::ShellLink::new(&d.target_file)?;

            if !d.working_dir.is_empty() {
                l.set_working_dir(Some(d.working_dir));
            }

            if !d.arguments.is_empty() {
                l.set_arguments(Some(d.arguments));
            }

            if !d.name.is_empty() {
                l.set_name(Some(d.name.clone()));
            }

            if !d.icon.is_empty() {
                l.set_icon_location(Some(d.icon));
            }

            l.create_lnk(d.link_file)
        }

        let link_file = args.data.link_file.clone();

        if let Err(e) = install(args.data) {
            return Err(SetupTaskError::other(e));
        }

        Ok(InstallData { link_file })
    }

    #[cfg(target_os = "linux")]
    async fn install(args: super::InstallArgs<Self>) -> super::Result<Self::Install> {
        fn write(link_file: &PathBuf, desktop: String) -> io::Result<()> {
            use std::io::Write as _;
            use std::os::unix::fs::PermissionsExt as _;

            let mut f = fs::File::create(link_file)?;
            f.write_all(desktop.as_bytes())?;

            let mut perms = f.metadata()?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(link_file, perms)?;

            Ok(())
        }

        if let Err(e) = write(&args.data.link_file, args.data.desktop) {
            return Err(SetupTaskError::io(args.data.link_file, e));
        }
        Ok(InstallData {
            link_file: args.data.link_file,
        })
    }

    async fn cancel_install(_: super::CancelInstallArgs<Self>) -> super::Result<()> {
        Ok(())
    }

    async fn validate_uninstall(args: super::ValidateUninstallArgs<Self>) -> super::Result<Self::Install> {
        Ok(args.data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> super::Result<()> {
        if let Err(e) = fs::remove_file(&args.data.link_file)
            && !matches!(e.kind(), io::ErrorKind::NotFound)
        {
            return Err(SetupTaskError::io(args.data.link_file, e));
        }
        Ok(())
    }
}

/// Config for [`CreateShortcut`].
pub struct CreateShortcutConfig {
    /// Path to the shortcut, without extension.
    pub link_file: PathBuf,
    /// Path to the shortcut target executable file.
    pub target_file: PathBuf,
    /// Path to the initial `current_dir` of the executable.
    pub working_dir: PathBuf,
    /// Arguments to pass the executable.
    pub args: Vec<String>,

    /// Optional display name of the shortcut.
    ///
    /// Empty is the `link_file` file name.
    pub name: String,
    /// Optional icon for the shortcut.
    ///
    /// Empty is the `target_file` icon 0 in Windows, and no icon in Linux.
    pub icon: PathBuf,
}

#[cfg(target_os = "linux")]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    link_file: PathBuf,
    desktop: String,
}

#[cfg(windows)]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    link_file: PathBuf,
    target_file: PathBuf,
    working_dir: String,
    arguments: String,
    name: String,
    icon: String,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    link_file: PathBuf,
}

fn path_utf8(p: PathBuf) -> super::Result<String> {
    match p.to_str() {
        Some(s) => Ok(s.to_owned()),
        None => Err(SetupTaskError::io(
            p,
            io::Error::new(io::ErrorKind::InvalidData, "path must be utf-8"),
        )),
    }
}
