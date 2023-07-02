mod ssh;
mod services;
mod util;
mod fs;
mod config;
mod task;
mod remote;

use services::run_task::run_task;

use crate::config::read_config;

#[tokio::main]
async fn main() {
    let config = read_config(Some("./examples/cross-compile.yml".into()));

    run_task(&config).await
}
