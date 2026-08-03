use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use zng_task::Progress;
use zng_txt::{ToTxt as _, Txt, formatx};
use zng_unit::ByteUnits as _;
use zng_var::{expr_var, var};

use crate::task::SetupTaskError;

/// Setup task that extracts TAR container to a new or existing directory
/// on install and removes these files on uninstall.
pub enum ExtractTar {}
impl super::SetupTaskImpl for ExtractTar {
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

        // make a temp path beside the target_dir
        let mut retries = 0;
        let (tmp, tmp_state) = loop {
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
        if let Err(e) = fs::write(&tmp_state, "preparing") {
            let _ = fs::remove_dir(&tmp);
            return Err(SetupTaskError::Io(vec![(tmp_state, e)]));
        }

        macro_rules! error {
            ($e:expr) => {{
                let _ = fs::write(&tmp_state, "error");
                let _ = fs::remove_dir(&tmp);
                return Err($e);
            }};
        }

        let mut entries = HashSet::new();

        // prepare progress reporting
        let tar = zng_task::io::Measure::new(args.config.tar, args.config.tar_len.bytes(), 0.bytes());
        let progress_name = var(Txt::default());
        let progress = expr_var! {
            let metrics = #{tar.metrics()};
            let (n, total) = metrics.read_progress;
            if n <= total {
                Progress::from_n_of(n.0, total.0)
            } else {
                Progress::indeterminate()
            }
            .with_msg(formatx!("{}\n{}", #{progress_name.clone()}, metrics))
        };
        progress.set_bind(&args.progress).perm();

        // extract
        let mut tar = tar::Archive::new(tar);
        let tar_entries = match tar.entries() {
            Ok(e) => e,
            Err(e) => error!(SetupTaskError::Io(vec![(":tar/entries".into(), e)])),
        };
        entries.insert(PathBuf::new()); // target_dir
        for entry in tar_entries {
            let mut entry = match entry {
                Ok(e) => e,
                Err(e) => error!(SetupTaskError::Io(vec![(":tar/entry".into(), e)])),
            };
            let path = match entry.path() {
                Ok(p) => p.into_owned(),
                Err(e) => error!(SetupTaskError::Io(vec![(":tar/entry/path".into(), e)])),
            };

            let display_name = path.as_os_str().to_string_lossy().replace('\\', "/").to_txt();

            let entry_type = entry.header().entry_type();
            if entry_type.is_dir() {
                entries.insert(path);
            } else if entry_type.is_file() {
                for p in path.ancestors() {
                    if !entries.contains(p) {
                        entries.insert(p.to_owned());
                    }
                }
                if !entries.insert(path) {
                    error!(SetupTaskError::Io(vec![(
                        ":tar/entry/path".into(),
                        io::Error::new(io::ErrorKind::InvalidData, "repeated file in tar")
                    )]))
                }
            } else {
                if args.config.strict {
                    error!(SetupTaskError::Io(vec![(
                        ":tar/entry/entry_type".into(),
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("found entry {entry_type:?}, only directory and files are allowed")
                        )
                    )]))
                } else {
                    continue;
                }
            }

            progress_name.set(display_name);

            if let Err(e) = entry.unpack_in(&tmp) {
                error!(SetupTaskError::Io(vec![(":tar/entry/unpack_in".into(), e)]))
            }

            if args.cancel.get() {
                break;
            }
        }
        let _ = tar.into_inner().finish();

        // if is updating find orphan entries
        let mut remove = vec![];
        if let Some(prev) = args.update
            && !args.cancel.get()
        {
            args.progress.set(Progress::indeterminate());

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

        let _ = fs::write(&tmp_state, "prepared");

        let mut entries: Vec<_> = entries.into_iter().collect();
        entries.sort(); // root first, this allows renaming entire dirs when possible during `install`

        Ok(PrepareInstallData {
            temp_dir: tmp,
            target_dir: args.config.target_dir,
            add: entries,
            remove,
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> super::Result<Self::Install> {
        let mut errors = vec![];

        let mut entries = args.data.add;
        let mut moved_dirs = HashSet::<&Path>::new();
        for add in &entries {
            if add.ancestors().any(|p| moved_dirs.contains(p)) {
                // already renamed parent dir
                continue;
            }
            let from = args.data.temp_dir.join(add);
            let to = args.data.target_dir.join(add);

            if from.is_dir() {
                // if `to` is existing dir
                if to.is_dir() {
                    // can still move entire dir if is empty
                    let empty = match fs::read_dir(&to) {
                        Ok(mut d) => d.next().is_none(),
                        Err(_) => false,
                    };
                    if !empty {
                        // otherwise needs to merge per entry
                        continue;
                    }
                }

                // if `to` is existing file, remove it
                if let Err(e) = fs::remove_file(&to)
                    && !matches!(e.kind(), io::ErrorKind::NotFound)
                {
                    errors.push((to, e));
                    continue;
                }

                // move dir
                if let Err(e) = fs::rename(from, &to) {
                    errors.push((to, e));
                    continue;
                }

                // moved entire dir
                if add.as_os_str().is_empty() {
                    // already moved root dir
                    break;
                }
                moved_dirs.insert(add);
            } else if from.is_file() {
                // if is existing dir, remove it all
                if let Err(e) = fs::remove_dir_all(&to)
                    && !matches!(e.kind(), io::ErrorKind::NotADirectory)
                {
                    errors.push((to, e));
                    continue;
                }

                // move file
                if let Err(e) = fs::rename(from, &to) {
                    errors.push((to, e));
                }
            } else {
                errors.push((from, io::Error::new(io::ErrorKind::NotFound, "expected dir or file")));
            }
        }
        if errors.is_empty() {
            entries.reverse(); // uninstall removes depth first to cleanup empty dirs as it goes
            Ok(InstallData {
                target_dir: args.data.target_dir,
                entries,
            })
        } else {
            Err(SetupTaskError::Io(errors))
        }
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

        // verify order, uninstall must consume depth first
        data.entries.sort_by(|a, b| b.cmp(a));

        Ok(data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> super::Result<()> {
        let mut errors = vec![];
        for entry in args.data.entries {
            let entry = args.data.target_dir.join(entry);
            if entry.is_dir() {
                if let Err(e) = fs::remove_dir(&entry)
                    && !matches!(
                        e.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory | io::ErrorKind::DirectoryNotEmpty
                    )
                {
                    errors.push((entry, e));
                }
            } else if entry.is_file()
                && let Err(e) = fs::remove_file(&entry)
                && !matches!(e.kind(), io::ErrorKind::NotFound)
            {
                errors.push((entry, e));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(super::SetupTaskError::Io(errors))
        }
    }
}

const TEMP_STATE: &str = ".zng-setup-ExtractTar";

/// Config for [`ExtractTar`]
pub struct ExtractTarConfig {
    tar_len: u64,
    tar: Box<dyn io::Read + Send>,
    target_dir: PathBuf,
    strict: bool,
}

impl ExtractTarConfig {
    /// New config.
    ///
    /// * `tar_len` estimated length of `tar`, used for progress reporting only. Pass `0` for indeterminate.
    /// * `tar` must read only a TAR stream from start to end. Must contain only directory and file entries.
    /// * `target_dir` Directory that will be created or merged with the `tar` root directory.
    ///
    /// If all entries share a common first path component (for example, `my-app/bin/app.exe` and `my-app/README.md`),
    /// that directory is created inside `target_dir`. To extract files directly into `target_dir`, the entries must
    /// not have a common leading directory component.
    pub fn new(tar_len: u64, tar: Box<dyn io::Read + Send>, target_dir: PathBuf) -> Self {
        Self {
            tar_len,
            tar,
            target_dir,
            strict: false,
        }
    }

    /// Enable strict errors.
    ///
    /// When enabled:
    ///
    /// * Error on TAR entry that is not directory nor file.
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
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
