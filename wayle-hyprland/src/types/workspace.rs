use serde::{Deserialize, Deserializer};

use crate::{
    Address, MonitorId, WorkspaceId, deserialize_optional_address, deserialize_optional_monitor_id,
};

/// A workspace rule from Hyprland configuration.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRule {
    /// The workspace identifier string (could be name or ID).
    pub workspace_string: String,
    /// The monitor this workspace is bound to, if specified.
    #[serde(default)]
    pub monitor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkspaceData {
    /// Stable workspace identity (addressable name).
    pub address: String,
    /// Workspace kind: `numbered`, `named` or `special`.
    pub kind: String,
    pub name: String,
    pub monitor: String,
    /// Monitor ID, if the workspace has an assigned monitor.
    pub monitor_id: Option<MonitorId>,
    pub windows: u16,
    pub fullscreen: bool,
    pub last_window: Option<Address>,
    pub last_window_title: String,
    pub persistent: bool,
    pub tiled_layout: String,
}

impl<'de> Deserialize<'de> for WorkspaceData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            address: Option<String>,
            #[serde(default)]
            #[allow(non_snake_case)]
            id: Option<WorkspaceId>,
            #[serde(default, rename = "type")]
            kind: Option<String>,
            name: String,
            #[serde(default)]
            monitor: String,
            #[serde(
                default,
                rename = "monitorID",
                deserialize_with = "deserialize_optional_monitor_id"
            )]
            monitor_id: Option<MonitorId>,
            #[serde(default)]
            windows: u16,
            #[serde(default, rename = "hasfullscreen")]
            fullscreen: bool,
            #[serde(
                default,
                rename = "lastwindow",
                deserialize_with = "deserialize_optional_address"
            )]
            last_window: Option<Address>,
            #[serde(default, rename = "lastwindowtitle")]
            last_window_title: String,
            #[serde(default, rename = "ispersistent")]
            persistent: bool,
            #[serde(default, rename = "tiledLayout")]
            tiled_layout: String,
        }

        let raw = Raw::deserialize(deserializer)?;
        let address = raw
            .address
            .or_else(|| raw.id.map(|id| id.to_string()))
            .unwrap_or_default();
        let kind = raw.kind.unwrap_or_else(|| {
            if raw.id.is_some_and(|id| id < 0) || address.starts_with("special") {
                "special".to_string()
            } else {
                "numbered".to_string()
            }
        });

        Ok(Self {
            address,
            kind,
            name: raw.name,
            monitor: raw.monitor,
            monitor_id: raw.monitor_id,
            windows: raw.windows,
            fullscreen: raw.fullscreen,
            last_window: raw.last_window,
            last_window_title: raw.last_window_title,
            persistent: raw.persistent,
            tiled_layout: raw.tiled_layout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> WorkspaceData {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn workspace_data_parses_new_address_shape() {
        let data = parse(
            r#"{
                "address": "hello",
                "type": "named",
                "name": "world",
                "monitor": "DP-1",
                "monitorID": "0",
                "windows": 2,
                "hasfullscreen": false,
                "lastwindow": "0x0",
                "lastwindowtitle": "",
                "ispersistent": false,
                "tiledLayout": "dwindle"
            }"#,
        );

        assert_eq!(data.address, "hello");
        assert_eq!(data.kind, "named");
        assert_eq!(data.name, "world");
        assert_eq!(data.monitor_id, Some(0));
        assert_eq!(data.windows, 2);
        assert!(data.last_window.is_none());
    }

    #[test]
    fn workspace_data_parses_monitor_id_null_string() {
        let data = parse(
            r#"{
                "address": "special:magic",
                "type": "special",
                "name": "magic",
                "monitor": "?",
                "monitorID": "null",
                "windows": 0,
                "hasfullscreen": false,
                "lastwindow": "0x0",
                "lastwindowtitle": "",
                "ispersistent": true,
                "tiledLayout": "dwindle"
            }"#,
        );

        assert_eq!(data.kind, "special");
        assert!(data.monitor_id.is_none());
        assert!(data.persistent);
    }

    #[test]
    fn workspace_data_parses_legacy_id_shape() {
        let data = parse(
            r#"{
                "id": 3,
                "name": "3",
                "monitor": "HDMI-A-1",
                "monitorID": 1,
                "windows": 1,
                "hasfullscreen": true,
                "lastwindow": "0x123",
                "lastwindowtitle": "foo",
                "ispersistent": false,
                "tiledLayout": "master"
            }"#,
        );

        assert_eq!(data.address, "3");
        assert_eq!(data.kind, "numbered");
        assert_eq!(data.monitor_id, Some(1));
        assert!(data.fullscreen);
        assert!(data.last_window.is_some());
        assert_eq!(data.tiled_layout, "master");
    }

    #[test]
    fn workspace_data_infers_special_kind_from_legacy_negative_id() {
        let data = parse(
            r#"{
                "id": -98,
                "name": "special",
                "monitor": "?",
                "monitorID": null,
                "windows": 0,
                "hasfullscreen": false,
                "lastwindow": "",
                "lastwindowtitle": "",
                "ispersistent": false,
                "tiledLayout": "dwindle"
            }"#,
        );

        assert_eq!(data.address, "-98");
        assert_eq!(data.kind, "special");
        assert!(data.monitor_id.is_none());
    }
}
