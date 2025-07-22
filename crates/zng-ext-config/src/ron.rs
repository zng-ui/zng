use super::*;

/// Represents a config source that synchronizes with a RON file.
pub type RonConfig = SyncConfig<RonBackend>;

#[doc(hidden)]
pub struct RonBackend;
impl SyncConfigBackend for RonBackend {
    fn read(mut file: WatchFile) -> io::Result<RawConfigMap> {
        file.ron().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn write(file: &mut WriteFile, map: &RawConfigMap) -> io::Result<()> {
        file.write_ron(map, true)
    }
}
