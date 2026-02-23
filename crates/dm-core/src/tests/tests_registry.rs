use crate::registry::*;

fn mock_node(id: &str, name: &str, description: &str, tags: &[&str]) -> NodeMeta {
    NodeMeta {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        build: "pip install demo".to_string(),
        system_deps: None,
        inputs: vec!["tick".to_string()],
        outputs: vec!["text".to_string()],
        tags: tags.iter().map(|t| t.to_string()).collect(),
        category: "Test".to_string(),
    }
}

#[test]
fn test_parse_install_yaml() {
    let yaml = "- id: dora-pyaudio\n  build: pip install dora-pyaudio\n  path: dora-pyaudio\n  inputs: \n    tick: <NODE>/audio";
    let config = parse_install_yaml(yaml).unwrap();

    assert_eq!(config.id, "dora-pyaudio");
    assert_eq!(config.build, "pip install dora-pyaudio");
    assert_eq!(config.path, "dora-pyaudio");
    assert_eq!(config.inputs.get("tick"), Some(&"<NODE>/audio".to_string()));
    assert!(config.outputs.is_empty());
}

#[test]
fn test_filter_nodes() {
    let nodes = vec![
        mock_node(
            "dora-whisper",
            "Whisper",
            "Transcribe audio to text",
            &["python", "audio"],
        ),
        mock_node(
            "dora-webcam",
            "Webcam",
            "Capture camera frames",
            &["rust", "image"],
        ),
    ];

    let by_name = filter_nodes(&nodes, "whisper");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].id, "dora-whisper");

    let by_desc = filter_nodes(&nodes, "camera");
    assert_eq!(by_desc.len(), 1);
    assert_eq!(by_desc[0].id, "dora-webcam");

    let by_tag = filter_nodes(&nodes, "audio");
    assert_eq!(by_tag.len(), 1);
    assert_eq!(by_tag[0].id, "dora-whisper");
}

#[test]
fn test_find_node() {
    let nodes = vec![
        mock_node(
            "dora-whisper",
            "Whisper",
            "Transcribe audio to text",
            &["python"],
        ),
        mock_node("dora-webcam", "Webcam", "Capture camera frames", &["rust"]),
    ];

    let found = find_node(&nodes, "dora-webcam").unwrap();
    assert_eq!(found.name, "Webcam");

    assert!(find_node(&nodes, "missing-node").is_none());
}

#[tokio::test]
#[ignore = "network smoke test"]
async fn test_fetch_registry_smoke() {
    let nodes = fetch_registry().await.unwrap();
    assert!(!nodes.is_empty());
    assert!(nodes.iter().any(|n| !n.id.is_empty()));
}
