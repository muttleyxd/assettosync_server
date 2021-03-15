use std::{fs::File, path::Path};
use walkdir::WalkDir;

pub fn rar_unpack(archive_path: &Path, destination_path: &Path) -> compress_tools::Result<()> {
    let path_as_string = archive_path.to_str().unwrap().to_string();
    let archive = unrar::Archive::new(path_as_string);
    let result = archive.extract_to(destination_path.to_str().unwrap().to_string());
    if let Err(error) = result {
        return Err(compress_tools::Error::from(error.to_string()));
    }
    let mut open_archive = result.unwrap();
    let process_result = open_archive.process();
    if let Err(error) = process_result {
        return Err(compress_tools::Error::from(error.to_string()));
    }
    Ok(())
}

pub fn compress_tools_unpack(
    archive_path: &Path,
    destination_path: &Path,
) -> compress_tools::Result<()> {
    let mut source = File::open(archive_path)?;
    compress_tools::uncompress_archive(
        &mut source,
        destination_path,
        compress_tools::Ownership::Ignore,
    )
}

pub fn unpack_archive(archive_path: &Path, destination_path: &Path) -> compress_tools::Result<()> {
    // compress_tools doesn't work with some rar archives, so unrar is used
    let extension = archive_path.extension().unwrap();
    if extension == "rar" {
        rar_unpack(archive_path, destination_path)
    } else {
        compress_tools_unpack(archive_path, destination_path)
    }
}

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
