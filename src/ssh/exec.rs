use std::{io::Read, sync::Arc, time::Duration};

use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

use super::SSHSession;

pub struct ExecChannel {
    end_notify: Arc<Notify>,
    complete_info: Arc<RwLock<Option<ExecChannelCompleteInfo>>>,
}

#[derive(Debug, Clone)]
pub struct ExecChannelCompleteInfo {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u8,
}

impl ExecChannel {
    pub async fn new(session: &SSHSession, line: &str) -> Self {
        let id = Uuid::new_v4();
        let tag = Self::tag(&id);
        let mut channel = session.create_exec_channel().await;

        // TODO: Sanitize `line`
        channel.exec(&format!("sh -c '{line}'; echo \"{tag}\"")).unwrap();

        let complete_info = Arc::new(RwLock::new(None));
        let end_notify = Arc::new(Notify::new());

        let complete_info_for_thread = complete_info.clone();
        let notify_for_thread = end_notify.clone();

        let new_exec = Self {
            end_notify,
            complete_info,
        };

        tokio::spawn(async move {
            loop {
                let mut stdout = String::new();
                let mut stderr = String::new();

                channel.stream(0).read_to_string(&mut stdout).unwrap();
                channel.stderr().read_to_string(&mut stderr).unwrap();

                if let Some(exit_code) = Self::extract_tag(&id, &stdout) {
                    *(complete_info_for_thread.write().await) = Some(ExecChannelCompleteInfo {
                        stdout,
                        stderr,
                        exit_code,
                    });
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            notify_for_thread.notify_one();
        });

        new_exec
    }

    pub async fn execute(session: &SSHSession, line: &str) -> ExecChannelCompleteInfo {
        Self::new(session, line).await.wait_done().await
    }

    pub async fn wait_done(&self) -> ExecChannelCompleteInfo {
        let notify = self.end_notify.clone();
        notify.notified().await;

        self.complete_info.read().await.clone().unwrap()
    }

    fn tag(id: &Uuid) -> String {
        format!(" [[ END-OF-TASK {} $? ]] ", id.hyphenated())
    }

    fn extract_tag(task_id: &Uuid, new_output: &str) -> Option<u8> {
        let output: Vec<_> = new_output
            .split(' ')
            .skip_while(|segment| *segment != "[[")
            .take(5)
            .collect();

        let ["[[", "END-OF-TASK", id, exit_code, "]]"] = output.as_slice() else {
            return None;
        };

        if *id != task_id.hyphenated().to_string().as_str() {
            return None;
        }

        let Ok(exit_code) = exit_code.parse::<u8>() else {
            return None;
        };

        Some(exit_code)
    }
}
