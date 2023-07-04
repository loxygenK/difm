use std::{
    collections::{HashMap, HashSet},
    io,
    ops::Deref,
    path::{Path, PathBuf},
};

use sha256::try_digest;

use crate::{
    adapter::{
        fs::{Entry, EntryType, FileTransferList},
        ssh::{exec::ExecChannel, SSHSession},
    },
    progress::ProgressView,
};

pub async fn check_file_change<'a>(
    session: &'a SSHSession,
    transfer_list: &FileTransferList,
) -> Result<Vec<Entry>, ()> {
    ProgressView::with("Checking if the file changed", |progress| async {
        let files: Vec<_> = transfer_list
            .traverse_dir()
            .filter(|entry| entry.kind == EntryType::File)
            .collect();

        let remote = tokio::spawn(calculate_remote_sha256(
            session.shared_clone(),
            files.clone(),
        ));

        let local = calculate_local_sha256(&files).unwrap();
        let remote = remote.await.unwrap().unwrap();

        let diff_path = check_differences(
            &local,
            &remote,
            transfer_list.local_source_origin(),
            transfer_list.remote_dest_origin(),
        );

        Ok(files
            .into_iter()
            .filter(|entry| diff_path.iter().any(|path| entry.is_same(path)))
            .collect())
    })
    .await
}

fn calculate_local_sha256(files: &[Entry]) -> Result<HashMap<PathBuf, String>, io::Error> {
    files
        .iter()
        .filter(|entry| entry.kind == EntryType::File)
        .map(|entry| {
            try_digest(&*entry.local_source).map(|digest| (entry.local_source.clone(), digest))
        })
        .collect()
}

async fn calculate_remote_sha256(
    session: SSHSession,
    files: Vec<Entry>,
) -> Result<HashMap<PathBuf, String>, io::Error> {
    let file_paths: Vec<String> = files
        .iter()
        .filter(|entry| entry.kind == EntryType::File)
        .map(|entry| format!("'{}'", entry.remote_dest.to_str().unwrap()))
        .collect();

    // dbg!(&file_paths);

    let executed =
        ExecChannel::execute(&session, &format!("sha256sum {}", file_paths.join(" "))).await;

    // dbg!(&executed.stdout);
    // dbg!(&executed.stderr);

    // dbg!(ExecChannel::execute(&session, "echo $PATH").await.stdout);

    Ok(executed
        .stdout
        .lines()
        .flat_map(|line| line.trim().split_once(['\t', ' ']))
        .map(|(digest, path)| (PathBuf::from(path.trim()), digest.to_string()))
        .collect())
}

fn check_differences(
    local: &HashMap<PathBuf, String>,
    remote: &HashMap<PathBuf, String>,
    local_base: &Path,
    remote_base: &Path,
) -> Vec<PathBuf> {
    let local_keys: HashSet<_> = local
        .keys()
        .map(|path| path.strip_prefix(local_base).unwrap())
        .collect();
    let remote_keys: HashSet<_> = remote
        .keys()
        .map(|path| path.strip_prefix(remote_base).unwrap())
        .collect();

    let missing_in_local = local_keys.difference(&remote_keys);
    let missing_in_remote = remote_keys.difference(&local_keys);
    let exist_in_both = local_keys.intersection(&remote_keys);

    let content_differs = exist_in_both.filter(|path| {
        local.get(&local_base.join(path)).unwrap() != remote.get(&remote_base.join(path)).unwrap()
    });

    missing_in_local
        .chain(missing_in_remote)
        .chain(content_differs)
        .map(|path| path.to_path_buf())
        .collect()
}
