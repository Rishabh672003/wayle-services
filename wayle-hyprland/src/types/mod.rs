mod bind;
mod client;
mod device;
mod layer;
mod monitor;
mod workspace;

use std::fmt::{self, Display};

pub use bind::*;
pub use client::*;
pub use device::*;
pub use layer::*;
pub use monitor::*;
use serde::{Deserialize, Deserializer};
pub(crate) use workspace::WorkspaceData;
pub use workspace::WorkspaceRule;

use crate::Error;

/// Unique identifier for a monitor.
pub type MonitorId = i64;
/// Legacy numeric workspace identifier.
///
/// Hyprland 0.57+ replaced numeric IDs exposed over IPC with stable
/// addressable names
/// This alias is kept only to parse older IPC payloads
pub type WorkspaceId = i64;
/// Process identifier.
pub type ProcessId = i32;
/// Focus history identifier.
pub type FocusHistoryId = i32;

/// The type of screencopy share.
///
/// Also accepts `"0"`, `"1"`, `"2"` for older Hyprland builds that
/// emitted raw integers before the string formatter was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreencastOwner {
    /// Monitor share.
    Monitor,
    /// Window share.
    Window,
    /// Region share.
    Region,
}

impl TryFrom<&str> for ScreencastOwner {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "monitor" | "0" => Ok(Self::Monitor),
            "window" | "1" => Ok(Self::Window),
            "region" | "2" => Ok(Self::Region),
            _ => Err(Error::InvalidEnumValue {
                type_name: "ScreencastOwner",
                value: value.to_string(),
            }),
        }
    }
}

/// Workspace information attached to monitors and clients.
///
/// Hyprland 0.57+ reports `{address, type, name}` where `address` is a stable
/// identity that survives renames. Older builds reported `{id, name}`; both
/// shapes are accepted and normalized onto `address`/`kind`.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct WorkspaceInfo {
    /// Stable workspace identity (addressable name).
    pub address: String,
    /// Workspace kind: `numbered`, `named` or `special`.
    pub kind: String,
    /// Workspace display name.
    pub name: String,
}

impl<'de> Deserialize<'de> for WorkspaceInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            #[serde(default)]
            address: Option<String>,
            #[serde(default)]
            #[allow(non_snake_case)]
            id: Option<WorkspaceId>,
            #[serde(default, rename = "type")]
            kind: Option<String>,
            name: String,
        }

        let raw = Raw::deserialize(deserializer)?;
        let address = raw
            .address
            .clone()
            .or_else(|| raw.id.map(|id| id.to_string()))
            .unwrap_or_default();
        let kind = match raw.kind {
            Some(kind) => kind,
            None => infer_workspace_kind(raw.id, raw.address.as_deref()).to_string(),
        };
        Ok(Self {
            address,
            kind,
            name: raw.name,
        })
    }
}

fn infer_workspace_kind(id: Option<WorkspaceId>, address: Option<&str>) -> &'static str {
    if id.is_some_and(|id| id < 0) || address.is_some_and(|a| a.starts_with("special")) {
        "special"
    } else {
        "numbered"
    }
}

/// Window address identifier.
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(from = "String")]
pub struct Address(String);

impl Address {
    /// Creates a new address from a string.
    pub fn new(address: String) -> Self {
        let normalized = address.strip_prefix("0x").unwrap_or(&address).to_string();
        Self(normalized)
    }

    /// Returns the address as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the address and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

pub(crate) fn deserialize_optional_address<'de, D>(
    deserializer: D,
) -> Result<Option<Address>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s == "0" || s == "0x0" || s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Address::new(s)))
    }
}

pub(crate) fn deserialize_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
}

/// Deserializes a workspace `monitorID` that Hyprland emits either as a JSON
/// number (older builds), a string (`"0"` / `"null"`, 0.57+) or `null`.
pub(crate) fn deserialize_optional_monitor_id<'de, D>(
    deserializer: D,
) -> Result<Option<MonitorId>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(num)) => num
            .as_i64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("invalid monitor id")),
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() || s == "null" {
                Ok(None)
            } else {
                s.trim().parse().map(Some).map_err(serde::de::Error::custom)
            }
        }
        Some(_) => Err(serde::de::Error::custom("invalid monitor id type")),
    }
}

/// Cursor position in global layout coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    /// The x-coordinate of the cursor
    pub x: i32,
    /// The y-coordinate of the cursor
    pub y: i32,
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[test]
    fn address_new_strips_0x_prefix() {
        let address = Address::new("0xdeadbeef".to_string());

        assert_eq!(address.as_str(), "deadbeef");
    }

    #[test]
    fn address_new_preserves_address_without_prefix() {
        let address = Address::new("deadbeef".to_string());

        assert_eq!(address.as_str(), "deadbeef");
    }

    #[test]
    fn screencast_owner_try_from_converts_monitor() {
        let result = ScreencastOwner::try_from("0");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ScreencastOwner::Monitor);
    }

    #[test]
    fn screencast_owner_try_from_converts_window() {
        let result = ScreencastOwner::try_from("1");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ScreencastOwner::Window);
    }

    #[test]
    fn screencast_owner_try_from_region() {
        let result = ScreencastOwner::try_from("2");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ScreencastOwner::Region);
    }

    #[test]
    fn screencast_owner_try_from_fails_for_invalid_value() {
        let result = ScreencastOwner::try_from("99");

        assert!(result.is_err());
        let error = result.unwrap_err();
        if let Error::InvalidEnumValue { type_name, value } = error {
            assert_eq!(type_name, "ScreencastOwner");
            assert_eq!(value, "99");
        } else {
            panic!("Expected InvalidEnumValue error");
        }
    }

    #[test]
    fn deserialize_optional_address_returns_none_for_zero() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_address")]
            address: Option<Address>,
        }

        let json = r#"{"address": "0"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.address.is_none());
    }

    #[test]
    fn deserialize_optional_address_returns_none_for_empty() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_address")]
            address: Option<Address>,
        }

        let json = r#"{"address": ""}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.address.is_none());
    }

    #[test]
    fn deserialize_optional_address_returns_some_for_valid() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_address")]
            address: Option<Address>,
        }

        let json = r#"{"address": "0xdeadbeef"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.address.is_some());
        assert_eq!(result.address.unwrap().as_str(), "deadbeef");
    }

    #[test]
    fn deserialize_optional_string_returns_none_for_empty() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_string")]
            value: Option<String>,
        }

        let json = r#"{"value": ""}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.value.is_none());
    }

    #[test]
    fn deserialize_optional_string_returns_some_for_non_empty() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_string")]
            value: Option<String>,
        }

        let json = r#"{"value": "test"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.value.is_some());
        assert_eq!(result.value.unwrap(), "test");
    }

    #[test]
    fn deserialize_optional_address_returns_none_for_0x0() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_address")]
            address: Option<Address>,
        }

        let json = r#"{"address": "0x0"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();

        assert!(result.address.is_none());
    }

    #[test]
    fn workspace_info_parses_new_address_shape() {
        let info: WorkspaceInfo =
            serde_json::from_str(r#"{"address": "hello", "type": "named", "name": "world"}"#)
                .unwrap();

        assert_eq!(info.address, "hello");
        assert_eq!(info.kind, "named");
        assert_eq!(info.name, "world");
    }

    #[test]
    fn workspace_info_parses_legacy_id_shape() {
        let info: WorkspaceInfo = serde_json::from_str(r#"{"id": 3, "name": "3"}"#).unwrap();

        assert_eq!(info.address, "3");
        assert_eq!(info.kind, "numbered");
        assert_eq!(info.name, "3");
    }

    #[test]
    fn workspace_info_infers_special_kind_from_legacy_negative_id() {
        let info: WorkspaceInfo =
            serde_json::from_str(r#"{"id": -98, "name": "special"}"#).unwrap();

        assert_eq!(info.address, "-98");
        assert_eq!(info.kind, "special");
    }

    #[test]
    fn monitor_id_deserializes_string_number_and_null() {
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_optional_monitor_id")]
            monitor_id: Option<MonitorId>,
        }

        let from_string: TestStruct = serde_json::from_str(r#"{"monitor_id": "0"}"#).unwrap();
        assert_eq!(from_string.monitor_id, Some(0));

        let from_null_string: TestStruct =
            serde_json::from_str(r#"{"monitor_id": "null"}"#).unwrap();
        assert!(from_null_string.monitor_id.is_none());

        let from_number: TestStruct = serde_json::from_str(r#"{"monitor_id": 1}"#).unwrap();
        assert_eq!(from_number.monitor_id, Some(1));

        let from_null: TestStruct = serde_json::from_str(r#"{"monitor_id": null}"#).unwrap();
        assert!(from_null.monitor_id.is_none());
    }
}
