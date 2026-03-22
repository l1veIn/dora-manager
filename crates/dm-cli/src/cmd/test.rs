use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::builtin::test_harness;

pub fn harness_serve(auto_trigger: bool, output_ports: &[String]) -> Result<()> {
    test_harness::harness_serve(auto_trigger, output_ports)
}

/// Generate a test dataflow YAML and run it end-to-end.
pub async fn run(
    home: &Path,
    verbose: bool,
    node_id: &str,
    config_overrides: &[String],
    auto_trigger: bool,
    timeout: u64,
) -> Result<()> {
    use dm_core::node::{resolve_dm_json_path, NodePortDirection};

    // 1. Read dm.json
    let meta_path = resolve_dm_json_path(home, node_id).with_context(|| {
        format!(
            "Node '{}' not found. Only nodes with dm.json can be tested.",
            node_id
        )
    })?;
    let meta_content = std::fs::read_to_string(&meta_path)
        .with_context(|| format!("Failed to read {}", meta_path.display()))?;
    let meta: dm_core::node::Node = serde_json::from_str(&meta_content)
        .with_context(|| format!("Invalid dm.json for '{}'", node_id))?;

    // 2. Classify ports
    let input_ports: Vec<_> = meta
        .ports
        .iter()
        .filter(|p| p.direction == NodePortDirection::Input)
        .collect();
    let output_ports: Vec<_> = meta
        .ports
        .iter()
        .filter(|p| p.direction == NodePortDirection::Output)
        .collect();

    // 3. Print header
    let inputs_str: Vec<_> = input_ports
        .iter()
        .map(|p| format!("{}(in)", p.id))
        .collect();
    let outputs_str: Vec<_> = output_ports
        .iter()
        .map(|p| format!("{}(out)", p.id))
        .collect();
    eprintln!(
        "\n{} {} v{}",
        "🧪 Testing:".bold(),
        node_id.bold().cyan(),
        meta.version
    );
    eprintln!(
        "   Ports: {} → {}",
        inputs_str.join(" ").dimmed(),
        outputs_str.join(" ").dimmed()
    );
    eprintln!("{}", "─".repeat(50));

    // 4. Build config block from schema defaults + overrides
    let mut config_map = serde_json::Map::new();
    if let Some(schema) = &meta.config_schema {
        if let Some(obj) = schema.as_object() {
            for (key, field) in obj {
                if let Some(default) = field.get("default") {
                    config_map.insert(key.clone(), default.clone());
                }
            }
        }
    }
    for kv in config_overrides {
        if let Some((k, v)) = kv.split_once('=') {
            config_map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
        } else {
            bail!("Invalid --config format: '{}'. Expected KEY=VALUE", kv);
        }
    }

    // 5. Generate YAML
    let harness_input_ports: Vec<_> = input_ports
        .iter()
        .filter(|p| p.id != "tick")
        .map(|p| p.id.as_str())
        .collect();
    let harness_output_ports_csv = harness_input_ports.join(",");

    let mut yaml = String::new();
    yaml.push_str("nodes:\n");

    // SUT node
    yaml.push_str(&format!("  - id: sut\n    node: {}\n", node_id));

    // SUT inputs
    if !input_ports.is_empty() {
        yaml.push_str("    inputs:\n");
        for port in &input_ports {
            if port.id == "tick" {
                yaml.push_str("      tick: dora/timer/millis/2000\n");
            } else {
                yaml.push_str(&format!("      {}: harness/{}\n", port.id, port.id));
            }
        }
    }

    // SUT outputs
    if !output_ports.is_empty() {
        yaml.push_str("    outputs:\n");
        for port in &output_ports {
            yaml.push_str(&format!("      - {}\n", port.id));
        }
    }

    // SUT config
    if !config_map.is_empty() {
        yaml.push_str("    config:\n");
        for (k, v) in &config_map {
            let v_str = match v {
                serde_json::Value::String(s) => format!("\"{}\"", s),
                other => other.to_string(),
            };
            yaml.push_str(&format!("      {}: {}\n", k, v_str));
        }
    }

    // Harness node
    yaml.push_str("\n  - id: harness\n    node: dm-test-harness\n");

    // Harness inputs (subscribe to SUT outputs)
    if !output_ports.is_empty() {
        yaml.push_str("    inputs:\n");
        for port in &output_ports {
            yaml.push_str(&format!("      {}: sut/{}\n", port.id, port.id));
        }
    }

    // Harness outputs (feed SUT inputs)
    if !harness_input_ports.is_empty() {
        yaml.push_str("    outputs:\n");
        for port_id in &harness_input_ports {
            yaml.push_str(&format!("      - {}\n", port_id));
        }
    }

    // Harness env (pass auto-trigger and output ports)
    yaml.push_str(&format!(
        "    env:\n      DM_TEST_AUTO_TRIGGER: \"{}\"\n      DM_TEST_OUTPUT_PORTS: \"{}\"\n",
        auto_trigger, harness_output_ports_csv
    ));

    if verbose {
        eprintln!("\n--- Generated YAML ---\n{}\n--- End YAML ---\n", yaml);
    }

    // 6. Ensure dora is running
    dm_core::ensure_runtime_up(home, verbose).await?;

    // 7. Start the test dataflow
    let dataflow_name = format!("dm-test-{}", node_id);
    let result = dm_core::runs::start_run_from_yaml_with_source_and_strategy(
        home,
        &yaml,
        &dataflow_name,
        dm_core::runs::RunSource::Cli,
        dm_core::runs::StartConflictStrategy::StopAndRestart,
    )
    .await?;

    let run_id = result.run.run_id.clone();
    eprintln!(
        "{} Run started: {} ({})",
        "✅".green(),
        run_id.dimmed(),
        dataflow_name
    );

    // 8. Stream SUT logs (follow mode)
    let home_owned = home.to_path_buf();
    let run_id_owned = run_id.clone();
    let log_handle = tokio::spawn(async move {
        let mut offset = 0u64;
        loop {
            match dm_core::runs::read_run_log_chunk(&home_owned, &run_id_owned, "sut", offset) {
                Ok(chunk) => {
                    if !chunk.content.is_empty() {
                        // Print each line with [LOG] prefix
                        for line in chunk.content.lines() {
                            eprintln!("{} {}", "[LOG]".dimmed(), line);
                        }
                    }
                    let no_progress = chunk.next_offset == offset;
                    offset = chunk.next_offset;
                    if chunk.finished && no_progress {
                        break;
                    }
                }
                Err(_) => {
                    // Log file might not exist yet
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    });

    // 9. Wait for timeout or Ctrl+C
    if timeout > 0 {
        eprintln!("{} Auto-exit in {}s...", "⏱".dimmed(), timeout);
        tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;
    } else {
        eprintln!("{} Interactive mode. Press Ctrl+C to stop.\n", "ℹ".cyan());
        // Wait for ctrl+c
        tokio::signal::ctrl_c().await.ok();
        eprintln!();
    }

    // 10. Cleanup
    eprintln!("{} Stopping test run...", "→".cyan());
    let _ = dm_core::runs::stop_run(home, &run_id).await;
    log_handle.abort();
    let _ = log_handle.await;

    dm_core::auto_down_if_idle(home, false).await;
    eprintln!("{} Test ended.\n", "🧪");

    Ok(())
}
