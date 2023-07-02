use std::{io::Read, sync::Arc, time::Duration};

use ssh2::Channel;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

pub struct ExecChannel {
    channel: Arc<RwLock<Channel>>,
    end_notify: Arc<Notify>,
    id: Uuid,
    line: String,
    stdout: Arc<RwLock<String>>,
    exit_code: Arc<RwLock<Option<u8>>>,
}

pub struct ExecChannelCompleteInfo {
    pub stdout: String,
    pub exit_code: u8,
}

impl ExecChannel {
    pub fn new(mut channel: Channel, line: &str) -> Self {
        let id = Uuid::new_v4();
        let tag = Self::tag(&id);

        // TODO: Sanitize `line`
        channel
            .exec(&format!("sh -c '{line}'; echo \"{tag}\""))
            .unwrap();

        let channel = Arc::new(RwLock::new(channel));
        let stdout = Arc::new(RwLock::new(String::new()));
        let exit_code = Arc::new(RwLock::new(None));
        let end_notify = Arc::new(Notify::new());

        let channel_for_thread = channel.clone();
        let exit_code_for_thread = exit_code.clone();
        let stdout_for_thread = stdout.clone();
        let notify_for_thread = end_notify.clone();

        let new_exec = Self {
            channel,
            end_notify,
            id,
            exit_code,
            line: line.to_string(),
            stdout,
        };

        tokio::spawn(async move {
            loop {
                let mut new_input = String::new();
                channel_for_thread
                    .write()
                    .await
                    .read_to_string(&mut new_input)
                    .unwrap();

                {
                    let mut stdout = stdout_for_thread.write().await;
                    stdout.push_str(&new_input);
                }

                if let Some(exit_code) = Self::extract_tag(&id, &new_input) {
                    *(exit_code_for_thread.write().await) = Some(exit_code);
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            notify_for_thread.notify_one();
        });

        new_exec
    }

    pub async fn stdout(&self) -> String {
        self.stdout.read().await.clone()
    }

    pub async fn wait_done(&self) -> ExecChannelCompleteInfo {
        let notify = self.end_notify.clone();
        notify.notified().await;

        let exit_code = self.exit_code.read().await.unwrap();
        let stdout = self.stdout.read().await.clone();

        ExecChannelCompleteInfo { exit_code, stdout }
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
