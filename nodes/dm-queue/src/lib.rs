use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use dora_node_api::{Metadata, Parameter};

use serde_json::{json, Value};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushOn {
    Signal,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TimeModeKind {
    Duration,
    Timestamp,
    WallClock,
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_size_buffers: usize,
    pub max_size_bytes: usize,
    pub max_size_time: u64,
    pub ring_buffer_max_size: usize,
    pub use_buffering: bool,
    pub high_watermark: f64,
    pub low_watermark: f64,
    pub temp_template: Option<String>,
    pub temp_remove: bool,
    pub flush_on: FlushOn,
    pub flush_timeout: u64,
    pub max_block_time: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size_buffers: 100,
            max_size_bytes: 2 * 1024 * 1024,
            max_size_time: 0,
            ring_buffer_max_size: 0,
            use_buffering: false,
            high_watermark: 0.99,
            low_watermark: 0.01,
            temp_template: None,
            temp_remove: true,
            flush_on: FlushOn::Signal,
            flush_timeout: 0,
            max_block_time: 10_000_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSignal {
    Start,
    Chunk,
    End,
}

#[derive(Debug, Clone)]
pub struct QueueMessage {
    pub bytes: Vec<u8>,
    pub metadata: BTreeMap<String, Value>,
    pub stream_signal: Option<StreamSignal>,
    pub stream_id: Option<String>,
    pub timestamp_ns: Option<u64>,
    pub duration_ns: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum QueueOutput {
    /// Raw data output (flushed bytes + queue metadata + upstream params).
    Data {
        port: String,
        bytes: Vec<u8>,
        metadata: BTreeMap<String, String>,
        /// Original producer's dora parameters, forwarded as-is.
        upstream_params: BTreeMap<String, Value>,
    },
    /// Control event output (buffering/error as JSON string).
    Event {
        port: String,
        payload: String,
    },
}

#[derive(Debug, Clone)]
struct QueueItem {
    bytes: Vec<u8>,
    timestamp_ns: Option<u64>,
    duration_ns: Option<u64>,
    enqueue_ns: u64,
}

#[derive(Debug, Clone)]
struct FileSpoolState {
    path: PathBuf,
    meta_path: PathBuf,
}

#[derive(Debug)]
pub struct QueueEngine {
    config: QueueConfig,
    items: VecDeque<QueueItem>,
    total_bytes: usize,
    total_duration_ns: u64,
    first_timestamp_ns: Option<u64>,
    last_timestamp_ns: Option<u64>,
    first_enqueue_ns: Option<u64>,
    last_enqueue_ns: Option<u64>,
    active_stream_id: Option<String>,
    blocked_since_ns: Option<u64>,
    was_high: bool,
    time_mode: Option<TimeModeKind>,
    /// Upstream dora metadata captured from the first item.
    /// Forwarded on flush so downstream sees original producer params.
    upstream_params: BTreeMap<String, Value>,
}

impl QueueEngine {
    pub fn new(config: QueueConfig) -> Self {
        Self {
            config,
            items: VecDeque::new(),
            total_bytes: 0,
            total_duration_ns: 0,
            first_timestamp_ns: None,
            last_timestamp_ns: None,
            first_enqueue_ns: None,
            last_enqueue_ns: None,
            active_stream_id: None,
            blocked_since_ns: None,
            was_high: false,
            time_mode: None,
            upstream_params: BTreeMap::new(),
        }
    }

    pub fn handle_data(&mut self, msg: QueueMessage, now_ns: u64) -> Vec<QueueOutput> {
        let mut out = self.handle_idle_timeout(now_ns);
        if let Err(err) = self.preprocess_stream(&msg, &mut out, now_ns) {
            out.push(error_output("invalid_stream_transition", &err.to_string(), false));
            return out;
        }

        if self.config.ring_buffer_max_size > 0 && msg.bytes.len() > self.config.ring_buffer_max_size {
            out.push(error_output(
                "oversize_ring_item",
                "single item exceeds ring-buffer-max-size",
                false,
            ));
            return out;
        }

        if self.config.ring_buffer_max_size == 0
            && self.config.max_size_bytes > 0
            && msg.bytes.len() > self.config.max_size_bytes
        {
            if !self.items.is_empty() {
                out.extend(self.flush("stream_switch", now_ns).unwrap_or_else(error_only));
            }
            out.push(flushed_output_from_single(&msg));
            return out;
        }

        if self.config.ring_buffer_max_size > 0 {
            out.extend(self.evict_for_ring(msg.bytes.len(), now_ns));
        } else if self.would_overflow(msg.bytes.len(), msg.duration_ns, now_ns) {
            // Auto-flush to make room, then accept new data.
            out.extend(self.flush("overflow", now_ns).unwrap_or_else(error_only));
        }

        // Capture upstream params from the first item.
        if self.items.is_empty() {
            self.upstream_params = msg.metadata.clone();
        }

        let item = QueueItem {
            bytes: msg.bytes,
            timestamp_ns: msg.timestamp_ns,
            duration_ns: msg.duration_ns,
            enqueue_ns: now_ns,
        };
        self.push_item(item);
        out.extend(self.emit_watermark_events());

        if self.config.flush_on == FlushOn::Full && self.is_full(now_ns) {
            out.extend(self.flush("full", now_ns).unwrap_or_else(error_only));
        } else if matches!(msg.stream_signal, Some(StreamSignal::End))
            && self.config.flush_on == FlushOn::Signal
        {
            out.extend(self.flush("signal", now_ns).unwrap_or_else(error_only));
        }

        out
    }

    pub fn handle_control(&mut self, command: &str, now_ns: u64) -> Vec<QueueOutput> {
        let mut out = self.handle_idle_timeout(now_ns);
        match command {
            "flush" => out.extend(self.flush("control", now_ns).unwrap_or_else(error_only)),
            "reset" => self.reset(),
            "stop" => out.extend(self.flush("stop", now_ns).unwrap_or_else(error_only)),
            other => out.push(error_output(
                "invalid_control_command",
                &format!("unsupported control command: {other}"),
                true,
            )),
        }
        out
    }

    pub fn handle_idle_timeout(&mut self, now_ns: u64) -> Vec<QueueOutput> {
        if self.items.is_empty() || self.config.flush_timeout == 0 {
            return Vec::new();
        }
        let last_enqueue = self.last_enqueue_ns.unwrap_or(now_ns);
        if now_ns.saturating_sub(last_enqueue) >= self.config.flush_timeout {
            return self.flush("timeout", now_ns).unwrap_or_else(error_only);
        }
        Vec::new()
    }

    fn preprocess_stream(&mut self, msg: &QueueMessage, out: &mut Vec<QueueOutput>, now_ns: u64) -> Result<()> {
        if let Some(stream_id) = &msg.stream_id {
            if let Some(active) = &self.active_stream_id {
                if active != stream_id && !self.items.is_empty() {
                    out.extend(self.flush("stream_switch", now_ns).unwrap_or_else(error_only));
                }
            }
            self.active_stream_id = Some(stream_id.clone());
        }

        if matches!(msg.stream_signal, Some(StreamSignal::Start)) && msg.stream_id.is_none() && !self.items.is_empty() {
            bail!("received stream=start without stream_id while queue is not empty");
        }
        Ok(())
    }

    fn ensure_capacity(&mut self, incoming_bytes: usize, incoming_duration_ns: Option<u64>, now_ns: u64) -> Result<()> {
        if !self.would_overflow(incoming_bytes, incoming_duration_ns, now_ns) {
            self.blocked_since_ns = None;
            return Ok(());
        }

        let blocked_since = self.blocked_since_ns.get_or_insert(now_ns);
        if now_ns.saturating_sub(*blocked_since) >= self.config.max_block_time {
            return Err(anyhow!("producer blocked for longer than max-block-time"));
        }

        Err(anyhow!("queue is full and waiting for capacity"))
    }

    fn evict_for_ring(&mut self, incoming_bytes: usize, now_ns: u64) -> Vec<QueueOutput> {
        let mut out = Vec::new();
        while self.ring_would_overflow(incoming_bytes, now_ns) {
            if let Some(front) = self.items.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(front.bytes.len());
                self.total_duration_ns = self.total_duration_ns.saturating_sub(front.duration_ns.unwrap_or(0));
                out.push(buffering_output(
                    "ring_overwrite",
                    self.snapshot_time(now_ns),
                    self.total_bytes,
                    self.items.len(),
                    "ring",
                    None,
                ));
            } else {
                break;
            }
        }
        self.recompute_derived();
        out
    }

    fn push_item(&mut self, item: QueueItem) {
        if self.items.is_empty() {
            self.first_enqueue_ns = Some(item.enqueue_ns);
        }

        match self.time_mode {
            None => {
                self.time_mode = if item.duration_ns.is_some() {
                    Some(TimeModeKind::Duration)
                } else if item.timestamp_ns.is_some() {
                    Some(TimeModeKind::Timestamp)
                } else {
                    Some(TimeModeKind::WallClock)
                };
            }
            Some(TimeModeKind::Duration) => {
                if item.duration_ns.is_none() {
                    self.time_mode = Some(TimeModeKind::WallClock);
                }
            }
            Some(TimeModeKind::Timestamp) => {
                if item.timestamp_ns.is_none() {
                    self.time_mode = Some(TimeModeKind::WallClock);
                }
            }
            Some(TimeModeKind::WallClock) => {}
        }

        self.total_bytes += item.bytes.len();
        self.total_duration_ns += item.duration_ns.unwrap_or(0);
        self.first_timestamp_ns = self.first_timestamp_ns.or(item.timestamp_ns);
        self.last_timestamp_ns = item.timestamp_ns.or(self.last_timestamp_ns);
        self.last_enqueue_ns = Some(item.enqueue_ns);
        self.items.push_back(item);
    }

    fn flush(&mut self, reason: &str, now_ns: u64) -> Result<Vec<QueueOutput>> {
        if self.items.is_empty() {
            return Ok(Vec::new());
        }

        let out = if self.config.temp_template.is_some() {
            vec![self.flush_to_file(reason)?]
        } else {
            vec![self.flush_to_memory(reason)]
        };

        self.reset();
        if self.was_high && self.config.use_buffering {
            return Ok(vec![
                out[0].clone(),
                buffering_output("low_watermark", 0, 0, 0, "backpressure", Some("flush")),
            ]);
        }

        let _ = now_ns;
        Ok(out)
    }

    fn flush_to_memory(&self, reason: &str) -> QueueOutput {
        let mut bytes = Vec::with_capacity(self.total_bytes);
        for item in &self.items {
            bytes.extend_from_slice(&item.bytes);
        }
        QueueOutput::Data {
            port: "flushed".to_string(),
            bytes,
            metadata: self.flush_metadata(reason),
            upstream_params: self.upstream_params.clone(),
        }
    }

    fn flush_to_file(&self, reason: &str) -> Result<QueueOutput> {
        let spool = create_spool(self.config.temp_template.as_deref().unwrap())?;
        let mut writer = std::fs::File::create(&spool.path)?;
        for item in &self.items {
            use std::io::Write;
            writer.write_all(&item.bytes)?;
        }

        let mut metadata = self.flush_metadata(reason);
        metadata.insert("storage".to_string(), "file".to_string());
        metadata.insert("path".to_string(), spool.path.to_string_lossy().to_string());
        metadata.insert("meta_path".to_string(), spool.meta_path.to_string_lossy().to_string());

        // Write a small meta.json for file-based recovery
        let meta = json!({
            "total_items": self.items.len(),
            "total_bytes": self.total_bytes,
            "flush_reason": reason,
            "temp_remove": self.config.temp_remove,
        });
        std::fs::write(&spool.meta_path, serde_json::to_vec_pretty(&meta)?)?;

        // Read back the flushed file as raw bytes for downstream output
        let bytes = std::fs::read(&spool.path)?;
        Ok(QueueOutput::Data {
            port: "flushed".to_string(),
            bytes,
            metadata,
            upstream_params: self.upstream_params.clone(),
        })
    }

    fn flush_metadata(&self, reason: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("flush_reason".to_string(), reason.to_string());
        m.insert("total_items".to_string(), self.items.len().to_string());
        m.insert("total_bytes".to_string(), self.total_bytes.to_string());
        m.insert("total_duration_ns".to_string(), self.snapshot_time(self.last_enqueue_ns.unwrap_or_default()).to_string());
        if let Some(sid) = &self.active_stream_id {
            m.insert("stream_id".to_string(), sid.clone());
        }
        m
    }

    fn reset(&mut self) {
        self.items.clear();
        self.total_bytes = 0;
        self.total_duration_ns = 0;
        self.first_timestamp_ns = None;
        self.last_timestamp_ns = None;
        self.first_enqueue_ns = None;
        self.last_enqueue_ns = None;
        self.active_stream_id = None;
        self.blocked_since_ns = None;
        self.was_high = false;
        self.time_mode = None;
        self.upstream_params.clear();
    }

    fn recompute_derived(&mut self) {
        self.total_bytes = self.items.iter().map(|item| item.bytes.len()).sum();
        self.total_duration_ns = self.items.iter().map(|item| item.duration_ns.unwrap_or(0)).sum();
        self.first_timestamp_ns = self.items.front().and_then(|item| item.timestamp_ns);
        self.last_timestamp_ns = self.items.back().and_then(|item| item.timestamp_ns);
        self.first_enqueue_ns = self.items.front().map(|item| item.enqueue_ns);
        self.last_enqueue_ns = self.items.back().map(|item| item.enqueue_ns);
        if self.items.is_empty() {
            self.time_mode = None;
            self.active_stream_id = None;
        }
    }

    fn emit_watermark_events(&mut self) -> Vec<QueueOutput> {
        if !self.config.use_buffering {
            return Vec::new();
        }

        let fill = self.fill_ratio(self.last_enqueue_ns.unwrap_or_default());
        let mut out = Vec::new();
        if !self.was_high && fill >= self.config.high_watermark {
            self.was_high = true;
            out.push(buffering_output(
                "high_watermark",
                self.snapshot_time(self.last_enqueue_ns.unwrap_or_default()),
                self.total_bytes,
                self.items.len(),
                "backpressure",
                None,
            ));
        } else if self.was_high && fill <= self.config.low_watermark {
            self.was_high = false;
            out.push(buffering_output(
                "low_watermark",
                self.snapshot_time(self.last_enqueue_ns.unwrap_or_default()),
                self.total_bytes,
                self.items.len(),
                "backpressure",
                None,
            ));
        }
        out
    }

    fn is_full(&self, now_ns: u64) -> bool {
        self.fill_ratio(now_ns) >= 1.0
    }

    fn fill_ratio(&self, now_ns: u64) -> f64 {
        let mut ratios = Vec::new();
        if self.config.max_size_buffers > 0 {
            ratios.push(self.items.len() as f64 / self.config.max_size_buffers as f64);
        }
        if self.config.max_size_bytes > 0 {
            ratios.push(self.total_bytes as f64 / self.config.max_size_bytes as f64);
        }
        if self.config.max_size_time > 0 {
            ratios.push(self.snapshot_time(now_ns) as f64 / self.config.max_size_time as f64);
        }
        ratios.into_iter().fold(0.0, f64::max)
    }

    fn snapshot_time(&self, now_ns: u64) -> u64 {
        match self.time_mode {
            Some(TimeModeKind::Duration) => self.total_duration_ns,
            Some(TimeModeKind::Timestamp) => match (self.first_timestamp_ns, self.last_timestamp_ns) {
                (Some(first), Some(last)) if last >= first => last - first,
                _ => 0,
            },
            Some(TimeModeKind::WallClock) => self
                .first_enqueue_ns
                .map(|first| now_ns.saturating_sub(first))
                .unwrap_or(0),
            None => 0,
        }
    }

    fn would_overflow(&self, incoming_bytes: usize, incoming_duration_ns: Option<u64>, now_ns: u64) -> bool {
        (self.config.max_size_buffers > 0 && self.items.len() + 1 > self.config.max_size_buffers)
            || (self.config.max_size_bytes > 0 && self.total_bytes + incoming_bytes > self.config.max_size_bytes)
            || (self.config.max_size_time > 0
                && self.snapshot_time(now_ns).saturating_add(incoming_duration_ns.unwrap_or(0))
                    > self.config.max_size_time)
    }

    fn ring_would_overflow(&self, incoming_bytes: usize, now_ns: u64) -> bool {
        (self.config.max_size_buffers > 0 && self.items.len() + 1 > self.config.max_size_buffers)
            || (self.config.ring_buffer_max_size > 0
                && self.total_bytes + incoming_bytes > self.config.ring_buffer_max_size)
            || (self.config.max_size_time > 0 && self.snapshot_time(now_ns) > self.config.max_size_time)
    }
}

fn create_spool(template: &str) -> Result<FileSpoolState> {
    let path = Path::new(template);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("dm-queue-XXXXXX.bin");
    let (prefix, suffix) = split_template(name);
    let temp = NamedTempFile::new_in(parent)?;
    let file_name = temp
        .path()
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("failed to allocate temp file name"))?;
    let output_name = format!("{prefix}{file_name}{suffix}");
    let output_path = parent.join(output_name);
    temp.persist(&output_path)?;
    let meta_path = output_path.with_extension("meta.json");
    Ok(FileSpoolState {
        path: output_path,
        meta_path,
    })
}

fn split_template(template: &str) -> (&str, &str) {
    if let Some(index) = template.find("XXXXXX") {
        (&template[..index], &template[index + 6..])
    } else {
        (template, "")
    }
}

pub fn extract_queue_message(data: &[u8], metadata: &Metadata) -> QueueMessage {
    let mut metadata_map = BTreeMap::new();
    let mut stream_signal = None;
    let mut stream_id = None;
    let mut timestamp_ns = None;
    let mut duration_ns = None;

    for (key, value) in &metadata.parameters {
        let json_value = parameter_to_json(value);
        match (key.as_str(), value) {
            ("stream", Parameter::String(raw)) => {
                stream_signal = match raw.as_str() {
                    "start" => Some(StreamSignal::Start),
                    "chunk" => Some(StreamSignal::Chunk),
                    "end" => Some(StreamSignal::End),
                    _ => None,
                };
            }
            ("stream_id", Parameter::String(raw)) => stream_id = Some(raw.clone()),
            ("timestamp_ns", Parameter::Integer(raw)) if *raw >= 0 => timestamp_ns = Some(*raw as u64),
            ("duration_ns", Parameter::Integer(raw)) if *raw >= 0 => duration_ns = Some(*raw as u64),
            _ => {}
        }
        metadata_map.insert(key.clone(), json_value);
    }

    QueueMessage {
        bytes: data.to_vec(),
        metadata: metadata_map,
        stream_signal,
        stream_id,
        timestamp_ns,
        duration_ns,
    }
}

pub fn parameter_to_json(parameter: &Parameter) -> Value {
    match parameter {
        Parameter::Bool(value) => json!(value),
        Parameter::Integer(value) => json!(value),
        Parameter::String(value) => json!(value),
        Parameter::ListInt(value) => json!(value),
        Parameter::Float(value) => json!(value),
        Parameter::ListFloat(value) => json!(value),
        Parameter::ListString(value) => json!(value),
    }
}



fn buffering_output(
    event: &str,
    current_level_time: u64,
    current_level_bytes: usize,
    current_level_buffers: usize,
    mode: &str,
    reason: Option<&str>,
) -> QueueOutput {
    QueueOutput::Event {
        port: "buffering".to_string(),
        payload: json!({
            "event": event,
            "current_level_buffers": current_level_buffers,
            "current_level_bytes": current_level_bytes,
            "current_level_time": current_level_time,
            "mode": mode,
            "reason": reason,
        })
        .to_string(),
    }
}

fn error_output(code: &str, message: &str, recoverable: bool) -> QueueOutput {
    QueueOutput::Event {
        port: "error".to_string(),
        payload: json!({
            "code": code,
            "message": message,
            "recoverable": recoverable,
        })
        .to_string(),
    }
}

fn flushed_output_from_single(msg: &QueueMessage) -> QueueOutput {
    let mut metadata = BTreeMap::new();
    metadata.insert("flush_reason".to_string(), "oversize_bypass".to_string());
    metadata.insert("total_items".to_string(), "1".to_string());
    metadata.insert("total_bytes".to_string(), msg.bytes.len().to_string());
    if let Some(sid) = &msg.stream_id {
        metadata.insert("stream_id".to_string(), sid.clone());
    }
    QueueOutput::Data {
        port: "flushed".to_string(),
        bytes: msg.bytes.clone(),
        metadata,
        upstream_params: msg.metadata.clone(),
    }
}

fn error_only(err: anyhow::Error) -> Vec<QueueOutput> {
    vec![error_output("flush_emit_error", &err.to_string(), false)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn message(bytes: usize) -> QueueMessage {
        QueueMessage {
            bytes: vec![1; bytes],
            metadata: BTreeMap::new(),
            stream_signal: None,
            stream_id: Some("stream-1".to_string()),
            timestamp_ns: Some(1),
            duration_ns: Some(10),
        }
    }

    fn output_port(output: &QueueOutput) -> &str {
        match output {
            QueueOutput::Data { port, .. } => port,
            QueueOutput::Event { port, .. } => port,
        }
    }

    #[test]
    fn flushes_on_signal_end() {
        let mut engine = QueueEngine::new(QueueConfig::default());
        let mut start = message(4);
        start.stream_signal = Some(StreamSignal::Start);
        assert!(engine.handle_data(start, 1).is_empty());

        let mut end = message(4);
        end.stream_signal = Some(StreamSignal::End);
        let out = engine.handle_data(end, 2);
        assert_eq!(out.len(), 1);
        assert_eq!(output_port(&out[0]), "flushed");
        match &out[0] {
            QueueOutput::Data { bytes, metadata, .. } => {
                assert_eq!(bytes.len(), 8); // 4 + 4 bytes
                assert_eq!(metadata.get("flush_reason").unwrap(), "signal");
            }
            _ => panic!("expected Data variant"),
        }
    }

    #[test]
    fn ring_mode_emits_overwrite_event() {
        let mut config = QueueConfig::default();
        config.ring_buffer_max_size = 8;
        config.max_size_bytes = 0;
        let mut engine = QueueEngine::new(config);

        assert!(engine.handle_data(message(6), 1).is_empty());
        let out = engine.handle_data(message(6), 2);
        assert!(out.iter().any(|event| output_port(event) == "buffering"));
    }

    #[test]
    fn full_mode_flushes_when_capacity_hit() {
        let mut config = QueueConfig::default();
        config.max_size_buffers = 2;
        config.max_size_bytes = 0;
        config.max_size_time = 0;
        config.flush_on = FlushOn::Full;
        let mut engine = QueueEngine::new(config);

        assert!(engine.handle_data(message(1), 1).is_empty());
        let out = engine.handle_data(message(1), 2);
        assert!(out.iter().any(|event| output_port(event) == "flushed"));
    }

    #[test]
    fn timeout_flushes_pending_items() {
        let mut config = QueueConfig::default();
        config.flush_timeout = 5;
        let mut engine = QueueEngine::new(config);
        assert!(engine.handle_data(message(2), 1).is_empty());

        let out = engine.handle_idle_timeout(6);
        assert!(out.iter().any(|event| output_port(event) == "flushed"));
    }

    #[test]
    fn writes_spool_files_when_template_is_set() {
        let dir = tempdir().unwrap();
        let mut config = QueueConfig::default();
        config.temp_template = Some(
            dir.path()
                .join("dm-queue-XXXXXX.bin")
                .to_string_lossy()
                .to_string(),
        );
        let mut engine = QueueEngine::new(config);
        assert!(engine.handle_data(message(3), 1).is_empty());
        let mut end = message(3);
        end.stream_signal = Some(StreamSignal::End);
        let out = engine.handle_data(end, 2);
        let flushed = out.iter().find(|item| output_port(item) == "flushed").unwrap();
        match flushed {
            QueueOutput::Data { bytes, metadata, .. } => {
                assert_eq!(bytes.len(), 6); // 3 + 3 bytes
                let path = metadata.get("path").unwrap();
                let meta_path = metadata.get("meta_path").unwrap();
                assert!(Path::new(path).exists());
                assert!(Path::new(meta_path).exists());
            }
            _ => panic!("expected Data variant"),
        }
    }
}
