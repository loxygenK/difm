use std::path::PathBuf;

use crate::ssh::SSHSession;

pub struct SyncSourceConfiguration {
    pub local_source: PathBuf,
    pub remote_dest: PathBuf,
}

pub fn sync_source(config: SyncSourceConfiguration, session: &SSHSession) {}
