pub(crate) mod discovery;
mod handlers;
mod types;
mod worker;

pub(crate) use self::handlers::*;
pub(crate) use self::types::*;

use std::path::PathBuf;
use std::time::Instant;

use tokio::sync::Mutex;
use tracing::info;

pub struct FaasState {
    pub start_time: Instant,
    pub functions: Mutex<Vec<Function>>,
    pub pools: worker::Pools,
    pub functions_dir: PathBuf,
}

impl FaasState {
    pub fn new(home: &PathBuf) -> anyhow::Result<Self> {
        let functions_dir = discovery::default_functions_dir(home);
        let functions = discovery::discover_functions(&functions_dir)?;
        info!("discovered {} functions", functions.len());
        let idle_timeout_secs = 300;
        Ok(Self {
            start_time: Instant::now(),
            functions: Mutex::new(functions),
            pools: worker::Pools::new(idle_timeout_secs),
            functions_dir,
        })
    }
}
