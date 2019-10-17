use super::{FontInstanceKey, FontKey};
use fnv::FnvHashMap;

pub(crate) struct FontInstances {
    pub font_key: FontKey,
    pub instances: FnvHashMap<u32, FontInstanceKey>,
}

#[derive(Clone)]
pub struct FontInstance {
    pub font_key: FontKey,
    pub instance_key: FontInstanceKey,
    pub size: u32,
}
