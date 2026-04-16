//! Node registry - static mapping of node IDs to their GitHub source URLs.

/// Static registry mapping node IDs to their GitHub repository URLs.
/// This allows automatic resolution of missing nodes during dataflow execution.
const NODE_REGISTRY: &[(&str, &str)] = &[
    // Core dora-rs nodes
    ("dora-echo", "https://github.com/dora-rs/dora-echo.git"),
    ("dora-yolo", "https://github.com/dora-rs/dora-yolo.git"),
    ("dora-keyboard", "https://github.com/dora-rs/dora-keyboard.git"),
    ("dora-microphone", "https://github.com/dora-rs/dora-microphone.git"),
    ("dora-qwen", "https://github.com/dora-rs/dora-qwen.git"),
    
    // Computer vision nodes
    ("opencv-video-capture", "https://github.com/dora-rs/opencv-video-capture.git"),
    ("opencv-plot", "https://github.com/dora-rs/opencv-plot.git"),
    
    // Audio/speech nodes
    ("dora-distil-whisper", "https://github.com/dora-rs/dora-distil-whisper.git"),
    ("dora-vad", "https://github.com/dora-rs/dora-vad.git"),
    ("dora-kokoro-tts", "https://github.com/dora-rs/dora-kokoro-tts.git"),
    ("dora-outtetts", "https://github.com/dora-rs/dora-outtetts.git"),
    ("dora-pyaudio", "https://github.com/dora-rs/dora-pyaudio.git"),
    
    // LLM/AI nodes
    ("dora-qwen2-5-vl", "https://github.com/dora-rs/dora-qwen2-5-vl.git"),
    ("dora-internvl", "https://github.com/dora-rs/dora-internvl.git"),
    ("dora-transformers", "https://github.com/dora-rs/dora-transformers.git"),
    ("dora-llama-cpp-python", "https://github.com/dora-rs/dora-llama-cpp-python.git"),
    ("dora-sam2", "https://github.com/dora-rs/dora-sam2.git"),
    
    // Robotics nodes
    ("dora-piper", "https://github.com/dora-rs/dora-piper.git"),
    ("dora-reachy1", "https://github.com/dora-rs/dora-reachy1.git"),
    ("dora-reachy2", "https://github.com/dora-rs/dora-reachy2.git"),
    ("dora-ugv", "https://github.com/dora-rs/dora-ugv.git"),
    ("dora-kit-car", "https://github.com/dora-rs/dora-kit-car.git"),
    ("dora-mujoco", "https://github.com/dora-rs/dora-mujoco.git"),
    ("dora-mujoco-husky", "https://github.com/dora-rs/dora-mujoco-husky.git"),
    ("dora-rustypot", "https://github.com/dora-rs/dora-rustypot.git"),
    ("dora-policy-inference", "https://github.com/dora-rs/dora-policy-inference.git"),
    
    // Sensor nodes
    ("dora-pyrealsense", "https://github.com/dora-rs/dora-pyrealsense.git"),
    ("dora-pyorbbecksdk", "https://github.com/dora-rs/dora-pyorbbecksdk.git"),
    ("dora-ios-lidar", "https://github.com/dora-rs/dora-ios-lidar.git"),
    ("dora-mediapipe", "https://github.com/dora-rs/dora-mediapipe.git"),
    ("dora-cotracker", "https://github.com/dora-rs/dora-cotracker.git"),
    
    // Media/video nodes
    ("dora-dav1d", "https://github.com/dora-rs/dora-dav1d.git"),
    ("dora-rav1e", "https://github.com/dora-rs/dora-rav1e.git"),
    ("video-encoder", "https://github.com/dora-rs/video-encoder.git"),
    ("dora-vggt", "https://github.com/dora-rs/dora-vggt.git"),
    
    // UI/dashboard nodes
    ("dora-gradio", "https://github.com/dora-rs/dora-gradio.git"),
    ("lerobot-dashboard", "https://github.com/dora-rs/lerobot-dashboard.git"),
    ("dora-teleop-xr", "https://github.com/dora-rs/dora-teleop-xr.git"),
    
    // Recording/playback nodes
    ("dora-record", "https://github.com/dora-rs/dora-record.git"),
    ("dora-dataset-record", "https://github.com/dora-rs/dora-dataset-record.git"),
    ("dora-parquet-recorder", "https://github.com/dora-rs/dora-parquet-recorder.git"),
    ("llama-factory-recorder", "https://github.com/dora-rs/llama-factory-recorder.git"),
    ("replay-client", "https://github.com/dora-rs/replay-client.git"),
    
    // Terminal/IO nodes
    ("terminal-input", "https://github.com/dora-rs/terminal-input.git"),
    ("terminal-print", "https://github.com/dora-rs/terminal-print.git"),
    
    // MCP nodes
    ("dora-mcp-host", "https://github.com/dora-rs/dora-mcp-host.git"),
    ("dora-mcp-server", "https://github.com/dora-rs/dora-mcp-server.git"),
    ("dora-openai-server", "https://github.com/dora-rs/dora-openai-server.git"),
    ("dora-openai-websocket", "https://github.com/dora-rs/dora-openai-websocket.git"),
    ("openai-proxy-server", "https://github.com/dora-rs/openai-proxy-server.git"),
    
    // Misc nodes
    ("dora-rerun", "https://github.com/dora-rs/dora-rerun.git"),
    ("dora-funasr", "https://github.com/dora-rs/dora-funasr.git"),
    ("dora-parler", "https://github.com/dora-rs/dora-parler.git"),
    ("dora-pytorch-kinematics", "https://github.com/dora-rs/dora-pytorch-kinematics.git"),
    ("dora-object-to-pose", "https://github.com/dora-rs/dora-object-to-pose.git"),
    ("dora-mistral-rs", "https://github.com/dora-rs/dora-mistral-rs.git"),
    ("dora-qwen-omni", "https://github.com/dora-rs/dora-qwen-omni.git"),
    
    // Third-party nodes
    ("gamepad", "https://github.com/dora-rs/gamepad.git"),
    ("lebai-client", "https://github.com/dora-rs/lebai-client.git"),
    ("mujoco-client", "https://github.com/dora-rs/mujoco-client.git"),
    ("pyarrow-assert", "https://github.com/dora-rs/pyarrow-assert.git"),
    ("pyarrow-sender", "https://github.com/dora-rs/pyarrow-sender.git"),
];

/// Resolve a node ID to its GitHub repository URL.
/// Returns None if the node is not found in the registry.
pub fn resolve_node_source(node_id: &str) -> Option<&'static str> {
    NODE_REGISTRY
        .iter()
        .find(|(id, _)| *id == node_id)
        .map(|(_, url)| *url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_nodes() {
        assert_eq!(
            resolve_node_source("dora-echo"),
            Some("https://github.com/dora-rs/dora-echo.git")
        );
        assert_eq!(
            resolve_node_source("dora-yolo"),
            Some("https://github.com/dora-rs/dora-yolo.git")
        );
        assert_eq!(
            resolve_node_source("opencv-video-capture"),
            Some("https://github.com/dora-rs/opencv-video-capture.git")
        );
    }

    #[test]
    fn resolve_unknown_node_returns_none() {
        assert_eq!(resolve_node_source("non-existent-node"), None);
    }

    #[test]
    fn registry_contains_common_nodes() {
        // Ensure key nodes are present
        assert!(resolve_node_source("dora-keyboard").is_some());
        assert!(resolve_node_source("dora-microphone").is_some());
        assert!(resolve_node_source("dora-qwen").is_some());
        assert!(resolve_node_source("dora-piper").is_some());
    }
}
