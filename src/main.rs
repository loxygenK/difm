mod config;
mod fs;
mod remote;
mod services;
mod ssh;
mod task;
mod util;

use services::run_task::run_task;

use crate::config::read_config;

#[tokio::main]
async fn main() {
    let config = read_config(Some("./examples/cross-compile.yml".into()));

    run_task(&config).await
}
