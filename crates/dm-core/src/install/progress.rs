use tokio::sync::mpsc;

use crate::types::{InstallPhase, InstallProgress};

pub(super) fn send_progress(
    tx: &Option<mpsc::UnboundedSender<InstallProgress>>,
    phase: InstallPhase,
    message: &str,
) {
    if let Some(tx) = tx {
        let _ = tx.send(InstallProgress {
            phase,
            message: message.to_string(),
        });
    }
}
