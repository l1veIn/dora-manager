use std::path::Path;

use anyhow::Result;

use crate::runs::repo;

pub fn delete_run(home: &Path, run_id: &str) -> Result<()> {
    repo::delete_run(home, run_id)?;
    let store = crate::events::EventStore::open(home)?;
    let _ = store.delete_by_case_id(run_id);
    Ok(())
}

pub fn clean_runs(home: &Path, keep: usize) -> Result<u32> {
    let all = super::service_query::list_runs(home, 10000, 0)?;
    let mut deleted = 0u32;
    if all.runs.len() > keep {
        for run in &all.runs[keep..] {
            if let Err(e) = delete_run(home, &run.id) {
                eprintln!("Warning: failed to clean run {}: {}", run.id, e);
            } else {
                deleted += 1;
            }
        }
    }
    Ok(deleted)
}
