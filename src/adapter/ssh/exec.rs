use std::{cell::OnceCell, io::Read, process::exit, sync::Arc, time::Duration};

use regex::Regex;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

use super::SSHSession;

pub const TAG_REGEX_PATTERN: &str = r"\[\[ END-OF-TASK (.{36}) (\d) ]]";
pub const TAG_REGEX: OnceCell<Regex> = OnceCell::new();

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
        channel
            .exec(&format!("sh -c '{line}'; echo \"{tag}\""))
            .unwrap();

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
                        stdout: Self::remove_tag(&stdout),
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
        let binding = TAG_REGEX;
        let regex = binding.get_or_init(|| Regex::new(TAG_REGEX_PATTERN).unwrap());
        let capture = regex.captures(new_output)?;

        if !capture
            .get(1)
            .is_some_and(|matched| matched.as_str() == task_id.hyphenated().to_string())
        {
            return None;
        }

        let Some(exit_code) = capture.get(2).and_then(|exit_code| exit_code.as_str().parse::<u8>().ok()) else {
            println!("Exit code parse fail");
            return None;
        };

        Some(exit_code)
    }

    fn remove_tag(new_output: &str) -> String {
        let binding = TAG_REGEX;
        let regex = binding.get_or_init(|| Regex::new(TAG_REGEX_PATTERN).unwrap());
        regex.replace(new_output, "").to_string()
    }
}
