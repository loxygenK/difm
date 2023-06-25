use std::path::Path;

use ignore::{Walk, WalkBuilder, gitignore::Gitignore, DirEntry};
use walkdir::WalkDir;

pub fn traverse_dir(base: &Path, ignore_match: Gitignore) -> Vec<DirEntry> {
  let walker = Walk::new(base);

  walker.into_iter()
    .filter_map(|path| path.ok())
    .filter(move |path| {
      let matched = ignore_match.matched(path.path(), path.file_type().unwrap().is_dir());

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