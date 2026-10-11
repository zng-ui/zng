#![cfg(windows)]

use std::path::PathBuf;

use zng_txt::{ToTxt, Txt, formatx};

use crate::task::{InstallTaskError, SetupTaskError};

/// Setup task that associates file extensions to open with the app.
pub enum RegisterFileTypes {}

/// Config for [`RegisterFileTypes`].
#[non_exhaustive]
pub struct RegisterFileTypesConfig {
    /// Register file associations for all users on the local machine.
    ///
    /// If enabled setup must run as administrator.
    ///
    /// Enabled by default.
    pub all_users: bool,

    /// Globally unique app ID.
    ///
    /// # Default
    ///
    /// If this is empty the [`zng::env::About::windows_aumid`] value is used. Note that this is only valid
    /// if the setup app is defined on the same app executable.
    ///
    /// [`zng::env::About::windows_aumid`]: zng_env::About::windows_aumid
    pub app_id: Txt,

    /// Unique identifier of the file types.
    ///
    /// e.g.: "Document" or "Image".
    ///
    /// Is "File" by default.
    pub file_types_id: Txt,

    /// Short display description of the file types.
    ///
    /// e.g.: "My App Documents".
    ///
    /// Default is "{app} {file_types_id}" where {app} is [`zng::env::About::app`].
    ///
    /// [`zng::env::About::app`]: zng_env::About::app
    pub file_types_description: Txt,

    /// All the file extensions to register.
    pub extensions: Vec<Txt>,

    /// App executable that will run on associated file open.
    ///
    /// Executable must exist when the task applies.
    pub open_exe: PathBuf,
    /// Arguments to pass `target_path` when an associated file open.
    ///
    /// An empty string entry here marks the spot where the file path is inserted,
    /// or the file path is the last argument.
    pub open_args: Vec<Txt>,

    /// If should notify Windows Explorer that file associations changed.
    ///
    /// This is `true` by default, only disable if the setup registers multiple file types
    /// and this task is not the last one.
    pub refresh_shell: bool,

    /// Icon resource.
    ///
    /// Value is path to an executable or dll and index of the icon resource.
    ///
    /// Default is the `open_exe,0`.
    pub icon_res: Option<(PathBuf, usize)>,
}
impl RegisterFileTypesConfig {
    /// New from required config.
    pub fn new(extensions: Vec<Txt>, open_exe: PathBuf) -> Self {
        Self {
            all_users: true,
            app_id: zng_env::about().windows_aumid(),
            file_types_id: "File".into(),
            file_types_description: Txt::default(),
            extensions,
            open_exe,
            open_args: vec![],
            refresh_shell: true,
            icon_res: None,
        }
    }
}

impl super::SetupTask for RegisterFileTypes {
    type InstallConfig = RegisterFileTypesConfig;

    type PrepareInstall = RegisterFileTypesPrepareInstallData;

    type Install = RegisterFileTypesInstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/RegisterFileTypes".into()
    }

    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> Result<Self::PrepareInstall, SetupTaskError> {
        let c = args.config;

        if !c.open_exe.is_absolute() {
            return Err(SetupTaskError::other("open_exe must be absolute path"));
        }
        let open_exe = match c.open_exe.to_str() {
            Some(s) => s.replace("/", "\\"),
            None => return Err(SetupTaskError::other("open_exe must be utf-8")),
        };
        let mut extensions = c.extensions;
        extensions.retain(|e| !e.is_empty());
        if extensions.is_empty() {
            return Err(SetupTaskError::other("missing extensions"));
        }

        let mut open_cmd = format!("\"{open_exe}\"");
        for arg in c.open_args {
            if arg.is_empty() {
                open_cmd.push_str(" \"%1\"");
            } else {
                use std::fmt::Write as _;
                write!(&mut open_cmd, " {arg:?}").unwrap();
            }
        }

        let (icon_exe, icon_idx) = match &c.icon_res {
            Some((p, i)) => {
                if p.is_relative() {
                    return Err(SetupTaskError::other("icon_res path must be absolute"));
                }
                match p.to_str() {
                    Some(p) => (p.replace("/", "\\"), *i),
                    None => return Err(SetupTaskError::other("icon_res path must be utf-8")),
                }
            }
            None => (open_exe, 0),
        };

        Ok(RegisterFileTypesPrepareInstallData {
            update: args.update,
            all_users: c.all_users,
            id: formatx!("{}.{}", c.app_id, c.file_types_id),
            description: c.file_types_description,
            extensions,
            open_cmd: open_cmd.to_txt(),
            refresh_shell: c.refresh_shell,
            icon_res: formatx!("{icon_exe};{icon_idx}"),
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> Result<Self::Install, InstallTaskError<Self::Install>> {
        let d = args.data;
        if let Some(od) = d.update
            && (od.id != d.id || od.all_users != d.all_users)
            && let Err(e) = unregister(od.all_users, &od.id, &od.extensions)
        {
            return Err(InstallTaskError {
                error: SetupTaskError::other(e),
                clean_data: Some(od),
            });
        }

        if let Err(e) = register(d.all_users, &d.id, &d.description, &d.open_cmd, &d.icon_res, &d.extensions) {
            return Err(InstallTaskError {
                error: SetupTaskError::other(e),
                clean_data: Some(RegisterFileTypesInstallData {
                    all_users: d.all_users,
                    id: d.id,
                    extensions: d.extensions,
                }),
            });
        }

        if d.refresh_shell {
            refresh_shell();
        }

        Ok(RegisterFileTypesInstallData {
            all_users: d.all_users,
            id: d.id,
            extensions: d.extensions,
        })
    }

    async fn cancel_install(_: super::CancelInstallArgs<Self>) -> Result<(), SetupTaskError> {
        Ok(())
    }

    async fn validate_uninstall(args: super::ValidateUninstallArgs<Self>) -> Result<Self::Install, SetupTaskError> {
        Ok(args.data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> Result<(), SetupTaskError> {
        let d = args.data;
        match unregister(d.all_users, &d.id, &d.extensions) {
            Ok(()) => Ok(()),
            Err(e) => Err(SetupTaskError::other(e)),
        }
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisterFileTypesPrepareInstallData {
    update: Option<RegisterFileTypesInstallData>,
    all_users: bool,
    id: Txt,
    description: Txt,
    extensions: Vec<Txt>,
    open_cmd: Txt,
    icon_res: Txt,
    refresh_shell: bool,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisterFileTypesInstallData {
    all_users: bool,
    id: Txt,
    extensions: Vec<Txt>,
}

fn register(all_users: bool, id: &str, description: &str, open_cmd: &str, icon_res: &str, extensions: &[Txt]) -> std::io::Result<()> {
    let classes = if all_users {
        windows_registry::LOCAL_MACHINE
    } else {
        windows_registry::CURRENT_USER
    }
    .create("Software\\Classes")?;

    let progid_key = classes.create(id)?;
    progid_key.set_string("", description)?;

    let icon_key = progid_key.create("DefaultIcon")?;
    icon_key.set_string("", icon_res)?;

    let cmd_key = progid_key.create("shell\\open\\command")?;
    cmd_key.set_string("", open_cmd)?;

    for ext in extensions {
        let ext_key = if ext.starts_with('.') {
            classes.create(ext)
        } else {
            classes.create(format!(".{}", ext))
        }?;
        ext_key.set_string("", id)?;

        let open_with_key = ext_key.create("OpenWithProgids")?;
        open_with_key.set_string(id, "")?;
    }

    Ok(())
}

fn unregister(all_users: bool, id: &str, extensions: &[Txt]) -> std::io::Result<()> {
    let classes = open_key(
        if all_users {
            windows_registry::LOCAL_MACHINE
        } else {
            windows_registry::CURRENT_USER
        },
        "Software\\Classes",
    )?;
    // can this ever happen?
    let classes = match classes {
        Some(c) => c,
        None => return Ok(()),
    };

    remove_key(&classes, id)?;

    for ext in extensions {
        let ext_key = if ext.starts_with('.') {
            open_key(&classes, ext)
        } else {
            open_key(&classes, &format!(".{}", ext))
        }?;
        if let Some(ext_key) = ext_key {
            remove_value(&ext_key, id)?;
        }
        // we just leave behind the extension key because other apps
        // may have interacted with it
    }

    Ok(())
}

fn open_key(key: &windows_registry::Key, path: &str) -> std::io::Result<Option<windows_registry::Key>> {
    match key.open(path) {
        Ok(k) => Ok(Some(k)),
        Err(e) => {
            if e.code().0 as u32 == 0x80070002 {
                // not found
                Ok(None)
            } else {
                Err(e.into())
            }
        }
    }
}

fn remove_key(parent: &windows_registry::Key, path: &str) -> std::io::Result<bool> {
    match parent.remove_tree(path) {
        Ok(()) => Ok(true),
        Err(e) => {
            if e.code().0 as u32 == 0x80070002 {
                Ok(false)
            } else {
                Err(e.into())
            }
        }
    }
}

fn remove_value(key: &windows_registry::Key, name: &str) -> std::io::Result<bool> {
    match key.remove_value(name) {
        Ok(()) => Ok(true),
        Err(e) => {
            if e.code().0 as u32 == 0x80070002 {
                Ok(false)
            } else {
                Err(e.into())
            }
        }
    }
}

fn refresh_shell() {
    use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    // SAFETY: this is a simple call
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}
