//! API extension types.

use std::{fmt, ops};

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

/// Custom serialized data, in a format defined by the extension.
///
/// Note that the bytes here should represent a serialized small `struct` only, you
/// can add an [`IpcBytes`] field to this struct to transfer large payloads.
///
/// [`IpcBytes`]: zng_task::channel::IpcBytes
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensionPayload(#[serde(with = "serde_bytes")] pub Vec<u8>);
impl ApiExtensionPayload {
    /// Serialize the payload.
    pub fn serialize<T: Serialize>(payload: &T) -> Result<Self, bincode::Error> {
        bincode::serialize(payload).map(Self)
    }

    /// Deserialize the payload.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, ApiExtensionRecvError> {
        if let Some((id, error)) = self.parse_invalid_request() {
            Err(ApiExtensionRecvError::InvalidRequest {
                extension_id: id,
                error: Txt::from_str(error),
            })
        } else if let Some(id) = self.parse_unknown_extension() {
            Err(ApiExtensionRecvError::UnknownExtension { extension_id: id })
        } else {
            match bincode::deserialize::<T>(&self.0) {
                Ok(r) => Ok(r),
                Err(e) => Err(ApiExtensionRecvError::Deserialize(e)),
            }
        }
    }

    /// Empty payload.
    pub const fn empty() -> Self {
        Self(vec![])
    }

    /// Value returned when an invalid extension is requested.
    ///
    /// Value is a string `"zng-view-api.unknown_extension;id={extension_id}"`.
    pub fn unknown_extension(extension_id: ApiExtensionId) -> Self {
        Self(format!("zng-view-api.unknown_extension;id={extension_id}").into_bytes())
    }

    /// Value returned when an invalid request is made for a valid extension key.
    ///
    /// Value is a string `"zng-view-api.invalid_request;id={extension_id};error={error}"`.
    pub fn invalid_request(extension_id: ApiExtensionId, error: impl fmt::Display) -> Self {
        Self(format!("zng-view-api.invalid_request;id={extension_id};error={error}").into_bytes())
    }

    /// If the payload is an [`unknown_extension`] error message, returns the key.
    ///
    /// if the payload starts with the invalid request header and the key cannot be retrieved the
    /// [`ApiExtensionId::INVALID`] is returned as the key.
    ///
    /// [`unknown_extension`]: Self::unknown_extension
    pub fn parse_unknown_extension(&self) -> Option<ApiExtensionId> {
        let p = self.0.strip_prefix(b"zng-view-api.unknown_extension;")?;
        if let Some(p) = p.strip_prefix(b"id=")
            && let Ok(id_str) = std::str::from_utf8(p)
        {
            return match id_str.parse::<ApiExtensionId>() {
                Ok(id) => Some(id),
                Err(id) => Some(id),
            };
        }
        Some(ApiExtensionId::INVALID)
    }

    /// If the payload is an [`invalid_request`] error message, returns the key and error.
    ///
    /// if the payload starts with the invalid request header and the key cannot be retrieved the
    /// [`ApiExtensionId::INVALID`] is returned as the key and the error message will mention "corrupted payload".
    ///
    /// [`invalid_request`]: Self::invalid_request
    pub fn parse_invalid_request(&self) -> Option<(ApiExtensionId, &str)> {
        let p = self.0.strip_prefix(b"zng-view-api.invalid_request;")?;
        if let Some(p) = p.strip_prefix(b"id=")
            && let Some(id_end) = p.iter().position(|&b| b == b';')
            && let Ok(id_str) = std::str::from_utf8(&p[..id_end])
        {
            let id = match id_str.parse::<ApiExtensionId>() {
                Ok(id) => id,
                Err(id) => id,
            };
            if let Some(p) = p[id_end..].strip_prefix(b";error=")
                && let Ok(err_str) = std::str::from_utf8(p)
            {
                return Some((id, err_str));
            }
            return Some((id, "invalid request, corrupted payload, unknown error"));
        }
        Some((
            ApiExtensionId::INVALID,
            "invalid request, corrupted payload, unknown extension_id and error",
        ))
    }
}
impl fmt::Debug for ApiExtensionPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtensionPayload({} bytes)", self.0.len())
    }
}

/// Identifies an API extension and version.
///
/// Note that the version is part of the name, usually in the pattern "crate-name.extension.v2",
/// there are no minor versions, all different versions are considered breaking changes and
/// must be announced and supported by exact match only. You can still communicate non-breaking changes
/// by using the extension payload
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensionName {
    name: Txt,
}
impl ApiExtensionName {
    /// New from unique name.
    ///
    /// The name must contain at least 1 characters, and match the pattern `[a-zA-Z][a-zA-Z0-9-_.]`.
    pub fn new(name: impl Into<Txt>) -> Result<Self, ApiExtensionNameError> {
        let name = name.into();
        Self::new_impl(name)
    }
    fn new_impl(name: Txt) -> Result<ApiExtensionName, ApiExtensionNameError> {
        if name.is_empty() {
            return Err(ApiExtensionNameError::NameCannotBeEmpty);
        }
        for (i, c) in name.char_indices() {
            if i == 0 {
                if !c.is_ascii_alphabetic() {
                    return Err(ApiExtensionNameError::NameCannotStartWithChar(c));
                }
            } else if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' {
                return Err(ApiExtensionNameError::NameInvalidChar(c));
            }
        }

        Ok(Self { name })
    }
}
impl fmt::Debug for ApiExtensionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.name, f)
    }
}
impl fmt::Display for ApiExtensionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}
impl ops::Deref for ApiExtensionName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name.as_str()
    }
}
impl From<&'static str> for ApiExtensionName {
    fn from(value: &'static str) -> Self {
        Self::new(value).unwrap()
    }
}

/// API extension invalid name.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ApiExtensionNameError {
    /// Name cannot empty `""`.
    NameCannotBeEmpty,
    /// Name can only start with ASCII alphabetic chars `[a-zA-Z]`.
    NameCannotStartWithChar(char),
    /// Name can only contains `[a-zA-Z0-9-_.]`.
    NameInvalidChar(char),
}
impl fmt::Display for ApiExtensionNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiExtensionNameError::NameCannotBeEmpty => write!(f, "API extension name cannot be empty"),
            ApiExtensionNameError::NameCannotStartWithChar(c) => {
                write!(f, "API cannot start with '{c}', name pattern `[a-zA-Z][a-zA-Z0-9-_.]`")
            }
            ApiExtensionNameError::NameInvalidChar(c) => write!(f, "API cannot contain '{c}', name pattern `[a-zA-Z][a-zA-Z0-9-_.]`"),
        }
    }
}
impl std::error::Error for ApiExtensionNameError {}

/// List of available API extensions.
#[derive(Default, Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensions(Vec<ApiExtensionName>);
impl ops::Deref for ApiExtensions {
    type Target = [ApiExtensionName];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ApiExtensions {
    /// New Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the position of the `ext` in the list of available extensions. This index
    /// identifies the API extension in the [`Api::app_extension`] and [`Api::render_extension`].
    ///
    /// The key can be cached only for the duration of the view process, each view re-instantiation
    /// must query for the presence of the API extension again, and it may change position on the list.
    ///
    /// [`Api::app_extension`]: crate::Api::app_extension
    /// [`Api::render_extension`]: crate::Api::render_extension
    pub fn id(&self, ext: &ApiExtensionName) -> Option<ApiExtensionId> {
        self.0.iter().position(|e| e == ext).map(ApiExtensionId::from_index)
    }

    /// Push the `ext` to the list, if it is not already inserted.
    ///
    /// Returns `Ok(key)` if inserted or `Err(key)` is was already in list.
    pub fn insert(&mut self, ext: ApiExtensionName) -> Result<ApiExtensionId, ApiExtensionId> {
        if let Some(key) = self.id(&ext) {
            Err(key)
        } else {
            let key = self.0.len();
            self.0.push(ext);
            Ok(ApiExtensionId::from_index(key))
        }
    }
}

/// Identifies an [`ApiExtensionName`] in a list.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiExtensionId(u32);
impl fmt::Debug for ApiExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::INVALID {
            if f.alternate() {
                write!(f, "ApiExtensionId::")?;
            }
            write!(f, "INVALID")
        } else {
            write!(f, "ApiExtensionId({})", self.0 - 1)
        }
    }
}
impl fmt::Display for ApiExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::INVALID {
            write!(f, "invalid")
        } else {
            write!(f, "{}", self.0 - 1)
        }
    }
}
impl ApiExtensionId {
    /// Dummy ID.
    pub const INVALID: Self = Self(0);

    /// Gets the ID as a list index.
    ///
    /// # Panics
    ///
    /// Panics if called in `INVALID`.
    pub fn index(self) -> usize {
        self.0.checked_sub(1).expect("invalid id") as _
    }

    /// New ID from the index of an [`ApiExtensionName`] in a list.
    ///
    /// # Panics
    ///
    /// Panics if `idx > u32::MAX - 1`.
    pub fn from_index(idx: usize) -> Self {
        if idx > (u32::MAX - 1) as _ {
            panic!("index out-of-bounds")
        }
        Self(idx as u32 + 1)
    }
}
impl std::str::FromStr for ApiExtensionId {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u32>() {
            Ok(i) => {
                let r = Self::from_index(i as _);
                if r == Self::INVALID { Err(r) } else { Ok(r) }
            }
            Err(_) => Err(Self::INVALID),
        }
    }
}

/// Error in the response of an API extension call.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApiExtensionRecvError {
    /// Requested extension was not in the list of extensions.
    UnknownExtension {
        /// Extension that was requested.
        ///
        /// Is `INVALID` only if error message is corrupted.
        extension_id: ApiExtensionId,
    },
    /// Invalid request format.
    InvalidRequest {
        /// Extension that was requested.
        ///
        /// Is `INVALID` only if error message is corrupted.
        extension_id: ApiExtensionId,
        /// Message from the view-process.
        error: Txt,
    },
    /// Failed to deserialize to the expected response type.
    Deserialize(bincode::Error),
}
impl fmt::Display for ApiExtensionRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiExtensionRecvError::UnknownExtension { extension_id } => write!(f, "invalid API request for unknown id {extension_id:?}"),
            ApiExtensionRecvError::InvalidRequest { extension_id, error } => {
                write!(f, "invalid API request for extension id {extension_id:?}, {error}")
            }
            ApiExtensionRecvError::Deserialize(e) => write!(f, "API extension response failed to deserialize, {e}"),
        }
    }
}
impl std::error::Error for ApiExtensionRecvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Deserialize(e) = self { Some(e) } else { None }
    }
}
