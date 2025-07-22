use super::*;

/// Represents a config source that synchronizes with a TOML file.
pub type TomlConfig = SyncConfig<TomlBackend>;

#[doc(hidden)]
pub struct TomlBackend;
impl SyncConfigBackend for TomlBackend {
    fn read(mut file: WatchFile) -> io::Result<RawConfigMap> {
        file.toml()
    }

    fn write(file: &mut WriteFile, map: &RawConfigMap) -> io::Result<()> {
        file.write_toml(map, true)
    }
}
