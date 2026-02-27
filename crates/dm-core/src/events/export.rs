use super::Event;

pub(super) fn render_xes(events: &[Event]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<log xes.version="1.0" xes.features="nested-attributes" xmlns="http://www.xes-standard.org/">
  <extension name="Concept" prefix="concept" uri="http://www.xes-standard.org/concept.xesext"/>
  <extension name="Time" prefix="time" uri="http://www.xes-standard.org/time.xesext"/>
  <extension name="Lifecycle" prefix="lifecycle" uri="http://www.xes-standard.org/lifecycle.xesext"/>
"#,
    );

    let mut cases: std::collections::BTreeMap<String, Vec<&Event>> = std::collections::BTreeMap::new();
    for event in events {
        cases.entry(event.case_id.clone()).or_default().push(event);
    }

    for (case_id, trace_events) in &cases {
        xml.push_str(&format!(
            "  <trace>\n    <string key=\"concept:name\" value=\"{}\"/>\n",
            escape_xml(case_id)
        ));

        for event in trace_events {
            xml.push_str("    <event>\n");
            xml.push_str(&format!(
                "      <string key=\"concept:name\" value=\"{}\"/>\n",
                escape_xml(&event.activity)
            ));
            xml.push_str(&format!(
                "      <date key=\"time:timestamp\" value=\"{}\"/>\n",
                escape_xml(&event.timestamp)
            ));
            xml.push_str(&format!(
                "      <string key=\"source\" value=\"{}\"/>\n",
                escape_xml(&event.source)
            ));
            xml.push_str(&format!(
                "      <string key=\"level\" value=\"{}\"/>\n",
                escape_xml(&event.level)
            ));
            if let Some(ref node_id) = event.node_id {
                xml.push_str(&format!(
                    "      <string key=\"node_id\" value=\"{}\"/>\n",
                    escape_xml(node_id)
                ));
            }
            if let Some(ref message) = event.message {
                xml.push_str(&format!(
                    "      <string key=\"message\" value=\"{}\"/>\n",
                    escape_xml(message)
                ));
            }
            xml.push_str("    </event>\n");
        }

        xml.push_str("  </trace>\n");
    }

    xml.push_str("</log>\n");
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
