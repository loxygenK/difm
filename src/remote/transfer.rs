use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{
    adapter::{
        fs::{Entry, EntryType},
        ssh::{
            exec::ExecChannel,
            transfer::{transfer_file, FileTransferError},
            SSHSession,
        },
    },
    progress::ProgressView,
};

pub async fn send_directory(
    session: &SSHSession,
    transfer_entries: &[Entry],
) -> Result<(), FileTransferError> {
    let mut progress = ProgressView::new("Enumerating contents");
    progress.start();

    let progress = Arc::new(Mutex::new(progress));
    let total_items = transfer_entries.len();

    progress.lock().await.update_task("Transferring files");

    for (i, dir) in transfer_entries.iter().enumerate() {
        match dir.kind {
            EntryType::File => {
                transfer_file(session, &dir.local_source, &dir.remote_dest)
                    .await
                    .unwrap();
            }
            EntryType::Dir => {
                ExecChannel::execute(
                    session,
                    &format!("mkdir -p {}", dir.remote_dest.to_str().unwrap()),
                )
                .await;
            }
        }

        progress.lock().await.report_intermediate(
            (i + 1, total_items),
            Some(&dir.local_source.display().to_string()),
        );
    }

    tokio::time::sleep(Duration::from_millis(20)).await;
    progress.lock().await.success(Some("Sent all files"));

    Ok(())
}
