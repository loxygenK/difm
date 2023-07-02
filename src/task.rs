use std::{io::{BufReader, BufRead, Seek}, path::Path, process::exit, str::from_utf8, time::Duration, num::NonZeroI32};

use crate::{ssh::SSHSession, config::TaskRun, util::{with_spinner, str_buf}};

pub struct TaskRunner<'s> {
    pub session: &'s SSHSession,
}

impl<'s> TaskRunner<'s> {
    pub fn new(session: &'s SSHSession) -> Self {
        Self { session }
    }

    fn read_line(stream: &mut (impl BufRead + Seek)) -> Option<String> {
        let mut buf = String::new();

        println!("==> {:?}", String::from_utf8(stream.fill_buf().unwrap().to_vec()));
        println!("==> {:?}", stream.fill_buf().unwrap());

        if !stream.fill_buf().unwrap().contains(&b'\n') {
            return None;
        };

        println!("\nWaiting for stream");
        let read = stream.read_line(&mut buf).unwrap();
        println!("\nRead!");

        Some(buf)
    }
    
    pub async fn perform(&self, pwd: &Path, run: &TaskRun) -> Result<(), NonZeroI32> {
        println!("===== {} =====", run.name);

        // TODO: Stream the output of the execution
        with_spinner(format!("Running task: {} \x1b[38;5;245m(Sorry, the output streaming is not implemented yet...)\x1b[m", run.name), |mut spinner| async move {
            let runner = self.session.create_exec_channel(
                &format!("cd {} && {}", pwd.to_str().unwrap(), run.run)
            ).await;

            let exit_code = runner.wait_done().await;

            match exit_code {
                0 => {
                    spinner.stop_with_message(format!("Running task: {} - done", run.name));
                    Ok(())
                },
                i => {
                    spinner.stop_with_message(format!("Running task: Exited with code {}", i));
                    Err(i.try_into().unwrap())
                }
            }
        }).await
    }
    
    pub async fn perform_task_set<'a>(&self, pwd: &Path, runs: &'a [TaskRun]) -> Result<(), (&'a TaskRun, NonZeroI32)> {
        for run in runs {
            self.perform(pwd, run).await.map_err(|exit| (run, exit))?
        }

        Ok(())
    }
}