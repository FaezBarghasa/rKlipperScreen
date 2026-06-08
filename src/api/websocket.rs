use crate::printer::state::{reduce_printer_state, PrinterState};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

pub enum MoonrakerCommand {
    SendGcode(String),
    EmergencyStop,
    PausePrint,
    ResumePrint,
    CancelPrint,
    StartPrint(String),
}

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

pub fn spawn_moonraker_client(
    host: String,
    port: u16,
    _api_key: String,
    state_notifier: Arc<Mutex<PrinterState>>,
    ui_update_trigger: mpsc::Sender<()>,
    mut cmd_rx: mpsc::Receiver<MoonrakerCommand>,
) {
    tokio::spawn(async move {
        let ws_url = format!("ws://{}:{}/websocket", host, port);
        let mut msg_id = 1u64;
        let backoff_delays = [1, 2, 4, 8, 16];
        let mut attempt = 0;

        loop {
            info!(
                "Connecting to Moonraker at {} (attempt {})...",
                ws_url,
                attempt + 1
            );
            let ws_stream = match connect_async(&ws_url).await {
                Ok((stream, _)) => {
                    attempt = 0; // Reset backoff on successful connection
                    stream
                }
                Err(err) => {
                    error!("WebSocket connection failed: {}.", err);
                    {
                        let mut state = state_notifier.lock().unwrap();
                        state.printer_state = "disconnected".to_string();
                    }
                    let _ = ui_update_trigger.send(()).await;

                    let delay_secs = backoff_delays[attempt.min(backoff_delays.len() - 1)];
                    warn!("Reconnecting in {} seconds...", delay_secs);
                    sleep(Duration::from_secs(delay_secs)).await;
                    attempt += 1;
                    continue;
                }
            };

            info!("Moonraker Connected! Sending subscriptions...");
            {
                let mut state = state_notifier.lock().unwrap();
                state.printer_state = "connected".to_string();
            }
            let _ = ui_update_trigger.send(()).await;

            let (mut write, mut read) = ws_stream.split();

            // Send subscription request
            let sub_params = serde_json::json!({
                "connection_id": "rklipperscreen-client",
                "objects": {
                    "gcode_move": ["gcode_position", "speed_factor", "extrude_factor"],
                    "toolhead": ["position", "status", "estimated_print_time"],
                    "extruder": ["temperature", "target", "power"],
                    "heater_bed": ["temperature", "target", "power"],
                    "fan": ["speed"],
                    "idle_timeout": ["state"],
                    "print_stats": ["filename", "total_duration", "print_duration", "filament_used", "state", "progress"],
                    "virtual_sdcard": ["progress", "is_active", "file_position"]
                }
            });

            let sub_request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "printer.objects.subscribe".to_string(),
                params: sub_params,
                id: msg_id,
            };
            msg_id += 1;

            if let Ok(sub_text) = serde_json::to_string(&sub_request) {
                if let Err(err) = write.send(Message::Text(sub_text.into())).await {
                    error!("Failed to send subscription request: {}", err);
                    continue;
                }
            }

            loop {
                tokio::select! {
                    ws_msg = read.next() => {
                        match ws_msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(json_val) = serde_json::from_str::<Value>(&text) {
                                    // Handle subscription response / snapshot
                                    if let Some(result) = json_val.get("result") {
                                        if let Some(status) = result.get("status") {
                                            let mut state = state_notifier.lock().unwrap();
                                            reduce_printer_state(&mut state, status);
                                        }
                                    }
                                    // Handle dynamic update notifications
                                    else if let Some(method) = json_val.get("method").and_then(|m| m.as_str()) {
                                        if method == "notify_status_update" {
                                            if let Some(params) = json_val.get("params").and_then(|p| p.as_array()) {
                                                for param in params {
                                                    if param.is_object() {
                                                        let mut state = state_notifier.lock().unwrap();
                                                        reduce_printer_state(&mut state, param);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let _ = ui_update_trigger.send(()).await;
                                }
                            }
                            Some(Err(err)) => {
                                error!("WebSocket connection error: {}", err);
                                break;
                            }
                            None => {
                                warn!("WebSocket stream ended.");
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        let (method, params) = match cmd {
                            MoonrakerCommand::SendGcode(script) => {
                                ("printer.gcode.script".to_string(), serde_json::json!({ "script": script }))
                            }
                            MoonrakerCommand::EmergencyStop => {
                                ("printer.emergency_stop".to_string(), Value::Null)
                            }
                            MoonrakerCommand::PausePrint => {
                                ("printer.print.pause".to_string(), Value::Null)
                            }
                            MoonrakerCommand::ResumePrint => {
                                ("printer.print.resume".to_string(), Value::Null)
                            }
                            MoonrakerCommand::CancelPrint => {
                                ("printer.print.cancel".to_string(), Value::Null)
                            }
                            MoonrakerCommand::StartPrint(filename) => {
                                ("printer.print.start".to_string(), serde_json::json!({ "filename": filename }))
                            }
                        };

                        let cmd_request = JsonRpcRequest {
                            jsonrpc: "2.0".to_string(),
                            method,
                            params,
                            id: msg_id,
                        };
                        msg_id += 1;

                        if let Ok(cmd_text) = serde_json::to_string(&cmd_request) {
                            if let Err(err) = write.send(Message::Text(cmd_text.into())).await {
                                error!("Failed to send command over WebSocket: {}", err);
                                break;
                            }
                        }
                    }
                }
            }

            // Connection closed, wait a brief moment before looping
            sleep(Duration::from_secs(2)).await;
        }
    });
}
