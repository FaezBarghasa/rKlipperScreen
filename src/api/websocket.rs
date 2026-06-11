use crate::RKlipperScreen;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use slint::Weak;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

pub async fn connect_to_moonraker(ui_handle: Weak<RKlipperScreen>) {
    let url = match Url::parse("ws://127.0.0.1:7125/websocket") {
        Ok(u) => u,
        Err(_) => return,
    };

    // Implements reconnection handling to ensure production robustness
    loop {
        let (ws_stream, _) = match connect_async(url.as_str()).await {
            Ok(stream) => stream,
            Err(_) => {
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };

        let (mut write, mut read) = ws_stream.split();

        let subscribe_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.subscribe",
            "params": {
                "objects": {
                    "extruder": ["temperature"],
                    "heater_bed": ["temperature"]
                }
            },
            "id": 1
        });

        if write.send(Message::Text(subscribe_msg.to_string().into())).await.is_err() {
            sleep(Duration::from_secs(3)).await;
            continue;
        }

        while let Some(msg) = read.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    if json["method"].as_str() == Some("notify_status_update") {
                        if let Some(params) = json["params"].get(0) {
                            if let Some(extruder) = params.get("extruder") {
                                if let Some(temp) = extruder.get("temperature") {
                                    let temp_str = temp.to_string();
                                    let ui_clone = ui_handle.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_clone.upgrade() {
                                            ui.set_hotend_temp(slint::SharedString::from(temp_str));
                                        }
                                    });
                                }
                            }
                            if let Some(heater_bed) = params.get("heater_bed") {
                                if let Some(temp) = heater_bed.get("temperature") {
                                    let temp_str = temp.to_string();
                                    let ui_clone = ui_handle.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_clone.upgrade() {
                                            ui.set_bed_temp(slint::SharedString::from(temp_str));
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}