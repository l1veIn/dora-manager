use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use async_stream::stream;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use dora_node_api::{Metadata, Parameter};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage, RgbaImage};
use tokio::sync::{watch, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameFormat {
    Jpeg,
    Rgb8,
    Rgba8,
    Yuv420p,
}

#[derive(Debug, Clone)]
pub struct MjpegConfig {
    pub host: String,
    pub port: u16,
    pub quality: u8,
    pub max_fps: u32,
    pub width: u32,
    pub height: u32,
    pub input_format: FrameFormat,
    pub drop_if_no_client: bool,
    pub allow_origin: Option<String>,
}

impl Default for MjpegConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4567,
            quality: 80,
            max_fps: 30,
            width: 0,
            height: 0,
            input_format: FrameFormat::Jpeg,
            drop_if_no_client: true,
            allow_origin: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IncomingFrame {
    pub bytes: Vec<u8>,
    pub format: FrameFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub stride: Option<usize>,
    pub timestamp_ns: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub jpeg: Arc<Vec<u8>>,
    pub timestamp_ns: Option<u64>,
}

#[derive(Clone)]
pub struct StreamState {
    latest: Arc<RwLock<Option<Arc<EncodedFrame>>>>,
    tx: watch::Sender<Option<Arc<EncodedFrame>>>,
    allow_origin: Option<String>,
}

impl StreamState {
    pub fn new(allow_origin: Option<String>) -> Self {
        let (tx, _rx) = watch::channel(None);
        Self {
            latest: Arc::new(RwLock::new(None)),
            tx,
            allow_origin,
        }
    }

    pub async fn update(&self, frame: EncodedFrame) {
        let frame = Arc::new(frame);
        *self.latest.write().await = Some(frame.clone());
        let _ = self.tx.send(Some(frame));
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<Arc<EncodedFrame>>> {
        self.tx.subscribe()
    }

    pub async fn latest(&self) -> Option<Arc<EncodedFrame>> {
        self.latest.read().await.clone()
    }

    pub fn allow_origin(&self) -> Option<&str> {
        self.allow_origin.as_deref()
    }
}

pub struct FrameProcessor {
    config: MjpegConfig,
    last_emitted_ns: Option<u64>,
}

impl FrameProcessor {
    pub fn new(config: MjpegConfig) -> Self {
        Self {
            config,
            last_emitted_ns: None,
        }
    }

    pub fn process(&mut self, frame: IncomingFrame) -> Result<Option<EncodedFrame>> {
        let now = frame.timestamp_ns.unwrap_or_else(now_ns);
        let min_interval = if self.config.max_fps == 0 {
            0
        } else {
            1_000_000_000u64 / self.config.max_fps as u64
        };

        if let Some(last) = self.last_emitted_ns {
            if min_interval > 0 && now.saturating_sub(last) < min_interval {
                return Ok(None);
            }
        }

        let jpeg = encode_frame(&frame, &self.config)?;
        self.last_emitted_ns = Some(now);
        Ok(Some(EncodedFrame {
            jpeg: Arc::new(jpeg),
            timestamp_ns: frame.timestamp_ns,
        }))
    }
}

pub fn router(state: StreamState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/snapshot.jpg", get(snapshot))
        .route("/stream", get(mjpeg_stream))
        .with_state(state)
}

pub fn extract_frame(data: &[u8], metadata: &Metadata, default_format: FrameFormat) -> Result<IncomingFrame> {
    let format = match metadata.parameters.get("format") {
        Some(Parameter::String(value)) => parse_format(value)?,
        _ => default_format,
    };
    let width = get_u32(metadata, "width");
    let height = get_u32(metadata, "height");
    let stride = get_u32(metadata, "stride").map(|value| value as usize);
    let timestamp_ns = get_u64(metadata, "timestamp_ns");

    if !matches!(format, FrameFormat::Jpeg) && (width.is_none() || height.is_none()) {
        bail!("invalid_frame_shape: width and height are required for raw frames");
    }

    Ok(IncomingFrame {
        bytes: data.to_vec(),
        format,
        width,
        height,
        stride,
        timestamp_ns,
    })
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn snapshot(State(state): State<StreamState>) -> Response<Body> {
    match state.latest().await {
        Some(frame) => image_response(StatusCode::OK, frame.jpeg.to_vec(), state.allow_origin()),
        None => text_response(StatusCode::SERVICE_UNAVAILABLE, "no frame available", state.allow_origin()),
    }
}

async fn mjpeg_stream(State(state): State<StreamState>) -> Response<Body> {
    let mut rx = state.subscribe();
    let initial = match state.latest().await {
        Some(frame) => Some(frame),
        None => match tokio::time::timeout(Duration::from_secs(30), rx.changed()).await {
            Ok(Ok(())) => rx.borrow().clone(),
            _ => None,
        },
    };

    let Some(initial_frame) = initial else {
        return text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "no frame available",
            state.allow_origin(),
        );
    };

    let body_stream = stream! {
        let mut next = Some(initial_frame);
        loop {
            if let Some(frame) = next.take() {
                yield Ok::<Bytes, Infallible>(multipart_chunk(&frame.jpeg));
            }

            if rx.changed().await.is_err() {
                break;
            }

            next = rx.borrow().clone();
        }
    };

    let mut response = Response::new(Body::from_stream(body_stream));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("multipart/x-mixed-replace; boundary=frame"),
    );
    maybe_insert_allow_origin(response.headers_mut(), state.allow_origin());
    response
}

fn text_response(status: StatusCode, body: &str, allow_origin: Option<&str>) -> Response<Body> {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    maybe_insert_allow_origin(response.headers_mut(), allow_origin);
    response
}

fn image_response(status: StatusCode, bytes: Vec<u8>, allow_origin: Option<&str>) -> Response<Body> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    maybe_insert_allow_origin(response.headers_mut(), allow_origin);
    response
}

fn maybe_insert_allow_origin(headers: &mut HeaderMap, allow_origin: Option<&str>) {
    if let Some(origin) = allow_origin {
        if let Ok(value) = HeaderValue::from_str(origin) {
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
        }
    }
}

fn multipart_chunk(bytes: &[u8]) -> Bytes {
    let mut out = Vec::with_capacity(bytes.len() + 128);
    out.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: ");
    out.extend_from_slice(bytes.len().to_string().as_bytes());
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n");
    Bytes::from(out)
}

fn parse_format(value: &str) -> Result<FrameFormat> {
    match value {
        "jpeg" => Ok(FrameFormat::Jpeg),
        "rgb8" => Ok(FrameFormat::Rgb8),
        "rgba8" => Ok(FrameFormat::Rgba8),
        "yuv420p" => Ok(FrameFormat::Yuv420p),
        other => bail!("unsupported_input_format: {other}"),
    }
}

fn get_u32(metadata: &Metadata, key: &str) -> Option<u32> {
    match metadata.parameters.get(key) {
        Some(Parameter::Integer(value)) if *value >= 0 => Some(*value as u32),
        _ => None,
    }
}

fn get_u64(metadata: &Metadata, key: &str) -> Option<u64> {
    match metadata.parameters.get(key) {
        Some(Parameter::Integer(value)) if *value >= 0 => Some(*value as u64),
        _ => None,
    }
}

fn encode_frame(frame: &IncomingFrame, config: &MjpegConfig) -> Result<Vec<u8>> {
    if matches!(frame.format, FrameFormat::Jpeg) && config.width == 0 && config.height == 0 {
        return Ok(frame.bytes.clone());
    }

    let mut image = match frame.format {
        FrameFormat::Jpeg => image::load_from_memory(&frame.bytes).context("jpeg_encode_failed: decode input jpeg")?,
        FrameFormat::Rgb8 => decode_rgb(frame)?,
        FrameFormat::Rgba8 => decode_rgba(frame)?,
        FrameFormat::Yuv420p => decode_yuv420p(frame)?,
    };

    if config.width > 0 || config.height > 0 {
        let (target_w, target_h) = resize_target(image.width(), image.height(), config.width, config.height);
        image = image.resize_exact(target_w, target_h, FilterType::Triangle);
    }

    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut out, config.quality);
    encoder.encode_image(&image).context("jpeg_encode_failed: encode output jpeg")?;
    Ok(out)
}

fn decode_rgb(frame: &IncomingFrame) -> Result<DynamicImage> {
    let width = frame.width.ok_or_else(|| anyhow!("invalid_frame_shape"))?;
    let height = frame.height.ok_or_else(|| anyhow!("invalid_frame_shape"))?;
    let expected = width as usize * height as usize * 3;
    if frame.bytes.len() < expected {
        bail!("invalid_frame_shape: rgb payload too small");
    }
    let image = RgbImage::from_raw(width, height, frame.bytes.clone())
        .ok_or_else(|| anyhow!("invalid_frame_shape: rgb image"))?;
    Ok(DynamicImage::ImageRgb8(image))
}

fn decode_rgba(frame: &IncomingFrame) -> Result<DynamicImage> {
    let width = frame.width.ok_or_else(|| anyhow!("invalid_frame_shape"))?;
    let height = frame.height.ok_or_else(|| anyhow!("invalid_frame_shape"))?;
    let expected = width as usize * height as usize * 4;
    if frame.bytes.len() < expected {
        bail!("invalid_frame_shape: rgba payload too small");
    }
    let image = RgbaImage::from_raw(width, height, frame.bytes.clone())
        .ok_or_else(|| anyhow!("invalid_frame_shape: rgba image"))?;
    Ok(DynamicImage::ImageRgba8(image))
}

fn decode_yuv420p(frame: &IncomingFrame) -> Result<DynamicImage> {
    let width = frame.width.ok_or_else(|| anyhow!("invalid_frame_shape"))? as usize;
    let height = frame.height.ok_or_else(|| anyhow!("invalid_frame_shape"))? as usize;
    let y_len = width * height;
    let uv_len = y_len / 4;
    if frame.bytes.len() < y_len + uv_len * 2 {
        bail!("invalid_frame_shape: yuv420p payload too small");
    }

    let y_plane = &frame.bytes[..y_len];
    let u_plane = &frame.bytes[y_len..y_len + uv_len];
    let v_plane = &frame.bytes[y_len + uv_len..y_len + uv_len * 2];

    let mut image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let y_value = y_plane[y * width + x] as f32;
            let uv_index = (y / 2) * (width / 2) + (x / 2);
            let u = u_plane[uv_index] as f32 - 128.0;
            let v = v_plane[uv_index] as f32 - 128.0;
            let r = (y_value + 1.402 * v).clamp(0.0, 255.0) as u8;
            let g = (y_value - 0.344_136 * u - 0.714_136 * v).clamp(0.0, 255.0) as u8;
            let b = (y_value + 1.772 * u).clamp(0.0, 255.0) as u8;
            image.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
        }
    }

    Ok(DynamicImage::ImageRgb8(image))
}

fn resize_target(src_w: u32, src_h: u32, target_w: u32, target_h: u32) -> (u32, u32) {
    match (target_w, target_h) {
        (0, 0) => (src_w, src_h),
        (0, h) => (((src_w as f64) * (h as f64 / src_h as f64)).round() as u32, h),
        (w, 0) => (w, ((src_h as f64) * (w as f64 / src_w as f64)).round() as u32),
        (w, h) => (w, h),
    }
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use dora_message::{metadata::ArrowTypeInfo, uhlc::HLC};
    use dora_node_api::Metadata;
    use dora_node_api::arrow::datatypes::DataType;
    use tower::ServiceExt;

    fn metadata(parameters: Vec<(&str, Parameter)>) -> Metadata {
        Metadata::from_parameters(
            HLC::default().new_timestamp(),
            ArrowTypeInfo {
                data_type: DataType::UInt8,
                len: 0,
                null_count: 0,
                validity: None,
                offset: 0,
                buffer_offsets: Vec::new(),
                child_data: Vec::new(),
            },
            parameters
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn sample_jpeg() -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(2, 2, Rgb([255, 0, 0])));
        let mut out = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut out, 80);
        encoder.encode_image(&image).unwrap();
        out
    }

    #[test]
    fn rejects_missing_dimensions_for_rgb() {
        let result = extract_frame(
            &[0; 12],
            &metadata(vec![("format", Parameter::String("rgb8".to_string()))]),
            FrameFormat::Jpeg,
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn snapshot_returns_503_without_frame() {
        let app = router(StreamState::new(None));
        let response = app
            .oneshot(axum::http::Request::builder().uri("/snapshot.jpg").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn snapshot_returns_jpeg_when_present() {
        let state = StreamState::new(Some("*".to_string()));
        state
            .update(EncodedFrame {
                jpeg: Arc::new(sample_jpeg()),
                timestamp_ns: Some(1),
            })
            .await;
        let app = router(state);
        let response = app
            .oneshot(axum::http::Request::builder().uri("/snapshot.jpg").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn stream_returns_multipart_headers() {
        let state = StreamState::new(None);
        state
            .update(EncodedFrame {
                jpeg: Arc::new(sample_jpeg()),
                timestamp_ns: Some(1),
            })
            .await;
        let app = router(state);
        let response = app
            .oneshot(axum::http::Request::builder().uri("/stream").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "multipart/x-mixed-replace; boundary=frame"
        );
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(body.starts_with(b"--frame\r\nContent-Type: image/jpeg\r\n"));
    }
}
