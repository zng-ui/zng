#![cfg(any(windows, target_os = "linux"))]

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
        "zng-setup/SetupTask".into()
    }

    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> super::Result<Self::PrepareInstall> {
        let c = args.config;
        Ok(PrepareInstallData {
            link_file: c.link_file,
            target_file: c.target_file,
            working_dir: c.working_dir,
            args: c.args,
            name: c.name,
            icon: c.icon,
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> super::Result<Self::Install> {
        create(&args.data)?;
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

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    link_file: PathBuf,
    target_file: PathBuf,
    working_dir: PathBuf,
    args: Vec<String>,
    name: String,
    icon: PathBuf,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    link_file: PathBuf,
}

pub fn create(d: &PrepareInstallData) -> super::Result<()> {
    #[cfg(windows)]
    if let Err(e) = windows_create(d) {
        return Err(SetupTaskError::other(e));
    }
    #[cfg(target_os = "linux")]
    if let Err(e) = linux_create(d) {
        return Err(SetupTaskError::io(d.link_file.clone(), e));
    }
    Ok(())
}

#[cfg(windows)]
fn windows_create(d: &PrepareInstallData) -> Result<(), mslnk::MSLinkError> {
    use std::fmt::Write as _;

    let mut l = mslnk::ShellLink::new(&d.target_file)?;

    l.set_working_dir(Some(d.working_dir.display().to_string()));

    if !d.args.is_empty() {
        let mut args = String::new();
        let mut sep = "";
        for arg in &d.args {
            write!(&mut args, "{sep}{arg:?}").unwrap();
            sep = " ";
        }
        l.set_arguments(Some(args));
    }

    l.set_name(Some(d.name.clone()));

    if !d.icon.as_os_str().is_empty()
        && let Some(ico) = d.icon.to_str()
    {
        l.set_icon_location(Some(ico.to_owned()));
    }

    l.create_lnk(d.link_file.with_extension("lnk"))
}

#[cfg(target_os = "linux")]
pub fn linux_create(d: &PrepareInstallData) -> io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;

    let link_file = d.link_file.with_extension("desktop");
    let mut file = fs::File::create(&link_file)?;

    file.write_all("[Desktop Entry]\nVersion=1.0\nType=Application".as_bytes())?;
    file.write_fmt(format_args!("\nExec=\"{}\"", d.target_file.display()))?;
    for arg in &d.args {
        file.write_fmt(format_args!(" {arg:?}"))?;
    }
    file.write_fmt(format_args!("\nPath={}", d.working_dir.display()))?;

    if d.name.is_empty() {
        match d.link_file.file_name() {
            Some(n) => file.write_fmt(format_args!("\nName={}", n.to_string_lossy()))?,
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing name")),
        }
    } else {
        let name = d
            .name
            .replace("\\", r"\\")
            .replace("\n", r"\n")
            .replace("\t", r"\t")
            .replace("\r", r"\r");
        file.write_fmt(format_args!("\nName={}", name))?;
    }

    if !d.icon.as_os_str().is_empty() {
        file.write_fmt(format_args!("\nIcon={}", d.icon.display()))?;
    }

    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&link_file, perms)?;

    Ok(())
}
