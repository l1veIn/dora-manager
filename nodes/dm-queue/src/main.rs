use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use dora_node_api::arrow::array::{StringArray, UInt8Array};
use dora_node_api::{DoraNode, Event, MetadataParameters, Parameter};

use dora_dm_queue::{extract_queue_message, FlushOn, QueueConfig, QueueEngine, QueueOutput};

fn main() -> Result<()> {
    let (mut node, mut events) = DoraNode::init_from_env()
        .map_err(|err| anyhow::anyhow!("failed to initialize dm-queue node: {err}"))?;
    let config = load_config()?;
    eprintln!("[dm-queue] config: flush_on={:?}, flush_timeout={}ns, max_size_buffers={}, max_size_bytes={}, max_size_time={}ns",
        config.flush_on, config.flush_timeout, config.max_size_buffers, config.max_size_bytes, config.max_size_time);
    let mut engine = QueueEngine::new(config);

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, metadata } => {
                let now_ns = now_ns();
                let outputs = match id.as_str() {
                    "data" => {
                        let data_type = format!("{:?}", data.data_type());
                        let bytes = arrow_to_bytes(&data)?;
                        let message = extract_queue_message(&bytes, &metadata);
                        eprintln!("[dm-queue] INPUT data: {} bytes, arrow_type={}, stream_signal={:?}, stream_id={:?}",
                            bytes.len(), data_type, message.stream_signal, message.stream_id);
                        engine.handle_data(message, now_ns)
                    }
                    "control" => {
                        let command = String::try_from(&data)
                            .map_err(|err| anyhow::anyhow!("control input must be utf8: {err}"))?;
                        eprintln!("[dm-queue] INPUT control: {command}");
                        engine.handle_control(&command, now_ns)
                    }
                    "tick" => {
                        engine.handle_idle_timeout(now_ns)
                    }
                    other => {
                        eprintln!("[dm-queue] INPUT unknown port: {other}");
                        vec![QueueOutput::Event {
                            port: "error".to_string(),
                            payload: serde_json::json!({
                                "code": "unknown_input_port",
                                "message": format!("unsupported input port: {other}"),
                                "recoverable": true,
                            })
                            .to_string(),
                        }]
                    }
                };

                // eprintln!("[dm-queue] outputs: {} items", outputs.len());
                for output in outputs {
                    match output {
                        QueueOutput::Data { port, bytes, metadata: meta, upstream_params } => {
                            eprintln!("[dm-queue] OUTPUT Data port={port}, {} bytes, meta={:?}, upstream={:?}", bytes.len(), meta, upstream_params);
                            let mut params: MetadataParameters = Default::default();
                            // Forward upstream producer params as-is.
                            for (k, v) in &upstream_params {
                                if let Some(param) = json_to_parameter(v) {
                                    params.insert(k.clone().into(), param);
                                }
                            }
                            // Add queue metadata with prefix to avoid collisions.
                            for (k, v) in meta {
                                params.insert(format!("queue.{k}").into(), Parameter::String(v));
                            }
                            node.send_output(port.into(), params, UInt8Array::from(bytes))
                                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                        }
                        QueueOutput::Event { port, payload } => {
                            eprintln!("[dm-queue] OUTPUT Event port={port}, payload={payload}");
                            node.send_output(port.into(), Default::default(), StringArray::from(vec![payload]))
                                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                        }
                    }
                }
            }
            Event::Stop(_) => {
                eprintln!("[dm-queue] STOP received");
                break;
            }
            Event::Error(err) => eprintln!("[dm-queue] ERROR: {err}"),
            _ => {}
        }
    }

    Ok(())
}

fn load_config() -> Result<QueueConfig> {
    Ok(QueueConfig {
        max_size_buffers: env_usize("MAX_SIZE_BUFFERS", 100)?,
        max_size_bytes: env_usize("MAX_SIZE_BYTES", 2 * 1024 * 1024)?,
        max_size_time: env_u64("MAX_SIZE_TIME", 0)?,
        ring_buffer_max_size: env_usize("RING_BUFFER_MAX_SIZE", 0)?,
        use_buffering: env_bool("USE_BUFFERING", false)?,
        high_watermark: env_f64("HIGH_WATERMARK", 0.99)?,
        low_watermark: env_f64("LOW_WATERMARK", 0.01)?,
        temp_template: env::var("TEMP_TEMPLATE").ok().filter(|value| !value.is_empty()),
        temp_remove: env_bool("TEMP_REMOVE", true)?,
        flush_on: match env::var("FLUSH_ON").unwrap_or_else(|_| "signal".to_string()).as_str() {
            "full" => FlushOn::Full,
            _ => FlushOn::Signal,
        },
        flush_timeout: env_u64("FLUSH_TIMEOUT", 0)?,
        max_block_time: env_u64("MAX_BLOCK_TIME", 10_000_000_000)?,
    })
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
    env::var(key)
        .ok()
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn env_u64(key: &str, default: u64) -> Result<u64> {
    env::var(key)
        .ok()
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn env_f64(key: &str, default: f64) -> Result<f64> {
    env::var(key)
        .ok()
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn env_bool(key: &str, default: bool) -> Result<bool> {
    env::var(key)
        .ok()
        .map(|value| match value.as_str() {
            "1" | "true" | "TRUE" | "yes" | "on" => Ok(true),
            "0" | "false" | "FALSE" | "no" | "off" => Ok(false),
            _ => Err(anyhow::anyhow!("invalid {key}")),
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn arrow_to_bytes(data: &dora_node_api::ArrowData) -> Result<Vec<u8>> {
    // Queue is data-agnostic: extract raw bytes from any Arrow type
    // via ArrayData's buffer access.
    use dora_node_api::arrow::array::Array;
    let array_data = data.to_data();
    let buffers = array_data.buffers();
    if buffers.is_empty() {
        return Ok(Vec::new());
    }
    // Last buffer contains actual data values
    // (first buffer is offsets for variable-length types).
    let buffer = buffers.last().unwrap();
    Ok(buffer.as_slice().to_vec())
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(0)
}

fn json_to_parameter(value: &serde_json::Value) -> Option<Parameter> {
    match value {
        serde_json::Value::String(s) => Some(Parameter::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Parameter::Integer(i))
            } else {
                n.as_f64().map(Parameter::Float)
            }
        }
        serde_json::Value::Bool(b) => Some(Parameter::Bool(*b)),
        _ => Some(Parameter::String(value.to_string())),
    }
}
