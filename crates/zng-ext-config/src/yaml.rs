use super::*;

/// Represents a config source that synchronizes with a YAML file.
pub type YamlConfig = SyncConfig<YamlBackend>;

#[doc(hidden)]
pub struct YamlBackend;
impl SyncConfigBackend for YamlBackend {
    fn read(mut file: WatchFile) -> io::Result<RawConfigMap> {
        file.yaml().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn write(file: &mut WriteFile, map: &RawConfigMap) -> io::Result<()> {
        file.write_yaml(map)
    }
}
