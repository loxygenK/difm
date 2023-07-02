use std::path::Path;

use ignore::{gitignore::GitignoreBuilder, DirEntry, Walk};

pub fn traverse_dir(base: &Path, ignore: &str, ignore_origin_file: &Path) -> Vec<DirEntry> {
    let mut gitignore = GitignoreBuilder::new(base);

    ignore.lines().for_each(|line| {
        gitignore
            .add_line(Some(ignore_origin_file.to_owned()), line)
            .unwrap();
    });

    let gitignore = gitignore.build().unwrap();

    let walker = Walk::new(base);

    walker
        .into_iter()
        .filter_map(|path| path.ok())
        .filter(move |path| {
            let matched = gitignore.matched(path.path(), path.file_type().unwrap().is_dir());

            if matched.is_whitelist() {
                true
            } else if matched.is_ignore() {
                false
            } else if matched.is_none() {
                true
            } else {
                unreachable!();
            }
        })
        .collect()
}
