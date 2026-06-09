use tracing::{info, warn};
use zbus::Connection;

pub async fn send_system_notification(summary: &str, body: &str) -> Result<(), zbus::Error> {
    info!("Sending system notification: {} - {}", summary, body);

    // Connect to session DBus for notifications
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(err) => {
            warn!(
                "Could not connect to Session DBus for notifications: {}",
                err
            );
            return Ok(());
        }
    };

    // DBus interface org.freedesktop.Notifications
    // Method Notify(app_name: String, replaces_id: u32, app_icon: String, summary: String, body: String, actions: Vec<String>, hints: Map<String, Value>, expire_timeout: i32) -> u32
    let app_name = "rKlipperScreen";
    let replaces_id = 0u32;
    let app_icon = "printer";
    let actions: Vec<String> = vec![];
    let hints: std::collections::HashMap<String, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    let expire_timeout = -1i32; // Default expiration

    let _reply: u32 = conn
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                app_name,
                replaces_id,
                app_icon,
                summary,
                body,
                &actions,
                &hints,
                expire_timeout,
            ),
        )
        .await?
        .body()
        .deserialize()?;

    Ok(())
}
