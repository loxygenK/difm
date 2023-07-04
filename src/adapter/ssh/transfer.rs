use std::{fs::File, io::Read, path::Path};

use super::SSHSession;

#[derive(Debug)]
pub enum FileTransferError {}

pub async fn transfer_file(
    session: &SSHSession,
    local_source: &Path,
    remote_dest: &Path,
) -> Result<(), FileTransferError> {
    let mut content = Vec::new();
    File::open(local_source)
        .unwrap()
        .read_to_end(&mut content)
        .unwrap();

    session.transfer_scp(remote_dest, &content).await;

    Ok(())
}
