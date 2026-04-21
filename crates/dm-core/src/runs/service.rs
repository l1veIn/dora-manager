#[path = "service_admin.rs"]
mod service_admin;
#[path = "service_metrics.rs"]
pub(crate) mod service_metrics;
#[path = "service_query.rs"]
mod service_query;
#[path = "service_runtime.rs"]
mod service_runtime;
#[path = "service_start.rs"]
mod service_start;
#[path = "service_tests.rs"]
mod service_tests;

use std::path::Path;

use anyhow::Result;

use super::model::RunInstance;
use crate::runs::runtime::RuntimeBackend;

pub use self::service_admin::{clean_runs, delete_run};
pub use self::service_metrics::{collect_all_active_metrics, get_run_metrics};
pub use self::service_query::{
    get_active_run, get_run, list_active_runs, list_runs, list_runs_filtered, read_run_log,
    read_run_log_chunk, read_run_transpiled, read_run_view,
};
pub use self::service_runtime::{
    mark_stop_requested, reconcile_stale_running_runs, refresh_run_statuses, stop_run,
    sync_run_outputs,
};
pub use self::service_start::{
    start_run_from_file, start_run_from_file_with_source_and_strategy,
    start_run_from_file_with_strategy, start_run_from_yaml,
    start_run_from_yaml_with_source_and_strategy, start_run_from_yaml_with_strategy,
};

fn find_active_run_by_name_with_backend<B: RuntimeBackend>(
    home: &Path,
    dataflow_name: &str,
    backend: &B,
) -> Result<Option<RunInstance>> {
    let mut runs = super::repo::list_run_instances(home)?;
    service_runtime::refresh_run_statuses_with_backend(home, &mut runs, backend)?;
    Ok(runs
        .into_iter()
        .find(|run| run.status.is_running() && run.dataflow_name == dataflow_name))
}
