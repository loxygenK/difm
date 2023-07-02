use std::{io::{Read, Write}, path::{PathBuf, Path}, fs::File, sync::Arc, time::Duration};

use anyhow::{Result, anyhow, bail, Context};
use futures::StreamExt;
use glob::glob;
use ignore::DirEntry;
use spinners_rs::Spinner;
use ssh2::{Session, Channel};
use ssh2_config::HostParams;
use tokio::sync::{Mutex, Notify, RwLock};
use uuid::Uuid;

use crate::{check, util::{with_spinner, create_spinner}, ssh::connect::{try_connection, configure_session, authenticate}};

mod connect;

#[derive(Debug)]
pub struct CommandExecuteResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub enum Location {
    Local(PathBuf),
    Remote(PathBuf)
}

pub struct SSHSession(Arc<Mutex<Session>>);
impl SSHSession {
    pub fn open(hostname: &str, params: &HostParams) -> Self {
        let host = params.host_name.as_deref().unwrap_or(hostname);
        let host = if host.contains(':') {
            check!(params.port.is_none(), "Port {} is ignored, because hostname seems to contain port (it has ':')", params.port.unwrap());
            host.to_string()
        } else {
            let port = params.port.unwrap_or(22);
            format!("{}:{}", host, port)
        };
        
        let stream = with_spinner(
            "Connecting to the host...",
            |mut spinner| {
                let stream = try_connection(&host).expect("Could not connect to the host");
                spinner.stop_with_message(
                    format!(
                        "👋 Connected to {}",
                        stream.peer_addr()
                            .map(|addr| addr.to_string())
                            .unwrap_or("[host]".to_string())
                    )
                );

                stream
            }
        );
        println!();
        
        let mut session = with_spinner(
            "Configuring the session...", 
            |mut spinner| {
                let mut session = Session::new().expect("Could not create session");
                configure_session(&mut session, &params);
                session.set_tcp_stream(stream);
                session.handshake().unwrap();
                spinner.stop_with_message("🤝 Handshaked!");

                session
            }
        );
        println!();
        
        authenticate(&mut session, params);
        
        println!("✅ Connected to the remote server");
        
        if let Some(banner) = session.banner() {
            println!("----------------------------------");
            println!("{}", banner);
            println!("----------------------------------");
        }
        
        Self(Arc::new(Mutex::new(session)))
    }

    pub fn shared_clone(&self) -> Self {
        Self(self.0.clone())
    }

    pub async fn create_exec_channel(&self, command_line: &str) -> ExecChannel {
        ExecChannel::new(self.0.clone().lock().await.channel_session().unwrap(), command_line)
    }

    async fn send_file(&self, local_source: &Path, remote_dest: &Path) -> Result<()> {
        // println!("\nReading {}", local_source.display());
        let mut content = Vec::new();
        // println!("\nRead {}", local_source.display());
        File::open(&local_source).unwrap().read_to_end(&mut content).unwrap();

        // println!("\nSend {}", local_source.display());

        async {
            let session = self.0.lock().await;

            let mut scp_session = session.scp_send(&remote_dest, 0o644, content.len() as u64, None).unwrap();
            scp_session.write_all(&mut content).unwrap();
            scp_session.send_eof().unwrap();
            scp_session.wait_eof().unwrap();
            scp_session.close().unwrap();
            scp_session.wait_close().unwrap();
        }.await;

        // println!("\nSent {}", local_source.display());

        Ok(())
    }

    pub async fn send_directory(&self, paths: &[DirEntry], remote_dest: &Path) -> Result<()> {
        let mut spinner = create_spinner("Sending contents");
        spinner.start();
        let spinner = Arc::new(Mutex::new(spinner));
        let total_items = paths.len();

        struct SendContext {
            index: usize,
            dir: DirEntry,
            dir_display: String,
            spinner: Arc<Mutex<Spinner>>,
            shared_session: SSHSession,
        }

        let ctxs = paths.iter()
            .enumerate()
            .map(|(i, dir)| {
                let dir_display = dir.path().display().to_string();
                SendContext {
                    index: i,
                    dir: dir.clone(),
                    dir_display,
                    spinner: spinner.clone(),
                    shared_session: self.shared_clone(),
                }
            })
            .collect::<Vec<_>>();

        futures::stream::iter(ctxs)
            .map(|ctx| async move {
                let file_remote_dest = remote_dest.join(ctx.dir.path());

                if ctx.dir.file_type().unwrap().is_dir() {
                    self.create_exec_channel(&format!("mkdir -p {}", file_remote_dest.to_str().unwrap()))
                        .await
                        .wait_done()
                        .await;
                    return;
                }

                tokio::spawn(async move {
                    ctx.shared_session.send_file(ctx.dir.path(), &file_remote_dest).await.unwrap();
                    tokio::time::timeout(Duration::from_millis(10), ctx.spinner.lock())
                        .await
                        .map(|mut spinner| spinner.set_message(format!("\x1b[0J{}/{}: {}", ctx.index + 1, total_items, ctx.dir_display)))
                        .ok();
                }).await.unwrap();
            })
            // TODO: I wanted to make file transfer parallel, but SCP itself does not have support
            .buffer_unordered(1)
            .collect::<Vec<_>>()
            .await;
        Ok(())
    }
}

pub struct ExecChannel {
    channel: Arc<Channel>,
    end_notify: Arc<Notify>,
    id: Uuid,
    line: String,
}
impl ExecChannel {
    pub fn new(mut channel: Channel, line: &str) -> Self {
        let id = Uuid::new_v4();

        let channel = Arc::new(channel);
        let end_notify = Arc::new(Notify::new());

        Self {
            channel: channel.clone(),
            end_notify: channel.clone(),
            id,
            line: line.to_string(),
        }

        channel.exec(line).unwrap();

        let channel_for_thread = channel.clone();
        let notify_for_thread = end_notify.clone();

        tokio::spawn(async move {
            while !channel_for_thread.() {
                println!("EOF => {}", channel_for_thread.wait_close());
                tokio::time::sleep(Duration::from_micros(300)).await
            }

            notify_for_thread.notify_one();
        });

    }

    pub fn stdout(&self) -> impl Read {
        self.channel.stream(0)
    }

    pub fn stderr(&self) -> impl Read {
        self.channel.stderr()
    }

    pub fn done(&self) -> bool {
        self.channel.eof()
    }

    pub async fn wait_done(&self) -> i32 {
        let notify = self.end_notify.clone();
        notify.notified().await;

        self.channel.exit_status().unwrap()
    }

    pub fn stdout_all(&self) -> String {
        let mut stdout = String::new();
        self.stdout().read_to_string(&mut stdout).unwrap();

        stdout
    }

    pub fn stderr_all(&self) -> String {
        let mut stderr = String::new();
        self.stderr().read_to_string(&mut stderr).unwrap();

        stderr
    }

    fn tag(&self) -> String {
        format!("[[--- END OF TASK <{}> ---]]", self.id.hyphenated().to_string())
    }
}