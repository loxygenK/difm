use crate::config::{ConfigContext, Configuration};
use crate::fs::FileTransferList;
use crate::remote::RemoteHost;
use crate::ssh::transfer::send_directory;
use crate::task::TaskRunner;

pub async fn run_task(config_ctx: &ConfigContext) {
    let Configuration::TaskDefinition(task) = &config_ctx.config; //  else { unreachable!(); };

    let session = RemoteHost::new(&task.host.name).open();

    let dirs = FileTransferList::new(
        &task.code.location,
        &task.host.base_dir.join(&task.code.dest),
        &task.code.ignore,
        &config_ctx.config_file,
    );

    send_directory(&session, dirs)
        .await
        .unwrap();

    TaskRunner::new(&session)
        .perform_task_set(&task.host.base_dir, &task.run)
        .await
        .unwrap();
}
