//! Unified Event Store - XES-compatible observability infrastructure
//!
//! All observability data (system logs, dataflow execution logs, HTTP request logs,
//! frontend analytics, CI metrics) is stored as events in a single SQLite table.

mod builder;
mod export;
mod model;
mod op;
mod store;

pub use builder::EventBuilder;
pub use model::{Event, EventFilter, EventLevel, EventSource};
pub use op::{try_emit, OperationEvent};
pub use store::EventStore;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_store() -> (tempfile::TempDir, EventStore) {
        let dir = tempdir().unwrap();
        let store = EventStore::open(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn emit_and_query() {
        let (_dir, store) = test_store();

        let event = EventBuilder::new(EventSource::Core, "node.install")
            .case_id("session_001")
            .message("Installing opencv-video-capture")
            .attr("version", "0.4.1")
            .build();

        let id = store.emit(&event).unwrap();
        assert!(id > 0);

        let results = store.query(&EventFilter::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].activity, "node.install");
        assert_eq!(results[0].source, "core");
    }

    #[test]
    fn filter_by_source() {
        let (_dir, store) = test_store();

        store
            .emit(&EventBuilder::new(EventSource::Core, "version.switch").case_id("s1").build())
            .unwrap();
        store
            .emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df1").build())
            .unwrap();
        store
            .emit(&EventBuilder::new(EventSource::Frontend, "ui.click").case_id("u1").build())
            .unwrap();

        let core_events = store
            .query(&EventFilter {
                source: Some("core".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(core_events.len(), 1);

        let all_events = store.query(&EventFilter::default()).unwrap();
        assert_eq!(all_events.len(), 3);
    }

    #[test]
    fn filter_by_case_id() {
        let (_dir, store) = test_store();

        store
            .emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df_abc").build())
            .unwrap();
        store
            .emit(&EventBuilder::new(EventSource::Dataflow, "node.output").case_id("df_abc").build())
            .unwrap();
        store
            .emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df_xyz").build())
            .unwrap();

        let results = store
            .query(&EventFilter {
                case_id: Some("df_abc".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn count_events() {
        let (_dir, store) = test_store();

        for i in 0..10 {
            store
                .emit(
                    &EventBuilder::new(EventSource::Server, "http.request")
                        .case_id(format!("req_{}", i))
                        .build(),
                )
                .unwrap();
        }

        let total = store.count(&EventFilter::default()).unwrap();
        assert_eq!(total, 10);

        let server_count = store
            .count(&EventFilter {
                source: Some("server".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(server_count, 10);
    }

    #[test]
    fn export_xes_format() {
        let (_dir, store) = test_store();

        store
            .emit(&EventBuilder::new(EventSource::Core, "node.install").case_id("s1").message("test").build())
            .unwrap();
        store
            .emit(&EventBuilder::new(EventSource::Core, "node.start").case_id("s1").build())
            .unwrap();

        let xes = store.export_xes(&EventFilter::default()).unwrap();
        assert!(xes.contains("xes.version"));
        assert!(xes.contains("concept:name"));
        assert!(xes.contains("node.install"));
        assert!(xes.contains("node.start"));
    }

    #[test]
    fn event_builder_attributes() {
        let event = EventBuilder::new(EventSource::Ci, "clippy.warn")
            .case_id("commit_abc123")
            .level(EventLevel::Warn)
            .message("unused variable")
            .attr("file", "src/main.rs")
            .attr("line", 42)
            .attr("severity", "warning")
            .build();

        assert_eq!(event.source, "ci");
        assert_eq!(event.level, "warn");

        let attrs: serde_json::Value = serde_json::from_str(event.attributes.as_ref().unwrap()).unwrap();
        assert_eq!(attrs["file"], "src/main.rs");
        assert_eq!(attrs["line"], 42);
    }
}
