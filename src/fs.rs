use std::path::{Path, PathBuf};

use ignore::{gitignore::GitignoreBuilder, Walk};

use crate::when;

pub struct FileTransferList {
    code_location: PathBuf,
    remote_dest: PathBuf,
    ignore_statement: String,
    ignore_origin: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub kind: EntryType,
    pub local_source: PathBuf,
    pub remote_dest: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryType {
    File,
    Dir,
}

impl FileTransferList {
    pub fn new(
        code_location: &Path,
        remote_dest: &Path,
        ignore_statement: &str,
        ignore_origin: &Path,
    ) -> Self {
        Self {
            code_location: code_location.to_path_buf(),
            remote_dest: remote_dest.to_path_buf(),
            ignore_statement: ignore_statement.to_string(),
            ignore_origin: ignore_origin.to_path_buf(),
        }
    }

    pub fn traverse_dir(&self) -> Vec<Entry> {
        let mut gitignore = GitignoreBuilder::new(&self.code_location);

        self.ignore_statement.lines().for_each(|line| {
            gitignore
                .add_line(Some(self.ignore_origin.to_owned()), line)
                .unwrap();
        });

        let gitignore = gitignore.build().unwrap();

        let walker = Walk::new(&self.code_location);

        walker
            .into_iter()
            .filter_map(|path| path.ok())
            .filter(move |path| {
                let matched = gitignore.matched(path.path(), path.file_type().unwrap().is_dir());

                when! {
                    matched.is_whitelist() => true,
                    matched.is_ignore() => false,
                    matched.is_none() => true,
                    _ => !,
                }
            })
            .map(move |path| {
                // None is possible if the `path` is stdin
                let file_type = path.file_type().unwrap();

                Entry {
                    kind: when! {
                        file_type.is_file() => EntryType::File,
                        file_type.is_dir() => EntryType::Dir,
                        _ => todo!(),
                    },
                    local_source: path.path().to_path_buf(),
                    remote_dest: self.remote_dest.join(path.path()),
                }
            })
            .collect()
    }
}
