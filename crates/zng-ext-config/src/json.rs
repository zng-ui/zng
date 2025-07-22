use zng_ext_fs_watcher::{WatchFile, WriteFile};

use super::*;

/// Represents a config source that synchronizes with a JSON file.
pub type JsonConfig = SyncConfig<JsonBackend>;

#[doc(hidden)]
pub struct JsonBackend;
impl SyncConfigBackend for JsonBackend {
    fn read(mut file: WatchFile) -> io::Result<RawConfigMap> {
        file.json().map_err(Into::into)
    }

    fn write(file: &mut WriteFile, map: &RawConfigMap) -> io::Result<()> {
        file.write_json(map, true)
    }
}
