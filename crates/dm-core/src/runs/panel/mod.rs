mod model;
mod store;

pub use model::{Asset, AssetFilter, OutputCommand, PaginatedAssets, PanelRun};
pub use store::PanelStore;

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use tempfile::tempdir;

    use super::{AssetFilter, PanelStore};

    #[test]
    fn write_and_query_assets() {
        let dir = tempdir().unwrap();
        let store = PanelStore::open(dir.path(), "run-a").unwrap();

        store
            .write_asset("camera", "text/plain", b"hello panel")
            .unwrap();
        store
            .write_asset("camera", "image/jpeg", &[0xff, 0xd8, 0xff, 0xd9])
            .unwrap();

        let all = store.query_assets(&AssetFilter::default()).unwrap();
        assert_eq!(all.total, 2);
        assert_eq!(all.assets.len(), 2);
        assert_eq!(all.assets[0].storage, "inline");
        assert_eq!(all.assets[0].data.as_deref(), Some("hello panel"));
        assert_eq!(all.assets[1].storage, "file");
        assert!(all.assets[1]
            .path
            .as_deref()
            .unwrap_or_default()
            .ends_with(".jpg"));
    }

    #[test]
    fn write_and_poll_commands() {
        let dir = tempdir().unwrap();
        let store = PanelStore::open(dir.path(), "run-b").unwrap();

        store.write_command("speed", "0.5").unwrap();
        store.write_command("direction", "-1").unwrap();

        let mut since = 0i64;
        let first = store.poll_commands(&mut since).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].output_id, "speed");
        assert_eq!(first[1].output_id, "direction");
        assert_eq!(since, first[1].seq);

        let second = store.poll_commands(&mut since).unwrap();
        assert!(second.is_empty());
    }

    #[test]
    fn list_runs_with_panel() {
        let dir = tempdir().unwrap();
        let s1 = PanelStore::open(dir.path(), "run-1").unwrap();
        s1.write_command("a", "1").unwrap();

        thread::sleep(Duration::from_millis(10));

        let s2 = PanelStore::open(dir.path(), "run-2").unwrap();
        s2.write_asset("cam", "text/plain", b"x").unwrap();

        let runs = PanelStore::list_runs(dir.path()).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].run_id, "run-2");
        assert_eq!(runs[1].run_id, "run-1");
    }
}
