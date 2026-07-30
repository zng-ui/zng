use std::{collections::HashSet, fs, io, path::PathBuf};

use zng::task::Progress;

use crate::tasks::SetupTaskError;

/// Setup task that extracts a directory from a TAR container to a directory on install and removes these files on uninstall.
pub enum ExtractTarTask {}
impl super::SetupTask for ExtractTarTask {
    type InstallConfig = ExtractTarConfig;

    type PrepareInstall = PrepareInstallData;

    type Install = InstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/ExtractTar".into()
    }

    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> super::Result<Self::PrepareInstall> {
        let (parent_dir, dir_name) = match (args.config.target_dir.parent(), args.config.target_dir.file_name()) {
            (Some(p), Some(n)) if let Some(n) = n.to_str() => (p, n),
            _ => {
                return Err(SetupTaskError::Io(vec![(
                    args.config.target_dir,
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid target"),
                )]));
            }
        };
        if let Err(e) = fs::create_dir_all(parent_dir) {
            return Err(SetupTaskError::Io(vec![(parent_dir.to_owned(), e)]));
        }

        // find a temp path beside the target_dir
        let mut retries = 0;
        let (tmp, state) = loop {
            let tmp = parent_dir.join(format!("{dir_name}-temp{retries}"));
            let state = tmp.join(TEMP_STATE);

            if tmp.is_dir() {
                if state.exists() {
                    if let Ok(s) = fs::read_to_string(&state)
                        && s != "prepared"
                        && fs::remove_dir_all(&tmp).is_ok()
                    {
                        // existed, but was leftover from a failed cleanup
                        break (tmp, state);
                    }
                } else if fs::remove_dir(&tmp).is_ok() {
                    // existed, but empty
                    break (tmp, state);
                }
            }

            retries += 1;
            if retries == 1000 {
                return Err(SetupTaskError::Io(vec![(
                    args.config.target_dir,
                    io::Error::new(io::ErrorKind::QuotaExceeded, "cannot create temp target"),
                )]));
            }
        };
        if let Err(e) = fs::create_dir(&tmp) {
            return Err(SetupTaskError::Io(vec![(tmp, e)]));
        }
        if let Err(e) = fs::write(&state, "preparing") {
            let _ = fs::remove_dir(&tmp);
            return Err(SetupTaskError::Io(vec![(state, e)]));
        }

        macro_rules! error {
            ($e:expr) => {{
                let _ = fs::write(&state, "error");
                let _ = fs::remove_dir(&tmp);
                return Err($e);
            }};
        }

        let mut entries = HashSet::new();

        let progress = args.progress;

        // extract
        let mut tar = tar::Archive::new(args.config.tar);
        let tar_entries = match tar.entries() {
            Ok(e) => e,
            Err(e) => error!(SetupTaskError::Io(vec![(":tar/entries".into(), e)])),
        };
        for entry in tar_entries {
            let mut entry = match entry {
                Ok(e) => e,
                Err(e) => error!(SetupTaskError::Io(vec![(":tar/entry".into(), e)])),
            };
            let path = match entry.path() {
                Ok(p) => p.into_owned(),
                Err(e) => error!(SetupTaskError::Io(vec![(":tar/entry/path".into(), e)])),
            };
            if !entries.insert(path) {
                error!(SetupTaskError::Io(vec![(
                    ":tar/entry/path".into(),
                    io::Error::new(io::ErrorKind::InvalidData, "repeated file in tar")
                )]))
            }
            if let Err(e) = entry.unpack_in(&tmp) {
                error!(SetupTaskError::Io(vec![(":tar/entry/unpack_in".into(), e)]))
            }

            if args.cancel.get() {
                break;
            }
        }

        // if is updating find orphan entries
        let mut remove = vec![];
        if let Some(prev) = args.update && !args.cancel.get() {
            if prev.target_dir != args.config.target_dir {
                remove = prev
                    .entries
                    .into_iter()
                    .filter_map(|p| {
                        let p = prev.target_dir.join(p);
                        if p.exists() { Some(p) } else { None }
                    })
                    .collect();
            } else {
                remove = prev
                    .entries
                    .into_iter()
                    .filter_map(|p| {
                        if entries.contains(&p) {
                            None
                        } else {
                            let p = prev.target_dir.join(p);
                            if p.exists() { Some(p) } else { None }
                        }
                    })
                    .collect();
            }
        }

        let _ = fs::write(&state, "prepared");

        Ok(PrepareInstallData {
            temp_dir: tmp,
            target_dir: args.config.target_dir,
            add: entries.into_iter().collect(),
            remove,
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> super::Result<Self::Install> {
        todo!()
    }

    async fn cancel_install(args: super::CancelInstallArgs<Self>) -> super::Result<()> {
        if let Err(e) = fs::remove_dir_all(&args.data.temp_dir)
            && !matches!(e.kind(), io::ErrorKind::NotFound)
        {
            return Err(SetupTaskError::Io(vec![(args.data.temp_dir, e)]));
        }
        Ok(())
    }

    async fn validate_uninstall(args: super::ValidateUninstallArgs<Self>) -> super::Result<Self::Install> {
        let mut data = args.data;
        if !data.target_dir.exists() {
            data.entries.clear();
            return Ok(data);
        }

        // retain only entries that exist
        // expand entries to full path
        // check if entries are really in the target_dir
        let len = data.entries.len() as u64;
        let mut i = 0u64;
        let mut invalid_entry = false;
        data.entries.retain_mut(|e| {
            let path = data.target_dir.join(&e);
            let retain = if path.starts_with(&data.target_dir) {
                path.exists()
            } else {
                invalid_entry = true;
                false
            };
            *e = path;
            i += 1;
            if i.is_multiple_of(50) {
                // avoid too many progress notifications
                args.progress.set(Progress::from_n_of(i, len));
            }
            retain
        });
        if invalid_entry {
            return Err(super::SetupTaskError::CorruptedTaskData("entry path not in target dir".into()));
        }

        if i > 50 {
            // go back to indeterminate if notified
            args.progress.set(Progress::indeterminate());
        }

        // sort so deeper files are removed first, because dirs are only removed if empty
        data.entries.sort_by(|a, b| b.cmp(a));

        Ok(data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> super::Result<()> {
        let mut errors = vec![];
        for entry in args.data.entries {
            // !!: TODO
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(super::SetupTaskError::Io(errors))
        }
    }
}

const TEMP_STATE: &str = ".zng-setup-ExtractTarTask";

/// Config for [`ExtractTarTask`]
pub struct ExtractTarConfig {
    tar: Box<dyn io::Read + Send>,
    target_dir: PathBuf,
}

impl ExtractTarConfig {
    /// New config.
    ///
    /// * `tar` must read only a TAR stream from start to end.
    /// * `target_dir` Directory that will be created or merged with the `tar` root directory.
    ///
    /// If all entries share a common first path component (for example, `my-app/bin/app.exe` and `my-app/README.md`),
    /// that directory is created inside `target_dir`. To extract files directly into `target_dir`, the entries must
    /// not have a common leading directory component.
    pub fn new(tar: Box<dyn io::Read + Send>, target_dir: PathBuf) -> Self {
        Self { tar, target_dir }
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    temp_dir: PathBuf,
    target_dir: PathBuf,
    /// relative to the temp_dir, must move to temp_dir
    add: Vec<PathBuf>,
    /// absolute paths from previous install
    remove: Vec<PathBuf>,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    target_dir: PathBuf,
    // entries relative to the `target_dir` that where created by the task.
    entries: Vec<PathBuf>,
}
