use std::{path::Path, sync::Arc, fs::File, io::Read, time::Duration};

use tokio::sync::Mutex;

use crate::{fs::{FileTransferList, EntryType}, progress::ProgressView};

use super::{SSHSession, exec::ExecChannel};

#[derive(Debug)]
pub enum FileTransferError {

}

pub async fn send_file(session: &SSHSession, local_source: &Path, remote_dest: &Path) -> Result<(), FileTransferError> {
    let mut content = Vec::new();
    File::open(local_source)
        .unwrap()
        .read_to_end(&mut content)
        .unwrap();

    session.transfer_scp(remote_dest, &content).await;

    Ok(())
}

pub async fn send_directory(session: &SSHSession, transfer_list: FileTransferList) -> Result<(), FileTransferError> {
    let mut progress = ProgressView::new("Enumerating contents");
    progress.start();

    let paths = transfer_list.traverse_dir();
    let progress = Arc::new(Mutex::new(progress));
    let total_items = paths.len();

    progress.lock().await.update_task("Transferring files");

    for (i, dir) in paths.iter().enumerate() {
        match dir.kind {
            EntryType::File => {
                send_file(session, &dir.local_source, &dir.remote_dest).await.unwrap();
            },
            EntryType::Dir => {
                ExecChannel::execute(session, &format!(
                    "mkdir -p {}",
                    dir.remote_dest.to_str().unwrap()
                )).await;
            },
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