use std::{
    num::{NonZeroI32, NonZeroU8},
    path::Path,
};

use crate::{config::TaskRun, ssh::SSHSession, util::with_spinner};

pub struct TaskRunner<'s> {
    pub session: &'s SSHSession,
}

impl<'s> TaskRunner<'s> {
    pub fn new(session: &'s SSHSession) -> Self {
        Self { session }
    }

    pub async fn perform(&self, pwd: &Path, run: &TaskRun) -> Result<(), NonZeroU8> {
        // TODO: Stream the output of the execution
        with_spinner(format!("Running task: {} \x1b[38;5;245m(Sorry, the output streaming is not implemented yet...)\x1b[m", run.name), |mut spinner| async move {
            let runner = self.session.create_exec_channel(
                &format!("cd {} && {}", pwd.to_str().unwrap(), run.run)
            ).await;

            let exit_info = runner.wait_done().await;


            match exit_info.exit_code {
                0 => spinner.stop_with_message(format!("\x1b[0J\x1b[38;5;2m✓ Running task: {} - done\x1b[m", run.name)),
                i => spinner.stop_with_message(format!("\x1b[0J\x1b[38;5;1m✓ Running task: Exited with code {}\x1b[m", i)),
            };

            println!("\n\n\x1b[1m----- Standard output -----\x1b[m");
            println!("\x1b[38;5;14m{}\x1b[m", exit_info.stdout);
            println!("\x1b[1m----- End of Standard output -----\n\x1b[m");

            match exit_info.exit_code {
                0 => Ok(()),
                i => Err(i.try_into().unwrap())
            }
        }).await
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
