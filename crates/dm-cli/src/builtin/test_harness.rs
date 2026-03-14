/// dm-test-harness — built-in dora node for interactive node testing.
///
/// Runs as a companion node alongside the SUT (System Under Test).
/// - Reads user commands from stdin → sends as dora outputs to SUT input ports
/// - Receives SUT outputs via dora inputs → pretty-prints to stderr
///
/// Dual-thread architecture (same pattern as panel_serve):
///   Thread 1 (spawned): dora event reader → format + print
///   Thread 2 (main):    stdin reader → parse + send_output

use anyhow::{Context, Result};
use colored::Colorize;

use dora_node_api::arrow::array::StringArray;
use dora_node_api::{DoraNode, Event};

/// Entry point called by transpiler via `dm test harness-serve`.
pub fn harness_serve(auto_trigger: bool, output_ports: &[String]) -> Result<()> {
    let (mut node, mut events) =
        DoraNode::init_from_env().map_err(|e| anyhow::anyhow!("Failed to init harness node: {e}"))?;

    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // -- Thread 1: Dora event reader (receives SUT outputs) --
    let stop_flag = should_stop.clone();
    let reader = std::thread::spawn(move || {
        while let Some(event) = events.recv() {
            match event {
                Event::Input { id, metadata, data } => {
                    let port_id = AsRef::<str>::as_ref(&id);
                    let type_hint = extract_type_hint(&metadata, &data);

                    if type_hint.starts_with("text/") || type_hint == "application/json" {
                        let text = String::try_from(&data).unwrap_or_else(|_| format!("{data:?}"));
                        eprintln!("{} {}", format!("[OUT:{}]", port_id).cyan(), text);
                    } else {
                        let size = arrow_data_size(&data);
                        eprintln!(
                            "{} <{} bytes {}>",
                            format!("[OUT:{}]", port_id).cyan(),
                            size,
                            type_hint.dimmed()
                        );
                    }
                }
                Event::Stop(_) => {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }
        }
        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    // -- Auto-trigger: send one empty event to each output port --
    if auto_trigger {
        for port in output_ports {
            eprintln!(
                "{} Auto-triggering port: {}",
                "→".cyan(),
                port.bold()
            );
            node.send_output(
                port.clone().into(),
                Default::default(),
                StringArray::from(vec!["trigger"]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to auto-trigger port '{}'", port))?;
        }
    }

    // -- Main loop: stdin reader --
    let stdin = std::io::stdin();
    loop {
        if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }

        if cmd == "@quit" {
            break;
        }

        // Parse command: "<port> [value]" or "@<port> <filepath>"
        if let Some(rest) = cmd.strip_prefix('@') {
            // File injection: @<port> <filepath>
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() != 2 {
                eprintln!("{} Usage: @<port> <filepath>", "⚠".yellow());
                continue;
            }
            let port = parts[0];
            let filepath = parts[1];
            match std::fs::read(filepath) {
                Ok(bytes) => {
                    use dora_node_api::arrow::array::UInt8Array;
                    eprintln!(
                        "{} Sending {} bytes from {} to {}",
                        "→".cyan(),
                        bytes.len(),
                        filepath.dimmed(),
                        port.bold()
                    );
                    node.send_output(
                        port.to_string().into(),
                        Default::default(),
                        UInt8Array::from(bytes),
                    )
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                }
                Err(e) => {
                    eprintln!("{} Failed to read {}: {}", "❌".red(), filepath, e);
                }
            }
        } else {
            // Text command: "<port>" or "<port> <value>"
            let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
            let port = parts[0];
            let value = parts.get(1).copied().unwrap_or("trigger");
            node.send_output(
                port.to_string().into(),
                Default::default(),
                StringArray::from(vec![value]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
    }

    reader
        .join()
        .map_err(|_| anyhow::anyhow!("Harness event reader thread panicked"))?;

    Ok(())
}

fn extract_type_hint(
    metadata: &dora_node_api::Metadata,
    data: &dora_node_api::ArrowData,
) -> String {
    use dora_node_api::arrow::datatypes::DataType;

    if let Some(dora_node_api::Parameter::String(ct)) = metadata.parameters.get("content_type") {
        return ct.clone();
    }
    match data.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => "text/plain".to_string(),
        DataType::Binary | DataType::LargeBinary | DataType::UInt8 => {
            "application/octet-stream".to_string()
        }
        _ => format!("application/x-arrow+{:?}", data.data_type()).to_ascii_lowercase(),
    }
}

fn arrow_data_size(data: &dora_node_api::ArrowData) -> usize {
    use dora_node_api::arrow::datatypes::DataType;
    match data.data_type() {
        DataType::UInt8 => Vec::<u8>::try_from(data).map(|v| v.len()).unwrap_or(0),
        DataType::Utf8 | DataType::LargeUtf8 => {
            String::try_from(data).map(|s| s.len()).unwrap_or(0)
        }
        _ => 0,
    }
}
