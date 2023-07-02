use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use ignore::DirEntry;
use ssh2::Session;
use ssh2_config::HostParams;
use tokio::sync::Mutex;

use crate::{
    check,
    ssh::connect::{authenticate, configure_session, try_connection},
    util::{create_spinner, with_spinner},
};

use self::exec::ExecChannel;

mod connect;
mod exec;

#[derive(Debug)]
pub struct CommandExecuteResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct SSHSession(Arc<Mutex<Session>>);
impl SSHSession {
    pub fn open(hostname: &str, params: &HostParams) -> Self {
        let host = params.host_name.as_deref().unwrap_or(hostname);
        let host = if host.contains(':') {
            check!(
                params.port.is_none(),
                "Port {} is ignored, because hostname seems to contain port (it has ':')",
                params.port.unwrap()
            );
            host.to_string()
        } else {
            let port = params.port.unwrap_or(22);
            format!("{}:{}", host, port)
        };

        let stream = with_spinner("Connecting to the host...", |mut spinner| {
            let stream = try_connection(&host).expect("Could not connect to the host");
            spinner.stop_with_message(format!(
                "👋 Connected to {}",
                stream
                    .peer_addr()
                    .map(|addr| addr.to_string())
                    .unwrap_or("[host]".to_string())
            ));

            stream
        });

        let mut session = with_spinner("Configuring the session...", |mut spinner| {
            let mut session = Session::new().expect("Could not create session");
            configure_session(&mut session, params);
            session.set_tcp_stream(stream);
            session.handshake().unwrap();
            spinner.stop_with_message("🤝 Handshaked!");

            session
        });

        authenticate(&mut session, params);

        println!("✅ Connected to the remote server");

        if let Some(banner) = session.banner() {
            println!("----------------------------------");
            println!("{}", banner);
            println!("----------------------------------");
        }

        Self(Arc::new(Mutex::new(session)))
    }

    pub async fn create_exec_channel(&self, command_line: &str) -> ExecChannel {
        ExecChannel::new(
            self.0.clone().lock().await.channel_session().unwrap(),
            command_line,
        )
    }

    async fn send_file(&self, local_source: &Path, remote_dest: &Path) -> Result<()> {
        // println!("\nReading {}", local_source.display());
        let mut content = Vec::new();
        // println!("\nRead {}", local_source.display());
        File::open(local_source)
            .unwrap()
            .read_to_end(&mut content)
            .unwrap();

        // println!("\nSend {}", local_source.display());

        async {
            let session = self.0.lock().await;

            let mut scp_session = session
                .scp_send(remote_dest, 0o644, content.len() as u64, None)
                .unwrap();
            scp_session.write_all(&content).unwrap();
            scp_session.send_eof().unwrap();
            scp_session.wait_eof().unwrap();
            scp_session.close().unwrap();
            scp_session.wait_close().unwrap();
        }
        .await;

        // println!("\nSent {}", local_source.display());

        Ok(())
    }

    pub async fn send_directory(&self, paths: &[DirEntry], remote_dest: &Path) -> Result<()> {
        let mut spinner = create_spinner("Sending contents");
        spinner.start();
        let spinner = Arc::new(Mutex::new(spinner));
        let total_items = paths.len();

        for (i, dir) in paths.iter().enumerate() {
            let file_remote_dest = remote_dest.join(dir.path());

            if dir.file_type().unwrap().is_dir() {
                self.create_exec_channel(&format!(
                    "mkdir -p {}",
                    file_remote_dest.to_str().unwrap()
                ))
                .await
                .wait_done()
                .await;
                continue;
            }

            self.send_file(dir.path(), &file_remote_dest).await.unwrap();
            tokio::time::timeout(Duration::from_millis(10), spinner.lock())
                .await
                .map(|mut spinner| {
                    spinner.set_message(format!(
                        "\x1b[0J{}/{}: {}",
                        i + 1,
                        total_items,
                        dir.path().display()
                    ))
                })
                .ok();
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
        spinner.lock().await.stop_with_message("✓ Sent all files");
        println!();

        Ok(())
    }
}
