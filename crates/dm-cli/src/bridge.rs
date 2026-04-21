use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use dora_node_api::arrow::array::{
    BooleanArray, Float64Array, Int64Array, StringArray, UInt8Array,
};
use dora_node_api::{DoraNode, Event};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

type ReadHalf = tokio::io::ReadHalf<tokio::net::UnixStream>;
type WriteHalf = tokio::io::WriteHalf<tokio::net::UnixStream>;
type Lines = tokio::io::Lines<BufReader<ReadHalf>>;

// ── Unified task queue ──

enum Task {
    DoraEvent(Event),
    SocketInput(String),
    Stop,
}

// ── Spec types ──

#[derive(Debug, Clone, Deserialize)]
struct BridgeSpec {
    yaml_id: String,
    node_id: String,
    env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    bindings: Vec<BridgeBinding>,
    #[serde(default)]
    bridge_input_port: Option<String>,
    #[serde(default)]
    bridge_output_port: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BridgeBinding {
    family: String,
    #[serde(default)]
    port: Option<String>,
    #[serde(default)]
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InputNotification {
    to: String,
    value: Value,
}

// ── Main entry ──

pub async fn bridge_serve(home: &Path, run_id: &str) -> Result<()> {
    let specs = parse_specs();
    let sock_path = home.join("bridge.sock");

    let (mut node, mut events) =
        DoraNode::init_from_env().map_err(|e| anyhow::anyhow!("Failed to init bridge: {e}"))?;

    // Widget routing table
    let mut widget_specs = std::collections::BTreeMap::new();
    for spec in &specs {
        if let Some(port) = &spec.bridge_output_port {
            widget_specs.insert(spec.yaml_id.clone(), (port.clone(), widget_payload(spec)));
        }
    }
    eprintln!(
        "[{}] [bridge] input widgets={:?}",
        now_ts(),
        widget_specs.keys().collect::<Vec<_>>()
    );

    // Display routing table
    let display_ports: std::collections::BTreeMap<String, BridgeSpec> = specs
        .iter()
        .filter_map(|s| s.bridge_input_port.clone().map(|p| (p, s.clone())))
        .collect();
    eprintln!(
        "[{}] [bridge] display ports={:?}",
        now_ts(),
        display_ports.keys().collect::<Vec<_>>()
    );

    // Connect to server
    let (lines, mut writer) = connect_with_retry(&sock_path).await?;
    write_line(
        &mut writer,
        &json!({"action":"init","run_id":run_id}).to_string(),
    )
    .await?;
    for (yaml_id, (_, payload)) in &widget_specs {
        let msg = json!({"action":"push","from":yaml_id,"tag":"widgets","payload":payload});
        write_line(&mut writer, &msg.to_string()).await?;
    }
    writer.flush().await?;

    // ── Unified FIFO queue ──
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Task>(256);

    // Producer 1: dora events (sync thread → channel)
    let dora_tx = tx.clone();
    std::thread::spawn(move || loop {
        match events.recv() {
            Some(event @ Event::Stop(_)) => {
                let _ = dora_tx.blocking_send(Task::DoraEvent(event));
                break;
            }
            Some(event) => {
                let _ = dora_tx.blocking_send(Task::DoraEvent(event));
            }
            None => break,
        }
    });

    // Producer 2: Unix socket input (async task → channel)
    let socket_tx = tx.clone();
    let mut lines = lines;
    tokio::spawn(async move {
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if socket_tx.send(Task::SocketInput(line)).await.is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });

    // Drop our sender so rx ends when both producers finish
    drop(tx);

    // ── Consumer: process tasks FIFO ──
    while let Some(task) = rx.recv().await {
        match task {
            Task::DoraEvent(Event::Input { id, data, .. }) => {
                let event_id: &str = id.as_ref();
                if let Some(spec) = display_ports.get(event_id) {
                    if let Some(decoded) = decode_display_payload(&data) {
                        let tag = decoded
                            .get("tag")
                            .and_then(Value::as_str)
                            .unwrap_or("text")
                            .to_string();
                        let payload = decoded.get("payload").cloned().unwrap_or_else(|| json!({}));
                        let msg = json!({
                            "action":"push",
                            "from": spec.yaml_id,
                            "tag": tag,
                            "payload": payload,
                        });
                        write_line(&mut writer, &msg.to_string()).await?;
                        writer.flush().await?;
                        eprintln!("[{}] [bridge] display {tag} -> {}", now_ts(), spec.yaml_id);
                    }
                }
            }
            Task::DoraEvent(Event::Stop(_)) => {
                eprintln!("[{}] [bridge] dora stop received", now_ts());
                break;
            }
            Task::DoraEvent(_) => {}
            Task::SocketInput(line) => {
                let notif: InputNotification = match serde_json::from_str(&line) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("[{}] [bridge] parse error: {e}", now_ts());
                        continue;
                    }
                };
                let Some((output_port, _)) = widget_specs.get(&notif.to) else {
                    continue;
                };
                let bridge_payload = json!({ "value": notif.value });
                send_json_command(&mut node, output_port, &bridge_payload)
                    .with_context(|| format!("Failed sending bridge output for {}", notif.to))?;
                eprintln!(
                    "[{}] [bridge] routed input -> {}/{output_port}",
                    now_ts(),
                    notif.to
                );
            }
            Task::Stop => break,
        }
    }

    Ok(())
}

// ── Helpers ──

async fn write_line(writer: &mut BufWriter<WriteHalf>, line: &str) -> Result<()> {
    writer.write_all(line.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

fn now_ts() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

async fn connect_with_retry(sock_path: &Path) -> Result<(Lines, BufWriter<WriteHalf>)> {
    let mut delay = Duration::from_millis(100);
    for attempt in 0..20 {
        match tokio::net::UnixStream::connect(sock_path).await {
            Ok(stream) => {
                let (read_half, write_half) = tokio::io::split(stream);
                let lines = BufReader::new(read_half).lines();
                return Ok((lines, BufWriter::new(write_half)));
            }
            Err(_) if attempt < 19 => {
                eprintln!(
                    "[{}] [bridge] socket not ready, retry in {delay:?}...",
                    now_ts()
                );
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(5));
            }
            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "Failed to connect to bridge socket at {}",
                        sock_path.display()
                    )
                })
            }
        }
    }
    unreachable!()
}

fn parse_specs() -> Vec<BridgeSpec> {
    std::env::var("DM_CAPABILITIES_JSON")
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<BridgeSpec>>(&raw).ok())
        .unwrap_or_default()
}

fn widget_payload(spec: &BridgeSpec) -> Value {
    let label = spec
        .env
        .get("LABEL")
        .cloned()
        .unwrap_or_else(|| spec.yaml_id.clone());
    let Some(binding) = spec
        .bindings
        .iter()
        .find(|b| b.family == "widget_input" && b.channel.as_deref() == Some("input"))
    else {
        return json!({"label": label, "widgets": {}});
    };
    let output_id = binding.port.clone().unwrap_or_else(|| "value".to_string());
    let widget = match spec.node_id.as_str() {
        "dm-text-input" => {
            let multiline = spec
                .env
                .get("MULTILINE")
                .map(|v| v == "true")
                .unwrap_or(false);
            json!({
                "type": if multiline { "textarea" } else { "input" },
                "label": label,
                "default": spec.env.get("DEFAULT_VALUE").cloned().unwrap_or_default(),
                "placeholder": spec.env.get("PLACEHOLDER").cloned().unwrap_or_else(|| "Type something...".to_string())
            })
        }
        "dm-button" => json!({"type": "button", "label": label, "value": label}),
        "dm-slider" => json!({
            "type": "slider",
            "label": label,
            "min": parse_f64(spec.env.get("MIN_VAL"), 0.0),
            "max": parse_f64(spec.env.get("MAX_VAL"), 100.0),
            "step": parse_f64(spec.env.get("STEP"), 1.0),
            "default": parse_f64(spec.env.get("DEFAULT_VALUE"), 50.0),
        }),
        "dm-input-switch" => json!({
            "type": "switch",
            "label": label,
            "default": spec.env.get("DEFAULT_VALUE").map(|v| v == "true").unwrap_or(false)
        }),
        _ => json!({}),
    };
    json!({"label": label, "widgets": {output_id: widget}})
}

fn parse_f64(value: Option<&String>, default: f64) -> f64 {
    value.and_then(|raw| raw.parse().ok()).unwrap_or(default)
}

fn decode_display_payload(data: &dora_node_api::ArrowData) -> Option<Value> {
    String::try_from(data)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn send_json_command(node: &mut DoraNode, output_id: &str, value: &Value) -> Result<()> {
    match value {
        Value::Bool(v) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                BooleanArray::from(vec![*v]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Int64Array::from(vec![i]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if let Some(f) = num.as_f64() {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Float64Array::from(vec![f]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
        Value::String(s) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                StringArray::from(vec![s.as_str()]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Value::Array(items) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                StringArray::from(vec![Value::Array(items.clone()).to_string()]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Value::Null | Value::Object(_) => {
            let bytes = value.to_string().into_bytes();
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                UInt8Array::from(bytes),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
    }
    Ok(())
}
