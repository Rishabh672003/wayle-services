use wayle_core::Property;

use crate::{Address, MonitorId, WorkspaceData};

/// A Hyprland workspace with reactive state.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Stable workspace identity (addressable name). Survives renames.
    pub address: Property<String>,
    /// Workspace kind: `numbered`, `named` or `special`.
    pub kind: Property<String>,
    /// Workspace display name.
    pub name: Property<String>,
    /// Monitor name this workspace is on.
    pub monitor: Property<String>,
    /// Monitor ID, if the workspace has an assigned monitor.
    pub monitor_id: Property<Option<MonitorId>>,
    /// Window count.
    pub windows: Property<u16>,
    /// Whether any window is fullscreen.
    pub fullscreen: Property<bool>,
    /// Address of the last focused window.
    pub last_window: Property<Option<Address>>,
    /// Title of the last focused window.
    pub last_window_title: Property<String>,
    /// Persistent workspace (survives having no windows).
    pub persistent: Property<bool>,
    /// Layout used for tiled windows on this workspace.
    pub tiled_layout: Property<String>,
}

impl PartialEq for Workspace {
    fn eq(&self, other: &Self) -> bool {
        self.address.get() == other.address.get()
    }
}

impl Workspace {
    pub(crate) fn from_props(workspace_data: WorkspaceData) -> Self {
        Self {
            address: Property::new(workspace_data.address),
            kind: Property::new(workspace_data.kind),
            name: Property::new(workspace_data.name),
            monitor: Property::new(workspace_data.monitor),
            monitor_id: Property::new(workspace_data.monitor_id),
            windows: Property::new(workspace_data.windows),
            fullscreen: Property::new(workspace_data.fullscreen),
            last_window: Property::new(workspace_data.last_window),
            last_window_title: Property::new(workspace_data.last_window_title),
            persistent: Property::new(workspace_data.persistent),
            tiled_layout: Property::new(workspace_data.tiled_layout),
        }
    }

    pub(crate) fn update(&self, workspace_data: WorkspaceData) {
        self.address.set(workspace_data.address);
        self.kind.set(workspace_data.kind);
        self.name.set(workspace_data.name);
        self.monitor.set(workspace_data.monitor);
        self.monitor_id.set(workspace_data.monitor_id);
        self.windows.set(workspace_data.windows);
        self.fullscreen.set(workspace_data.fullscreen);
        self.last_window.set(workspace_data.last_window);
        self.last_window_title.set(workspace_data.last_window_title);
        self.persistent.set(workspace_data.persistent);
        self.tiled_layout.set(workspace_data.tiled_layout);
    }
}
