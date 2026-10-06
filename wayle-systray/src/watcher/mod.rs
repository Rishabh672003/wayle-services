#![allow(missing_docs)]
pub(crate) mod discovery;
mod monitoring;

use std::sync::Arc;

use derive_more::Debug;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use wayle_traits::ServiceMonitoring;
use zbus::{
    Connection,
    fdo::{self, DBusProxy},
    message::Header,
    names::BusName,
    object_server::SignalEmitter,
};

use super::{
    error::Error,
    events::TrayEvent,
    types::{ITEM_OBJECT_PATH, PROTOCOL_VERSION, WATCHER_INTERFACE, WATCHER_OBJECT_PATH},
};

#[derive(Debug)]
pub(crate) struct StatusNotifierWatcher {
    #[debug(skip)]
    pub zbus_connection: Connection,
    #[debug(skip)]
    pub event_tx: broadcast::Sender<TrayEvent>,
    #[debug(skip)]
    pub cancellation_token: CancellationToken,

    pub registered_items: Arc<RwLock<Vec<String>>>,
    pub registered_hosts: Arc<RwLock<Vec<String>>>,
}

pub(crate) async fn register_item(
    service: &str,
    registered_items: &Arc<RwLock<Vec<String>>>,
    event_tx: &broadcast::Sender<TrayEvent>,
    connection: &Connection,
) -> bool {
    let service = service.to_string();

    // The same object can be reachable under a well-known name and the owner's
    // unique name (e.g. orphan scan vs. explicit registration). Keep one entry,
    // preferring the well-known name so its release still unregisters the item.
    if let Some(existing) = find_same_object(&service, registered_items, connection).await {
        if service.starts_with(':') || !existing.starts_with(':') {
            return false;
        }
        let _ =
            monitoring::unregister_item(&existing, registered_items, event_tx, connection).await;
    }

    {
        let mut items = registered_items.write().await;
        if items.contains(&service) {
            return false;
        }
        items.push(service.clone());
    }

    let _ = event_tx.send(TrayEvent::ItemRegistered(service.clone()));

    connection
        .emit_signal(
            None::<()>,
            WATCHER_OBJECT_PATH,
            WATCHER_INTERFACE,
            "StatusNotifierItemRegistered",
            &service,
        )
        .await
        .unwrap_or_else(|err| {
            error!(error = %err, service = %service, "cannot emit item registered signal");
        });

    true
}

/// Returns a registered entry that points at the same (owner, path) as `service`.
async fn find_same_object(
    service: &str,
    registered_items: &Arc<RwLock<Vec<String>>>,
    connection: &Connection,
) -> Option<String> {
    let dbus_proxy = DBusProxy::new(connection).await.ok()?;
    let target = resolve_object(&dbus_proxy, service).await?;
    let existing = registered_items.read().await.clone();

    for entry in existing {
        if entry != service && resolve_object(&dbus_proxy, &entry).await.as_ref() == Some(&target) {
            return Some(entry);
        }
    }
    None
}

/// Splits a registration (`name` or `name/path`) and resolves `name` to its unique owner.
async fn resolve_object(dbus_proxy: &DBusProxy<'_>, service: &str) -> Option<(String, String)> {
    let (name, path) = split_service(service);
    if name.starts_with(':') {
        return Some((name.to_string(), path.to_string()));
    }
    let owner = dbus_proxy
        .get_name_owner(BusName::try_from(name).ok()?)
        .await
        .ok()?;
    Some((owner.to_string(), path.to_string()))
}

fn split_service(service: &str) -> (&str, &str) {
    match service.find('/') {
        Some(index) => service.split_at(index),
        None => (service, ITEM_OBJECT_PATH),
    }
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    #[instrument(skip(self, _ctx, header), fields(service = %service))]
    async fn register_status_notifier_item(
        &mut self,
        #[zbus(signal_context)] _ctx: SignalEmitter<'_>,
        #[zbus(header)] header: Header<'_>,
        service: String,
    ) -> fdo::Result<()> {
        let full_service = if service.starts_with('/') {
            let sender = header
                .sender()
                .ok_or_else(|| fdo::Error::Failed("No sender in D-Bus message header".into()))?;
            format!("{sender}{service}")
        } else {
            service
        };

        info!(service = %full_service, "registering StatusNotifierItem");

        register_item(
            &full_service,
            &self.registered_items,
            &self.event_tx,
            &self.zbus_connection,
        )
        .await;

        Ok(())
    }

    #[instrument(skip(self, ctx), fields(service = %service))]
    async fn register_status_notifier_host(
        &mut self,
        #[zbus(signal_context)] ctx: SignalEmitter<'_>,
        service: String,
    ) -> fdo::Result<()> {
        info!(service = %service, "registering StatusNotifierHost");

        let should_signal = {
            let mut hosts = self.registered_hosts.write().await;
            let was_empty = hosts.is_empty();

            if hosts.contains(&service) {
                false
            } else {
                hosts.push(service.clone());
                was_empty
            }
        };

        if should_signal {
            Self::status_notifier_host_registered(&ctx).await?;
        }

        Ok(())
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registered_items.read().await.clone()
    }

    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        !self.registered_hosts.read().await.is_empty()
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        PROTOCOL_VERSION
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        ctx: &SignalEmitter<'_>,
        service: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        ctx: &SignalEmitter<'_>,
        service: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(ctx: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(ctx: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl StatusNotifierWatcher {
    pub(crate) async fn with_initial_host(
        event_tx: broadcast::Sender<TrayEvent>,
        connection: &Connection,
        cancellation_token: &CancellationToken,
        initial_host: String,
    ) -> Result<Self, Error> {
        let registered_items = Arc::new(RwLock::new(Vec::new()));
        let registered_hosts = Arc::new(RwLock::new(vec![initial_host]));

        let watcher = Self {
            zbus_connection: connection.clone(),
            event_tx,
            cancellation_token: cancellation_token.clone(),
            registered_items,
            registered_hosts,
        };

        watcher.start_monitoring().await?;

        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_service_defaults_to_item_path() {
        assert_eq!(split_service(":1.5"), (":1.5", ITEM_OBJECT_PATH));
        assert_eq!(
            split_service(":1.5/org/ayatana/NotificationItem/x"),
            (":1.5", "/org/ayatana/NotificationItem/x")
        );
        assert_eq!(
            split_service("org.kde.StatusNotifierItem-42-1"),
            ("org.kde.StatusNotifierItem-42-1", ITEM_OBJECT_PATH)
        );
    }
}
