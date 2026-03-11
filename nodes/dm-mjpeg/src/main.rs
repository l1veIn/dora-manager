use std::env;

use anyhow::{Context, Result};
use dora_node_api::arrow::datatypes::DataType;
use dora_node_api::{DoraNode, Event};
use tokio::sync::mpsc;

use dora_dm_mjpeg::{extract_frame, router, FrameFormat, FrameProcessor, MjpegConfig, StreamState};

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()?;
    let state = StreamState::new(config.allow_origin.clone());
    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();
    let dora_format = config.input_format.clone();

    let dora_thread = std::thread::spawn(move || run_dora_loop(frame_tx, dora_format));

    let state_for_frames = state.clone();
    let config_for_frames = config.clone();
    let frame_task = tokio::spawn(async move {
        let mut processor = FrameProcessor::new(config_for_frames);
        while let Some(frame) = frame_rx.recv().await {
            if let Some(encoded) = processor.process(frame)? {
                state_for_frames.update(encoded).await;
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .with_context(|| "failed to bind dm-mjpeg listener")?;
    let server = axum::serve(listener, router(state));

    tokio::select! {
        result = server => result.context("dm-mjpeg server failed")?,
        result = frame_task => result.context("frame task join failed")??,
    }

    dora_thread.join().map_err(|_| anyhow::anyhow!("dora thread panicked"))??;
    Ok(())
}

fn run_dora_loop(tx: mpsc::UnboundedSender<dora_dm_mjpeg::IncomingFrame>, default_format: FrameFormat) -> Result<()> {
    let (_node, mut events) = DoraNode::init_from_env()
        .map_err(|err| anyhow::anyhow!("failed to initialize dm-mjpeg node: {err}"))?;
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, metadata } if id.as_str() == "frame" => {
                let bytes = arrow_to_bytes(&data)?;
                let frame = extract_frame(&bytes, &metadata, default_format.clone())?;
                if tx.send(frame).is_err() {
                    break;
                }
            }
            Event::Stop(_) => break,
            Event::Error(err) => eprintln!("dm-mjpeg error: {err}"),
            _ => {}
        }
    }
    Ok(())
}

fn load_config() -> Result<MjpegConfig> {
    Ok(MjpegConfig {
        host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: env_u16("PORT", 4567)?,
        quality: env_u8("QUALITY", 80)?,
        max_fps: env_u32("MAX_FPS", 30)?,
        width: env_u32("WIDTH", 0)?,
        height: env_u32("HEIGHT", 0)?,
        input_format: parse_env_format(env::var("INPUT_FORMAT").unwrap_or_else(|_| "jpeg".to_string()).as_str())?,
        drop_if_no_client: env_bool("DROP_IF_NO_CLIENT", true)?,
        allow_origin: env::var("ALLOW_ORIGIN").ok().filter(|value| !value.is_empty()),
    })
}

fn parse_env_format(value: &str) -> Result<FrameFormat> {
    match value {
        "jpeg" => Ok(FrameFormat::Jpeg),
        "rgb8" => Ok(FrameFormat::Rgb8),
        "rgba8" => Ok(FrameFormat::Rgba8),
        "yuv420p" => Ok(FrameFormat::Yuv420p),
        _ => Err(anyhow::anyhow!("invalid INPUT_FORMAT")),
    }
}

fn env_u8(key: &str, default: u8) -> Result<u8> {
    env::var(key)
        .ok()
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn env_u16(key: &str, default: u16) -> Result<u16> {
    env::var(key)
        .ok()
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn env_u32(key: &str, default: u32) -> Result<u32> {
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
    match data.data_type() {
        DataType::UInt8 => Vec::<u8>::try_from(data).map_err(|err| anyhow::anyhow!(err.to_string())),
        other => Err(anyhow::anyhow!("unsupported input data type: {other:?}")),
    }
}
