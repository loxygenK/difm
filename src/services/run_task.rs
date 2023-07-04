use crate::{
    adapter::fs::FileTransferList,
    config::{ssh::SSHConfig, ConfigContext, Configuration},
    remote::{integrity::check_file_change, task::TaskRunner, transfer::send_directory},
};

pub async fn run_task(config_ctx: &ConfigContext) {
    let Configuration::TaskDefinition(task) = &config_ctx.config; //  else { unreachable!(); };
    let session = SSHConfig::new(&task.host.name).open();

    let dirs = FileTransferList::new(
        &task.code.location,
        &task.host.base_dir.join(&task.code.dest),
        &task.code.ignore,
        &config_ctx.config_file,
    );

    let entries = check_file_change(&session, &dirs).await.unwrap();

    if entries.is_empty() {
        println!("No files is required to be send");
    } else {
        for entry in &entries {
            println!("- {}", entry);
        }
        send_directory(&session, &entries).await.unwrap();
    }

    TaskRunner::new(&session)
        .perform_task_set(&task.host.base_dir, &task.run)
        .await
        .unwrap();
}
