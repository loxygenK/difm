use std::{fs::File, io::BufReader};

use ssh2_config::{HostParams, ParseRule};

use crate::ssh::SSHSession;

pub struct RemoteHost {
  hostname: String,
  config: HostParams
}

impl RemoteHost {
  pub fn new(hostname: &str) -> Self {
    let file = File::open("/Users/flisan/.ssh/config").unwrap();
    let mut reader = BufReader::new(file);

    let config = ssh2_config::SshConfig::default()
        .parse(&mut reader, ParseRule::STRICT)
        .unwrap();

    let config = config.query(&hostname);

    Self {
      hostname: hostname.to_string(),
      config
    }
  }

  pub fn open(&self) -> SSHSession {
    SSHSession::open(&self.hostname, &self.config)
  }
}