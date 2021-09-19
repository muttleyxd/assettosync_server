use std::path::Path;
use walkdir::WalkDir;

pub struct FsEntry {
    pub path: String,
    pub is_file: bool,
}

pub fn recursive_ls(dir: &Path) -> Vec<FsEntry> {
    let mut ret = vec![];
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        ret.push(FsEntry {
            path: entry.path().display().to_string(),
            is_file: entry.path().is_file(),
        });
    }
    ret
}
