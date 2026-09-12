use tokio::sync::broadcast::Sender;

use crate::{Error, HyprlandEvent, Result};

pub(crate) fn handle_workspace(data: &str, hyprland_tx: Sender<HyprlandEvent>) -> Result<()> {
    hyprland_tx.send(HyprlandEvent::Workspace {
        name: data.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_workspace_v2(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let Some((address, name)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data,
            field: "workspace_data",
            expected: "comma-separated address,name",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::WorkspaceV2 {
        address: address.to_string(),
        name: name.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_create_workspace(
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    hyprland_tx.send(HyprlandEvent::CreateWorkspace {
        name: data.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_create_workspace_v2(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let Some((address, name)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data,
            field: "workspace_data",
            expected: "comma-separated address,name",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::CreateWorkspaceV2 {
        address: address.to_string(),
        name: name.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_destroy_workspace(
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    hyprland_tx.send(HyprlandEvent::DestroyWorkspace {
        name: data.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_destroy_workspace_v2(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let Some((address, name)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data,
            field: "workspace_data",
            expected: "comma-separated address,name",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::DestroyWorkspaceV2 {
        address: address.to_string(),
        name: name.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_move_workspace(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let Some((name, monitor)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data: format!("{event}>>{data}"),
            field: "workspace_data",
            expected: "comma-separated name,monitor",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::MoveWorkspace {
        name: name.to_string(),
        monitor: monitor.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_move_workspace_v2(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let parts: Vec<&str> = data.split(',').collect();
    let [address, name, monitor] = parts.as_slice() else {
        return Err(Error::EventParseError {
            event_data,
            field: "workspace_data",
            expected: "3 comma-separated values (address,name,monitor)",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::MoveWorkspaceV2 {
        address: (*address).to_string(),
        name: (*name).to_string(),
        monitor: (*monitor).to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_rename_workspace(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let Some((address, new_name)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data,
            field: "workspace_data",
            expected: "comma-separated address,new_name",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::RenameWorkspace {
        address: address.to_string(),
        new_name: new_name.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_active_special(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let Some((workspace, monitor)) = data.split_once(',') else {
        return Err(Error::EventParseError {
            event_data: format!("{event}>>{data}"),
            field: "special_workspace_data",
            expected: "comma-separated workspace,monitor",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::ActiveSpecial {
        workspace: workspace.to_string(),
        monitor: monitor.to_string(),
    })?;

    Ok(())
}

pub(crate) fn handle_active_special_v2(
    event: &str,
    data: &str,
    hyprland_tx: Sender<HyprlandEvent>,
) -> Result<()> {
    let event_data = format!("{event}>>{data}");
    let parts: Vec<&str> = data.split(',').collect();
    let [address, workspace, monitor] = parts.as_slice() else {
        return Err(Error::EventParseError {
            event_data,
            field: "special_workspace_data",
            expected: "3 comma-separated values (address,workspace,monitor)",
            value: data.to_string(),
        });
    };

    hyprland_tx.send(HyprlandEvent::ActiveSpecialV2 {
        address: (*address).to_string(),
        workspace: (*workspace).to_string(),
        monitor: (*monitor).to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast;

    use super::*;

    fn channel() -> (Sender<HyprlandEvent>, broadcast::Receiver<HyprlandEvent>) {
        broadcast::channel(1)
    }

    #[test]
    fn workspace_v2_parses_named_selector() {
        let (tx, mut rx) = channel();
        handle_workspace_v2("workspacev2", "name:foo,bar", tx).unwrap();

        assert!(matches!(
            rx.try_recv().unwrap(),
            HyprlandEvent::WorkspaceV2 { ref address, ref name }
                if address == "name:foo" && name == "bar"
        ));
    }

    #[test]
    fn workspace_v2_parses_legacy_numeric_selector() {
        let (tx, mut rx) = channel();
        handle_workspace_v2("workspacev2", "2,2", tx).unwrap();

        assert!(matches!(
            rx.try_recv().unwrap(),
            HyprlandEvent::WorkspaceV2 { ref address, ref name }
                if address == "2" && name == "2"
        ));
    }

    #[test]
    fn active_special_v2_parses_special_selector() {
        let (tx, mut rx) = channel();
        handle_active_special_v2("activespecialv2", "special:foo,,DP-1", tx).unwrap();

        assert!(matches!(
            rx.try_recv().unwrap(),
            HyprlandEvent::ActiveSpecialV2 { ref address, ref workspace, ref monitor }
                if address == "special:foo" && workspace.is_empty() && monitor == "DP-1"
        ));
    }

    #[test]
    fn rename_workspace_parses_address_and_name() {
        let (tx, mut rx) = channel();
        handle_rename_workspace("renameworkspace>>", "foo,My Workspace", tx).unwrap();

        assert!(matches!(
            rx.try_recv().unwrap(),
            HyprlandEvent::RenameWorkspace { ref address, ref new_name }
                if address == "foo" && new_name == "My Workspace"
        ));
    }

    #[test]
    fn move_workspace_v2_parses_three_fields() {
        let (tx, mut rx) = channel();
        handle_move_workspace_v2("moveworkspacev2", "special:foo,special,DP-1", tx).unwrap();

        assert!(matches!(
            rx.try_recv().unwrap(),
            HyprlandEvent::MoveWorkspaceV2 { ref address, ref name, ref monitor }
                if address == "special:foo" && name == "special" && monitor == "DP-1"
        ));
    }
}
