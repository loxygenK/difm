use std::{
    fmt::{write, Display},
    path::{self, Path, PathBuf},
};

use ignore::{gitignore::GitignoreBuilder, Walk};

use crate::{remote, when};

pub struct FileTransferList {
    local_source_origin: PathBuf,
    remote_dest_origin: PathBuf,
    ignore_statement: String,
    ignore_origin: PathBuf,
}

impl FileTransferList {
    pub fn new(
        local_source_origin: &Path,
        remote_dest_origin: &Path,
        ignore_statement: &str,
        ignore_origin: &Path,
    ) -> Self {
        Self {
            local_source_origin: local_source_origin.to_path_buf(),
            remote_dest_origin: remote_dest_origin.to_path_buf(),
            ignore_statement: ignore_statement.to_string(),
            ignore_origin: ignore_origin.to_path_buf(),
        }
    }

    pub fn local_source_origin(&self) -> &Path {
        &self.local_source_origin
    }

    pub fn remote_dest_origin(&self) -> &Path {
        &self.remote_dest_origin
    }

    pub fn traverse_dir(&self) -> impl Iterator<Item = Entry> + '_ {
        let mut gitignore = GitignoreBuilder::new(&self.local_source_origin);

        self.ignore_statement.lines().for_each(|line| {
            gitignore
                .add_line(Some(self.ignore_origin.to_owned()), line)
                .unwrap();
        });

        let gitignore = gitignore.build().unwrap();

        let walker = Walk::new(&self.local_source_origin);

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

                Entry::new(
                    when! {
                        file_type.is_file() => EntryType::File,
                        file_type.is_dir() => EntryType::Dir,
                        _ => todo!(),
                    },
                    &self.local_source_origin,
                    &self.remote_dest_origin,
                    path.path(),
                )
            })
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub kind: EntryType,
    pub local_source: PathBuf,
    pub remote_dest: PathBuf,
    pub path_name: PathBuf,
    local_origin: PathBuf,
    remote_origin: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryType {
    File,
    Dir,
}

impl Entry {
    pub fn new(
        kind: EntryType,
        local_origin: &Path,
        remote_origin: &Path,
        path_name: &Path,
    ) -> Self {
        Self {
            kind,
            local_source: local_origin.join(path_name),
            remote_dest: remote_origin.join(path_name),
            path_name: path_name.to_path_buf(),
            local_origin: local_origin.to_path_buf(),
            remote_origin: remote_origin.to_path_buf(),
        }
    }

    pub fn is_same(&self, path: &Path) -> bool {
        self.path_name == path
            || self.local_source == path
            || self.remote_dest == path
            || self.local_source == self.local_origin.join(path)
            || self.remote_dest == self.remote_origin.join(path)
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} -> {})",
            self.path_name.display(),
            self.local_source.display(),
            self.remote_dest.display()
        )
    }
}
