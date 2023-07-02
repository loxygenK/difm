use std::{num::NonZeroU8, path::Path};

use crate::{config::TaskRun, progress::ProgressView, ssh::SSHSession, util::indent_str};

pub struct TaskRunner<'s> {
    pub session: &'s SSHSession,
}

impl<'s> TaskRunner<'s> {
    pub fn new(session: &'s SSHSession) -> Self {
        Self { session }
    }

    pub async fn perform(&self, pwd: &Path, run: &TaskRun) -> Result<(), NonZeroU8> {
        // TODO: Stream the output of the execution
        ProgressView::with(
            format!("Running task: {}", run.name),
            |mut progress| async move {
                let runner = self
                    .session
                    .create_exec_channel(&format!("cd {} && {}", pwd.to_str().unwrap(), run.run))
                    .await;

                let exit_info = runner.wait_done().await;

                match exit_info.exit_code {
                    0 => progress.success(Some("done")),
                    i => progress.failure(Some(&format!("Exited with code {}\x1b[m", i))),
                };

                println!("\x1b[1m----- Standard output -----\x1b[m");
                println!("\x1b[38;5;14m{}\x1b[m", indent_str(&exit_info.stdout, 1));
                println!("\x1b[1m----- Standard Error  -----\x1b[m");
                println!("\x1b[38;5;11m{}\x1b[m", indent_str(&exit_info.stderr, 1));
                println!("\x1b[1m----- End of Standard output -----\x1b[m");

                match exit_info.exit_code {
                    0 => Ok(()),
                    i => Err(i.try_into().unwrap()),
                }
            },
        )
        .await
    }

    pub async fn perform_task_set<'a>(
        &self,
        pwd: &Path,
        runs: &'a [TaskRun],
    ) -> Result<(), (&'a TaskRun, NonZeroU8)> {
        for run in runs {
            self.perform(pwd, run).await.map_err(|exit| (run, exit))?
        }

        Ok(())
    }
}
