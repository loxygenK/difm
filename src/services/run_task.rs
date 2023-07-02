use std::path::{Path, PathBuf};

use crate::config::{ConfigContext, Configuration};
use crate::fs::traverse_dir;
use crate::remote::RemoteHost;
use crate::task::TaskRunner;

pub async fn run_task(config_ctx: &ConfigContext) {
    let Configuration::TaskDefinition(task) = &config_ctx.config; //  else { unreachable!(); };

    let session = RemoteHost::new(&task.host.name).open();

    let dirs = traverse_dir(Path::new("./"), &task.code.ignore, &config_ctx.config_file);

    let location = PathBuf::from(&task.host.base_dir.join(&task.code.location));

    session
        .send_directory(dirs.as_slice(), &location)
        .await
        .unwrap();

    TaskRunner::new(&session)
        .perform_task_set(&location, &task.run)
        .await
        .unwrap();
}
