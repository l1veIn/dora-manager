#![cfg(not(target_os = "windows"))]

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

struct ServicedChild(Child);

impl Drop for ServicedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn serviced_accepts_json_over_socket() {
    let temp = tempfile::tempdir().unwrap();
    let socket_path = temp.path().join("service.sock");
    let child = Command::new(env!("CARGO_BIN_EXE_dm-serviced"))
        .env("DM_HOME", temp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _child = ServicedChild(child);

    wait_for_socket(&socket_path).await;

    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    write_half
        .write_all(br#"{"service_id":"add","method":"run","input":{"x":2,"y":3},"context":{}}"#)
        .await
        .unwrap();
    write_half.write_all(b"\n").await.unwrap();
    write_half.flush().await.unwrap();

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(line.trim()).unwrap();

    assert_eq!(response["service_id"], "add");
    assert_eq!(response["method"], "run");
    assert_eq!(response["output"]["result"], 5);
}

async fn wait_for_socket(socket_path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if socket_path.exists() && UnixStream::connect(socket_path).await.is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "dm-serviced socket did not become ready"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
