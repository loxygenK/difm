use std::{io::BufReader, fs::File, path::Path};

use ignore::gitignore::GitignoreBuilder;
use ssh2_config::ParseRule;

use crate::config::{ConfigContext, Configuration};
use crate::fs::traverse_dir;
use crate::ssh::SSHSession;

pub async fn run_task(config_ctx: &ConfigContext) {
    let Configuration::TaskDefinition(task) = &config_ctx.config; //  else { unreachable!(); };

    let file = File::open("/Users/flisan/.ssh/config").unwrap();
    let mut reader = BufReader::new(file);

    let config = ssh2_config::SshConfig::default()
        .parse(&mut reader, ParseRule::STRICT)
        .unwrap();

    let config = config.query(&task.host.name);
    let session = SSHSession::open(&task.host.name, &config);
    let result = session.run_command("ls -al").await;
    println!("{}", result.stdout);

    let mut gitignore = GitignoreBuilder::new(&task.code.location);

    task.code.ignore
        .lines()
        .for_each(|line| { gitignore.add_line(Some(config_ctx.config_file.clone()), line).unwrap(); });

    session.send_directory(
        traverse_dir(
            Path::new("./"),
            GitignoreBuilder::new(Path::new("./"))
                .build().unwrap()
        ).as_slice(),
        Path::new(&task.host.base_dir.join(&task.code.location))
    ).await.unwrap();
}