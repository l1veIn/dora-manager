use anyhow::{bail, Context, Result};

use dora_node_api::arrow::array::{
    BooleanArray, Float64Array, Int64Array, StringArray, UInt8Array,
};
use dora_node_api::arrow::datatypes::DataType;
use dora_node_api::{DoraNode, Event, Metadata, Parameter};

pub fn panel_serve(home: &std::path::Path, run_id: &str, _node_id: &str) -> Result<()> {
    let store = dm_core::runs::panel::PanelStore::open(home, run_id)?;
    let (mut node, mut events) =
        DoraNode::init_from_env().map_err(|e| anyhow::anyhow!("Failed to init panel node: {e}"))?;
    let mut last_cmd_seq = 0i64;
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let store2 = store.clone();
    let stop_flag = should_stop.clone();
    let reader = std::thread::spawn(move || {
        let mut saw_stop = false;
        while let Some(event) = events.recv() {
            match event {
                Event::Input { id, metadata, data } => {
                    let type_hint = extract_type_hint(&metadata, &data);
                    let bytes = arrow_to_bytes(&metadata, &data);
                    eprintln!("[panel] INPUT id={}, type_hint={}, data_type={:?}, {} bytes, params={:?}",
                        AsRef::<str>::as_ref(&id), type_hint, data.data_type(), bytes.len(),
                        metadata.parameters.keys().collect::<Vec<_>>());
                    if let Err(e) = store2.write_asset(id.as_ref(), &type_hint, &bytes) {
                        eprintln!("Panel write error: {e}");
                    }
                }
                Event::Stop(_) => {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    saw_stop = true;
                }
                _ => {}
            }
            if saw_stop {
                continue;
            }
        }
        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    loop {
        if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        for cmd in store.poll_commands(&mut last_cmd_seq)? {
            send_json_command(&mut node, &cmd.output_id, &cmd.value)
                .with_context(|| format!("Failed sending output '{}'", cmd.output_id))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    reader
        .join()
        .map_err(|_| anyhow::anyhow!("Panel event reader thread panicked"))?;

    Ok(())
}

pub fn panel_send(
    home: &std::path::Path,
    output_id: &str,
    value: &str,
    run: Option<String>,
) -> Result<()> {
    let run_id = match run {
        Some(id) => {
            let db_path = dm_core::runs::run_panel_dir(home, &id).join("index.db");
            if !db_path.exists() {
                bail!("Panel run '{}' not found", id);
            }
            id
        }
        None => {
            let runs = dm_core::runs::refresh_run_statuses(home)?;
            let mut active = runs
                .into_iter()
                .filter(|run| run.status.is_running() && run.has_panel)
                .collect::<Vec<_>>();
            active.sort_by(|a, b| b.started_at.cmp(&a.started_at));

            match active.len() {
                0 => bail!("No active run with panel found"),
                1 => active[0].run_id.clone(),
                _ => {
                    eprintln!("Multiple active runs with panel found; using most recent:");
                    for run in &active {
                        eprintln!("  {} ({})", run.run_id, run.started_at);
                    }
                    active[0].run_id.clone()
                }
            }
        }
    };

    let store = dm_core::runs::panel::PanelStore::open(home, &run_id)?;
    store.write_command(output_id, value)?;
    println!("✅ Sent: {} = {}", output_id, value);
    Ok(())
}

fn extract_type_hint(
    metadata: &Metadata,
    data: &dora_node_api::ArrowData,
) -> String {
    if let Some(Parameter::String(content_type)) =
        metadata.parameters.get("content_type")
    {
        return content_type.clone();
    }
    if sample_rate_from_metadata(metadata).is_some() && is_audio_array(data.data_type()) {
        return "audio/wav".to_string();
    }
    // UInt8 with sample_rate = raw PCM bytes from queue passthrough.
    if sample_rate_from_metadata(metadata).is_some() && matches!(data.data_type(), DataType::UInt8) {
        return "audio/wav".to_string();
    }
    match data.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => "text/plain".to_string(),
        DataType::Binary | DataType::LargeBinary => "application/octet-stream".to_string(),
        DataType::UInt8 => "application/octet-stream".to_string(),
        _ => format!("application/x-arrow+{:?}", data.data_type()).to_ascii_lowercase(),
    }
}

fn arrow_to_bytes(metadata: &Metadata, data: &dora_node_api::ArrowData) -> Vec<u8> {
    if let Some(sample_rate) = sample_rate_from_metadata(metadata) {
        if let Some(bytes) = audio_array_to_wav_bytes(sample_rate, data) {
            return bytes;
        }
        // Handle raw bytes (from queue passthrough) as Float32 PCM.
        if matches!(data.data_type(), DataType::UInt8) {
            let raw = Vec::<u8>::try_from(data).unwrap_or_default();
            let samples: Vec<i16> = raw
                .chunks_exact(4)
                .map(|c| {
                    let v = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                    (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                })
                .collect();
            return encode_wav_mono_i16(sample_rate, &samples);
        }
    }
    match data.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => String::try_from(data)
            .map(|s| s.into_bytes())
            .unwrap_or_else(|_| format!("{data:?}").into_bytes()),
        DataType::UInt8 => Vec::<u8>::try_from(data).unwrap_or_default(),
        DataType::Binary | DataType::LargeBinary => format!("{data:?}").into_bytes(),
        _ => format!("{data:?}").into_bytes(),
    }
}

fn sample_rate_from_metadata(metadata: &Metadata) -> Option<u32> {
    match metadata.parameters.get("sample_rate") {
        Some(Parameter::Integer(v)) if *v > 0 => Some(*v as u32),
        Some(Parameter::Float(v)) if *v > 0.0 => Some(*v as u32),
        _ => None,
    }
}

fn is_audio_array(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Float32
            | DataType::Float64
            | DataType::Int16
            | DataType::Int32
            | DataType::UInt16
            | DataType::UInt32
    )
}

fn audio_array_to_wav_bytes(sample_rate: u32, data: &dora_node_api::ArrowData) -> Option<Vec<u8>> {
    let samples = match data.data_type() {
        DataType::Float32 => {
            let values = Vec::<f32>::try_from(data).ok()?;
            values
                .into_iter()
                .map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect::<Vec<_>>()
        }
        DataType::Float64 => {
            let values = Vec::<f64>::try_from(data).ok()?;
            values
                .into_iter()
                .map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f64) as i16)
                .collect::<Vec<_>>()
        }
        _ => return None,
    };

    Some(encode_wav_mono_i16(sample_rate, &samples))
}

fn encode_wav_mono_i16(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let bits_per_sample = 16u16;
    let channels = 1u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_len = std::mem::size_of_val(samples) as u32;
    let riff_len = 36 + data_len;

    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn send_json_command(node: &mut DoraNode, output_id: &str, raw_json: &str) -> Result<()> {
    let value = serde_json::from_str::<serde_json::Value>(raw_json)
        .unwrap_or_else(|_| serde_json::Value::String(raw_json.to_string()));

    match value {
        serde_json::Value::Bool(v) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                BooleanArray::from(vec![v]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        serde_json::Value::Number(num) => {
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
            } else {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(vec![num.to_string()]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
        serde_json::Value::String(s) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                StringArray::from(vec![s]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        serde_json::Value::Array(items) => {
            if items.iter().all(|v| v.as_i64().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_i64().unwrap_or_default())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Int64Array::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if items.iter().all(|v| v.as_f64().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_f64().unwrap_or_default())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Float64Array::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if items.iter().all(|v| v.as_str().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(vec![serde_json::Value::Array(items).to_string()]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Object(_) => {
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
